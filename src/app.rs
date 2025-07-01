use color_eyre::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseEvent};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};
use std::{
    error::Error,
    time::{Duration, Instant},
};
use tokio::runtime::Runtime;

use crate::{
    slurm::{
        Job,
        squeue::{self, SqueueOptions, run_squeue},
    },
    ui::{
        columns::{ColumnsAction, ColumnsPopup, JobColumn, SortColumn, SortOrder},
        filter::{FilterAction, FilterPopup},
        jobslist::JobsList,
        layout::{centered_popup_area, draw_main_layout},
    },
    utils::{
        event::{Event as AppEvent, EventConfig, EventHandler},
        get_username,
    },
};

/// Application state and logic
pub struct App {
    /// Is the application running?
    pub running: bool,
    /// Event handler for user input
    pub event_handler: EventHandler,
    /// Jobs list widget
    pub jobs_list: JobsList,
    /// Current squeue options
    pub squeue_options: SqueueOptions,
    /// Tokio runtime for async operations
    pub runtime: Runtime,
    /// Last time jobs were refreshed
    pub last_refresh: Instant,
    /// Is the filter popup visible?
    pub show_filter_popup: bool,
    /// Filter popup state
    pub filter_popup: FilterPopup,
    /// Is the job detail popup visible?
    pub show_job_detail: bool,
    /// Is the columns management popup visible?
    pub show_columns_popup: bool,
    /// Columns popup state
    pub columns_popup: ColumnsPopup,
    /// Status message to display in the status bar
    pub status_message: String,
    /// Status message display timeout
    pub status_timeout: Option<Instant>,
    /// Auto-refresh interval in seconds
    pub refresh_interval: u64,
    /// Active tab index
    pub active_tab: usize,
    /// Available partitions
    pub available_partitions: Vec<String>,
    /// Available QOS options
    pub available_qos: Vec<String>,
    /// Selected columns for display
    pub selected_columns: Vec<JobColumn>,
    /// Sort columns
    pub sort_columns: Vec<SortColumn>,
}

impl App {
    /// Create a new application instance
    pub fn new() -> Result<Self> {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("Failed to create Tokio runtime");

        // Default username for squeue
        let username = get_username();
        let squeue_options = SqueueOptions {
            user: Some(username),
            ..Default::default()
        };

        // Get available partitions and QOS (TODO:placeholder values for now)
        let available_partitions = squeue::available_partitions()?;
        let available_qos = squeue::available_qos()?;

        // Default columns and sort options
        let selected_columns = JobColumn::defaults();
        let sort_columns = vec![SortColumn {
            column: JobColumn::Id,
            order: SortOrder::Ascending,
        }];

        Ok(Self {
            running: true,
            event_handler: EventHandler::new(EventConfig::default()),
            jobs_list: JobsList::new(),
            squeue_options,
            runtime,
            last_refresh: Instant::now(),
            show_filter_popup: false,
            filter_popup: FilterPopup::new(),
            show_job_detail: false,
            show_columns_popup: false,
            columns_popup: ColumnsPopup::new(selected_columns.clone(), sort_columns.clone()),
            status_message: String::new(),
            status_timeout: None,
            refresh_interval: 10, // Default to 10 seconds refresh
            active_tab: 0,
            available_partitions,
            available_qos,
            selected_columns,
            sort_columns,
        })
    }

    /// Run the application's main loop
    pub fn run<B: ratatui::backend::Backend>(
        &mut self,
        terminal: &mut ratatui::Terminal<B>,
    ) -> Result<()> {
        // Initial job loading
        self.refresh_jobs()?;

        // Initialize filter popup with current options
        self.filter_popup.initialize(&self.squeue_options);

        // Update squeue format string based on selected columns
        self.update_squeue_format();

        // Ensure the column popup has the correct initial state
        self.columns_popup =
            ColumnsPopup::new(self.selected_columns.clone(), self.sort_columns.clone());

        while self.running {
            terminal.draw(|frame| self.render(frame))?;
            self.handle_events()?;
        }

        Ok(())
    }

    /// Refresh the jobs list from Slurm
    pub fn refresh_jobs(&mut self) -> Result<()> {
        // Update squeue format and sort options
        self.update_squeue_format();

        // Clone options after format has been updated
        let options = self.squeue_options.clone();
        let jobs = self
            .runtime
            .block_on(async { run_squeue(&options).await })?;

        self.jobs_list.update_jobs(jobs);
        self.set_status_message(format!("Loaded {} jobs", self.jobs_list.jobs.len()), 3);
        self.last_refresh = Instant::now();
        Ok(())
    }

    /// Render the application UI
    pub fn render(&mut self, frame: &mut Frame) {
        let areas = draw_main_layout(frame);

        // Draw header with status information
        self.render_header(frame, areas[0]);

        // Draw jobs list in the main content area with current column settings
        self.jobs_list
            .render(frame, areas[1], &self.selected_columns, &self.sort_columns);

        // Draw the footer with controls
        crate::ui::layout::draw_footer(frame, areas[2], "");

        // If filter popup is visible, draw it
        if self.show_filter_popup {
            let popup_area = centered_popup_area(frame.area(), 80, 80);
            self.render_filter_popup(frame, popup_area);
        }

        // If job detail popup is visible, draw it
        if self.show_job_detail {
            let popup_area = centered_popup_area(frame.area(), 80, 60);
            self.render_job_detail(frame, popup_area);
        }

        // If columns popup is visible, draw it
        if self.show_columns_popup {
            let popup_area = centered_popup_area(frame.area(), 80, 80);
            self.columns_popup.render(frame, popup_area);
        }
    }

    /// Render the filter popup
    fn render_filter_popup(&mut self, frame: &mut Frame, area: Rect) {
        // All possible job states for the filter
        let all_states = [
            crate::slurm::JobState::Pending,
            crate::slurm::JobState::Running,
            crate::slurm::JobState::Completed,
            crate::slurm::JobState::Failed,
            crate::slurm::JobState::Cancelled,
            crate::slurm::JobState::Timeout,
            crate::slurm::JobState::NodeFail,
            crate::slurm::JobState::Preempted,
            crate::slurm::JobState::Boot,
        ];

        self.filter_popup.render(
            frame,
            area,
            &self.squeue_options,
            &all_states,
            &self.available_partitions,
            &self.available_qos,
        );
    }

    /// Render job detail popup
    fn render_job_detail(&self, frame: &mut Frame, area: Rect) {
        if let Some(job) = self.jobs_list.selected_job() {
            let detail_text = format!(
                "Job ID: {}\n\
                 Name: {}\n\
                 User: {}\n\
                 State: {}\n\
                 Partition: {}\n\
                 QoS: {}\n\
                 Nodes: {}\n\
                 CPUs: {}\n\
                 Time: {}",
                job.id,
                job.name,
                job.user,
                job.state,
                job.partition,
                job.qos,
                job.nodes,
                job.cpus,
                job.time
            );

            let job_detail = Paragraph::new(detail_text)
                .block(
                    Block::default()
                        .title(format!("Job Details: {}", job.id))
                        .borders(Borders::ALL),
                )
                .style(Style::default());

            frame.render_widget(job_detail, area);
        } else {
            let job_detail = Paragraph::new("No job selected")
                .block(Block::default().title("Job Details").borders(Borders::ALL))
                .style(Style::default());

            frame.render_widget(job_detail, area);
        }
    }

    /// Render the header with status information
    fn render_header(&self, frame: &mut Frame, area: Rect) {
        // Prepare the status text
        let mut status_text = String::new();

        // Add current status message if exists and hasn't timed out
        let show_status = if let Some(timeout) = self.status_timeout {
            Instant::now() < timeout
        } else {
            false
        };

        if show_status {
            status_text = self.status_message.clone();
        } else {
            // Show filter information
            let filter_desc = self.get_filter_description();
            if !filter_desc.is_empty() {
                status_text = format!("Filters: {}", filter_desc);
            } else {
                status_text = "No filters applied".to_string();
            }
        }

        // Draw the header with status information
        crate::ui::layout::draw_header(
            frame,
            area,
            &status_text,
            self.last_refresh.elapsed(),
            self.refresh_interval,
        );
    }

    /// Handle application events
    pub fn handle_events(&mut self) -> Result<()> {
        match self.event_handler.rx.recv()? {
            AppEvent::Key(key) if key.kind == KeyEventKind::Press => self.handle_key_event(key),
            AppEvent::Mouse(mouse) => self.handle_mouse_event(mouse),
            AppEvent::Resize(_, _) => {}
            AppEvent::Tick => self.handle_tick(),
            _ => {}
        }

        Ok(())
    }

    /// Handle key events
    fn handle_key_event(&mut self, key: KeyEvent) {
        match (key.modifiers, key.code) {
            // Quit application
            (_, KeyCode::Char('q'))
            | (_, KeyCode::Esc)
            | (KeyModifiers::CONTROL, KeyCode::Char('c')) => {
                if self.show_filter_popup || self.show_job_detail || self.show_columns_popup {
                    self.show_filter_popup = false;
                    self.show_job_detail = false;
                    self.show_columns_popup = false;
                } else {
                    self.quit();
                }
            }

            // Filter toggle
            (_, KeyCode::Char('f')) if !self.show_job_detail && !self.show_filter_popup => {
                self.show_filter_popup = true;
                // Initialize filter popup with current options
                self.filter_popup.initialize(&self.squeue_options);
            }

            // Navigation
            (_, KeyCode::Up)
                if !self.show_filter_popup && !self.show_job_detail && !self.show_columns_popup =>
            {
                self.jobs_list.previous();
            }
            (_, KeyCode::Down)
                if !self.show_filter_popup && !self.show_job_detail && !self.show_columns_popup =>
            {
                self.jobs_list.next();
            }

            // Selection
            (_, KeyCode::Char(' '))
                if !self.show_filter_popup && !self.show_job_detail && !self.show_columns_popup =>
            {
                self.jobs_list.toggle_select();
            }

            // Column management popup
            (_, KeyCode::Char('c'))
                if !self.show_filter_popup && !self.show_job_detail && !self.show_columns_popup =>
            {
                self.show_columns_popup = true;
                self.columns_popup =
                    ColumnsPopup::new(self.selected_columns.clone(), self.sort_columns.clone());
            }

            // Handle filter popup key events
            _ if self.show_filter_popup => {
                let action = self.filter_popup.handle_key(
                    key,
                    &mut self.squeue_options,
                    &[
                        crate::slurm::JobState::Pending,
                        crate::slurm::JobState::Running,
                        crate::slurm::JobState::Completed,
                        crate::slurm::JobState::Failed,
                        crate::slurm::JobState::Cancelled,
                        crate::slurm::JobState::Timeout,
                        crate::slurm::JobState::NodeFail,
                        crate::slurm::JobState::Preempted,
                        crate::slurm::JobState::Boot,
                    ],
                    &self.available_partitions,
                    &self.available_qos,
                );

                match action {
                    FilterAction::Close => {
                        self.show_filter_popup = false;
                    }
                    FilterAction::Apply => {
                        self.show_filter_popup = false;
                        if let Err(e) = self.apply_filters() {
                            self.set_status_message(format!("Failed to apply filters: {}", e), 3);
                        }
                    }
                    FilterAction::None => {}
                }
            }

            // Job detail view
            (_, KeyCode::Enter)
                if !self.show_filter_popup && !self.show_job_detail && !self.show_columns_popup =>
            {
                if self.jobs_list.selected_job().is_some() {
                    self.show_job_detail = true;
                }
            }

            // Close job detail view
            (_, KeyCode::Enter | KeyCode::Esc) if self.show_job_detail => {
                self.show_job_detail = false;
            }

            // Handle columns popup key events
            _ if self.show_columns_popup => {
                let action = self.columns_popup.handle_key(key);

                match action {
                    ColumnsAction::Close => {
                        self.show_columns_popup = false;
                    }
                    ColumnsAction::Apply => {
                        self.show_columns_popup = false;
                        self.selected_columns = self.columns_popup.selected_columns.clone();
                        self.sort_columns = self.columns_popup.sort_columns.clone();

                        // Update the format and refresh
                        if let Err(e) = self.refresh_jobs() {
                            self.set_status_message(format!("Failed to refresh: {}", e), 3);
                        } else {
                            self.set_status_message("Column settings applied".to_string(), 3);
                        }
                    }
                    ColumnsAction::SaveAndApply => {
                        self.show_columns_popup = false;
                        self.selected_columns = self.columns_popup.selected_columns.clone();
                        self.sort_columns = self.columns_popup.sort_columns.clone();

                        // TODO: Save settings to config file
                        self.set_status_message("Column settings saved and applied".to_string(), 3);

                        // Update the format and refresh
                        if let Err(e) = self.refresh_jobs() {
                            self.set_status_message(format!("Failed to refresh: {}", e), 3);
                        }
                    }
                    ColumnsAction::None => {}
                }
            }

            // Refresh jobs
            (_, KeyCode::Char('r'))
                if !self.show_filter_popup && !self.show_job_detail && !self.show_columns_popup =>
            {
                if let Err(e) = self.refresh_jobs() {
                    self.set_status_message(format!("Failed to refresh: {}", e), 3);
                }
            }

            _ => {}
        }
    }

    /// Handle mouse events
    fn handle_mouse_event(&mut self, mouse: MouseEvent) {
        // TODO: Implement mouse event handling for TUI interactions
    }

    /// Handle tick events (called periodically)
    fn handle_tick(&mut self) {
        // Check if it's time to auto-refresh
        if !self.show_filter_popup
            && !self.show_job_detail
            && !self.show_columns_popup
            && self.last_refresh.elapsed().as_secs() >= self.refresh_interval
        {
            if let Err(e) = self.refresh_jobs() {
                self.set_status_message(format!("Auto-refresh failed: {}", e), 3);
            }
        }
    }

    /// Set a temporary status message
    pub fn set_status_message(&mut self, message: String, duration_secs: u64) {
        self.status_message = message;
        self.status_timeout = Some(Instant::now() + Duration::from_secs(duration_secs));
    }

    /// Set the auto-refresh interval in seconds
    pub fn set_refresh_interval(&mut self, seconds: u64) {
        self.refresh_interval = seconds;
        self.set_status_message(format!("Auto-refresh interval set to {}s", seconds), 3);
    }

    /// Toggle a job state filter
    pub fn toggle_state_filter(&mut self, state: crate::slurm::JobState) {
        let state_pos = self.squeue_options.states.iter().position(|s| *s == state);

        if let Some(pos) = state_pos {
            self.squeue_options.states.remove(pos);
        } else {
            self.squeue_options.states.push(state);
        }
    }

    /// Toggle a partition filter
    pub fn toggle_partition_filter(&mut self, partition: String) {
        let partition_pos = self
            .squeue_options
            .partitions
            .iter()
            .position(|p| *p == partition);

        if let Some(pos) = partition_pos {
            self.squeue_options.partitions.remove(pos);
        } else {
            self.squeue_options.partitions.push(partition);
        }
    }

    /// Toggle a QOS filter
    pub fn toggle_qos_filter(&mut self, qos: String) {
        let qos_pos = self.squeue_options.qos.iter().position(|q| *q == qos);

        if let Some(pos) = qos_pos {
            self.squeue_options.qos.remove(pos);
        } else {
            self.squeue_options.qos.push(qos);
        }
    }

    /// Set the job name filter
    pub fn set_name_filter(&mut self, name: Option<String>) {
        self.squeue_options.name_filter = name;
    }

    /// Apply all filter changes and refresh jobs
    pub fn apply_filters(&mut self) -> Result<()> {
        self.show_filter_popup = false;
        self.set_status_message("Applying filters...".to_string(), 3);

        // Ensure we refresh the jobs with the updated filters
        let result = self.refresh_jobs();

        // Display feedback about the filter application
        if result.is_ok() {
            let filter_desc = self.get_filter_description();
            if !filter_desc.is_empty() {
                self.set_status_message(format!("Filters applied: {}", filter_desc), 3);
            } else {
                self.set_status_message("All filters cleared".to_string(), 3);
            }
        }

        result
    }

    /// Get a human-readable description of current filters
    fn get_filter_description(&self) -> String {
        let mut parts = Vec::new();

        // User filter
        if let Some(user) = &self.squeue_options.user {
            parts.push(format!("user={}", user));
        }

        // States filter
        if !self.squeue_options.states.is_empty() {
            let states = self
                .squeue_options
                .states
                .iter()
                .map(|s| format!("{}", s))
                .collect::<Vec<_>>()
                .join(",");
            parts.push(format!("states={}", states));
        }

        // Partitions filter
        if !self.squeue_options.partitions.is_empty() {
            let partitions = self.squeue_options.partitions.join(",");
            parts.push(format!("partitions={}", partitions));
        }

        // QOS filter
        if !self.squeue_options.qos.is_empty() {
            let qos = self.squeue_options.qos.join(",");
            parts.push(format!("qos={}", qos));
        }

        // Name filter
        if let Some(name) = &self.squeue_options.name_filter {
            parts.push(format!("name={}", name));
        }

        parts.join(", ")
    }

    /// Set running to false to quit the application
    pub fn quit(&mut self) {
        self.running = false;
    }

    /// Update the squeue format string and sort options based on selected columns
    fn update_squeue_format(&mut self) {
        // Ensure we have at least one column selected
        if self.selected_columns.is_empty() {
            // Use default columns if none are selected
            self.selected_columns = JobColumn::defaults();
        }

        // Generate format string for squeue based on column selection
        let format_string = self
            .selected_columns
            .iter()
            .map(|col| col.format_code())
            .collect::<Vec<&str>>()
            .join("|");
        self.squeue_options.format = format_string;

        // Build sort string based on sort columns
        if !self.sort_columns.is_empty() {
            let first_sort = &self.sort_columns[0];
            let sort_code = first_sort.column.format_code();
            let sort_direction = if matches!(first_sort.order, SortOrder::Descending) {
                "-"
            } else {
                ""
            };
            // actually, sort_code is just a single character
            // remove % from the format code
            let sort_code = sort_code.trim_start_matches('%');
            self.squeue_options.sort_by = Some(format!("{}{}", sort_direction, sort_code));
            self.squeue_options.sort_desc = matches!(first_sort.order, SortOrder::Descending);

            // Update jobs list sorting based on sort columns
            let sort_column_index = self
                .selected_columns
                .iter()
                .position(|col| {
                    std::mem::discriminant(col) == std::mem::discriminant(&first_sort.column)
                })
                .unwrap_or(0);

            // Set the jobs list sort column and direction
            self.jobs_list.sort_column = sort_column_index;
            self.jobs_list.sort_ascending = matches!(first_sort.order, SortOrder::Ascending);
        } else {
            // Default to sort by job ID if no sort columns
            self.squeue_options.sort_by = Some("i".to_string());
            self.squeue_options.sort_desc = false;
            self.jobs_list.sort_column = 0;
            self.jobs_list.sort_ascending = true;
        }

        // Make sure the jobs are sorted according to our criteria
        // self.jobs_list.sort_jobs();
    }
}
