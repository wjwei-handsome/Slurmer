use color_eyre::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseEvent};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders, Clear, Paragraph},
};
use regex;
use std::time::{Duration, Instant};
use tokio::runtime::Runtime;

use crate::{
    slurm::{
        JobState,
        command::{execute_scancel, get_partitions, get_qos},
        squeue::{SqueueOptions, run_squeue},
    },
    ui::{
        columns::{ColumnsAction, ColumnsPopup, JobColumn, SortColumn, SortOrder},
        filter::{FilterAction, FilterPopup},
        jobscript::JobScript,
        jobslist::JobsList,
        layout::{centered_popup_area, draw_footer, draw_header, draw_main_layout},
        logview::LogView,
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
    /// Filter popup state
    pub filter_popup: FilterPopup,
    /// Is the job detail popup visible?
    /// Columns popup state
    pub columns_popup: ColumnsPopup,
    /// Log view state
    pub log_view: LogView,
    /// Script View state
    pub script_view: JobScript,
    /// Status message to display in the status bar
    pub status_message: String,
    /// Status message display timeout
    pub status_timeout: Option<Instant>,
    /// Auto-refresh interval in seconds
    pub job_refresh_interval: u64,
    /// Available partitions
    pub available_partitions: Vec<String>,
    /// Available QOS options
    pub available_qos: Vec<String>,
    /// Available job states
    pub available_states: Vec<JobState>,
    /// Selected columns for display
    pub selected_columns: Vec<JobColumn>,
    /// Sort columns
    pub sort_columns: Vec<SortColumn>,
    /// Confirm cancel popup state
    cancel_confirm: bool,
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

        // Get available partitions and QOS
        let available_partitions = runtime.block_on(async { get_partitions().await })?;
        let available_qos = runtime.block_on(async { get_qos().await })?;
        let available_states = JobState::get_available_states();

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
            filter_popup: FilterPopup::new(),
            columns_popup: ColumnsPopup::new(selected_columns.clone(), sort_columns.clone()),
            log_view: LogView::new(),
            script_view: JobScript::new(),
            status_message: String::new(),
            status_timeout: None,
            job_refresh_interval: 10, // Default to 10 seconds refresh
            available_partitions,
            available_qos,
            available_states,
            selected_columns,
            sort_columns,
            cancel_confirm: false,
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
        // self.filter_popup.initialize(&self.squeue_options);

        // Update squeue format string based on selected columns
        // self.update_squeue_format();

        // Ensure the column popup has the correct initial state
        // self.columns_popup =
        //     ColumnsPopup::new(self.selected_columns.clone(), self.sort_columns.clone());

        while self.running {
            terminal.draw(|frame| self.render(frame))?;
            self.handle_events()?;
        }

        Ok(())
    }

    /// Refresh the jobs list from Slurm
    fn refresh_jobs(&mut self) -> Result<()> {
        // Update squeue format and sort options
        self.update_squeue_format();

        // Clone options after format has been updated
        let options = self.squeue_options.clone();
        let mut jobs = self
            .runtime
            .block_on(async { run_squeue(&options).await })?;

        let mut filter_stats = Vec::new();
        let initial_count = jobs.len();

        // Apply regex name filter if it exists
        if let Some(name_filter) = &self.squeue_options.name_filter {
            if !name_filter.is_empty() {
                // Try to compile the regex pattern
                match regex::Regex::new(name_filter) {
                    Ok(re) => {
                        // Filter jobs by name using regex
                        let before_count = jobs.len();
                        jobs.retain(|job| re.is_match(&job.name));
                        let after_count = jobs.len();

                        // Track filtering stats
                        if before_count != after_count && before_count > 0 {
                            filter_stats.push(format!(
                                "name: {}/{} ({:.1}%)",
                                after_count,
                                before_count,
                                (after_count as f64 / before_count as f64) * 100.0
                            ));
                        }
                    }
                    Err(e) => {
                        self.set_status_message(format!("Invalid name regex pattern: {}", e), 3);
                    }
                }
            }
        }

        // Apply regex node filter if it exists
        if let Some(node_filter) = &self.squeue_options.node_filter {
            if !node_filter.is_empty() {
                // Try to compile the regex pattern
                match regex::Regex::new(node_filter) {
                    Ok(re) => {
                        // Filter jobs by node name using regex
                        let before_count = jobs.len();
                        jobs.retain(|job| {
                            // If there's a node field, check it against the regex
                            if let Some(node) = &job.node {
                                re.is_match(node)
                            } else {
                                // If no node name available, don't filter this job
                                true
                            }
                        });
                        let after_count = jobs.len();

                        // Track filtering stats
                        if before_count != after_count && before_count > 0 {
                            filter_stats.push(format!(
                                "node: {}/{} ({:.1}%)",
                                after_count,
                                before_count,
                                (after_count as f64 / before_count as f64) * 100.0
                            ));
                        }
                    }
                    Err(e) => {
                        self.set_status_message(format!("Invalid node regex pattern: {}", e), 3);
                    }
                }
            }
        }

        // Show filter statistics if any filters were applied
        if !filter_stats.is_empty() {
            let final_count = jobs.len();
            let total_pct = if initial_count > 0 {
                (final_count as f64 / initial_count as f64) * 100.0
            } else {
                100.0
            };

            self.set_status_message(
                format!(
                    "Filtered: {}/{} total ({:.1}%) [{}]",
                    final_count,
                    initial_count,
                    total_pct,
                    filter_stats.join(", ")
                ),
                5,
            );
        }

        self.jobs_list.update_jobs(jobs);
        self.last_refresh = Instant::now();

        Ok(())
    }

    /// Render the application UI
    pub fn render(&mut self, frame: &mut Frame) {
        let areas = draw_main_layout(frame);

        // Draw header with status information
        self.render_header(frame, areas[0]);

        // Draw jobs list in the main content area with current column settings
        // Make sure to still render the jobs list even when log view is visible
        // so that the jobs list is updated when user navigates with SHIFT+arrow keys
        self.render_joblist(frame, areas[1]);

        // Draw the footer with controls
        self.render_footer(frame, areas[2]);

        // If filter popup is visible, draw it
        if self.filter_popup.visible {
            let popup_area = centered_popup_area(frame.area(), 80, 80);
            self.render_filter_popup(frame, popup_area);
        }

        // If job detail popup is visible, draw it
        if self.script_view.visible {
            let popup_area = centered_popup_area(frame.area(), 80, 60);
            self.render_job_script(frame, popup_area);
        }

        // If columns popup is visible, draw it
        if self.columns_popup.visible {
            let popup_area = centered_popup_area(frame.area(), 80, 80);
            self.render_columns_popup(frame, popup_area);
        }

        // If log view is visible, draw it and check if we need to refresh its content
        if self.log_view.visible {
            let popup_area = centered_popup_area(frame.area(), 80, 80);
            self.render_log_view(frame, popup_area);
        }

        // If cancel confirm popup is visible, draw it
        if self.cancel_confirm {
            let popup_area = centered_popup_area(frame.area(), 50, 30);
            self.render_cancel_confirm(frame, popup_area);
        }
    }

    /// Render the joblist
    fn render_joblist(&mut self, frame: &mut Frame, area: Rect) {
        // Draw the jobs list in the main content area with current column settings
        self.jobs_list
            .render(frame, area, &self.selected_columns, &self.sort_columns);
    }

    /// Render the columns management popup
    fn render_columns_popup(&mut self, frame: &mut Frame, area: Rect) {
        // Render the columns management popup
        self.columns_popup.render(frame, area);
    }

    /// Render the log view
    fn render_log_view(&mut self, frame: &mut Frame, area: Rect) {
        // Render the log view if it's visible
        self.log_view.render(frame, area);
    }

    /// Render the filter popup
    fn render_filter_popup(&mut self, frame: &mut Frame, area: Rect) {
        self.filter_popup.render(
            frame,
            area,
            &self.squeue_options,
            &self.available_states,
            &self.available_partitions,
            &self.available_qos,
        );
    }

    /// Render job detail popup
    fn render_job_script(&self, frame: &mut Frame, area: Rect) {
        self.script_view.render(frame, area);
    }

    /// Render the footer with XXX TODO:replace it
    fn render_footer(&self, frame: &mut Frame, area: Rect) {
        // Prepare footer text
        // let footer_text = "";
        //

        // calculate Pending/Running/Other jobs count percentages
        let pending_count = self
            .jobs_list
            .jobs
            .iter()
            .filter(|j| j.state == JobState::Pending)
            .count();
        let running_count = self
            .jobs_list
            .jobs
            .iter()
            .filter(|j| j.state == JobState::Running)
            .count();
        let other_count = self.jobs_list.jobs.len() - pending_count - running_count;
        let job_stat = (pending_count, running_count, other_count);

        // Draw the footer
        draw_footer(frame, area, job_stat);
    }

    /// Render the header with status information
    fn render_header(&self, frame: &mut Frame, area: Rect) {
        // Prepare the status text
        let status_text = if let Some(timeout) = self.status_timeout {
            if Instant::now() < timeout {
                // Use current status message if it exists and hasn't timed out
                self.status_message.clone()
            } else {
                // Show filter information
                let filter_desc = self.get_filter_description();
                if !filter_desc.is_empty() {
                    format!("Filters: {}", filter_desc)
                } else {
                    "No filters applied".to_string()
                }
            }
        } else {
            // Show filter information if there's no status message
            let filter_desc = self.get_filter_description();
            if !filter_desc.is_empty() {
                format!("Filters: {}", filter_desc)
            } else {
                "No filters applied".to_string()
            }
        };

        // Draw the header with status information
        draw_header(
            frame,
            area,
            &status_text,
            self.last_refresh.elapsed(),
            self.job_refresh_interval,
        );
    }

    fn render_cancel_confirm(&self, frame: &mut Frame, area: Rect) {
        frame.render_widget(Clear, area);
        // Render the cancel confirm popup
        let selected_count = self.jobs_list.get_selected_jobs().len();
        let cancel_text = if selected_count == 0 {
            "No jobs selected for cancellation.".to_string()
        } else {
            format!(
                "Are you sure you want to cancel {} selected job(s)? (y/n)",
                selected_count
            )
        };

        let block = Block::default()
            .title("Confirm Cancel")
            .borders(Borders::ALL)
            .style(Style::default().bg(Color::Black));

        let cancel_popup = Paragraph::new(cancel_text).block(block).centered();

        frame.render_widget(cancel_popup, area);
    }

    /// Handle application events
    fn handle_events(&mut self) -> Result<()> {
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
                if self.filter_popup.visible
                    || self.script_view.visible
                    || self.columns_popup.visible
                    || self.log_view.visible
                    || self.cancel_confirm
                {
                    self.filter_popup.visible = false;
                    self.script_view.visible = false;
                    self.columns_popup.visible = false;
                    self.log_view.hide();
                    self.cancel_confirm = false;
                } else {
                    self.quit();
                }
            }

            // Filter toggle
            (_, KeyCode::Char('f')) if !self.script_view.visible && !self.filter_popup.visible => {
                self.filter_popup.visible = true;
                // Initialize filter popup with current options
                self.filter_popup.initialize(&self.squeue_options);
            }

            // Navigation
            (_, KeyCode::Up)
                if !self.filter_popup.visible
                    && !self.script_view.visible
                    && !self.columns_popup.visible
                    && !self.log_view.visible =>
            {
                self.jobs_list.previous();
            }
            (_, KeyCode::Down)
                if !self.filter_popup.visible
                    && !self.script_view.visible
                    && !self.columns_popup.visible
                    && !self.log_view.visible =>
            {
                self.jobs_list.next();
            }

            // Selection
            (_, KeyCode::Char(' '))
                if !self.filter_popup.visible
                    && !self.script_view.visible
                    && !self.columns_popup.visible =>
            {
                self.jobs_list.toggle_select();
            }
            (_, KeyCode::Char('a'))
                if !self.filter_popup.visible
                    && !self.script_view.visible
                    && !self.columns_popup.visible =>
            {
                // if all jobs are selected, deselect all
                if self.jobs_list.all_selected() {
                    self.jobs_list.clear_selection();
                } else {
                    // Otherwise, select all jobs
                    self.jobs_list.select_all();
                }
            }
            (_, KeyCode::Char('x'))
                if !self.filter_popup.visible
                    && !self.script_view.visible
                    && !self.columns_popup.visible =>
            {
                // scancel the selected jobs and remove them
                self.cancel_confirm = true;
            }
            (_, KeyCode::Char('y'))
                if self.cancel_confirm
                    && !self.filter_popup.visible
                    && !self.script_view.visible
                    && !self.columns_popup.visible =>
            {
                // Confirm cancel selected jobs
                self.cancel_selected_jobs();
                self.cancel_confirm = false;
            }
            (_, KeyCode::Char('n'))
                if self.cancel_confirm
                    && !self.filter_popup.visible
                    && !self.script_view.visible
                    && !self.columns_popup.visible =>
            {
                // Cancel the cancel confirmation
                self.cancel_confirm = false;
            }

            // Column management popup
            (_, KeyCode::Char('c'))
                if !self.filter_popup.visible
                    && !self.script_view.visible
                    && !self.columns_popup.visible
                    && !self.cancel_confirm =>
            {
                self.columns_popup =
                    ColumnsPopup::new(self.selected_columns.clone(), self.sort_columns.clone());
                self.columns_popup.visible = true;
            }

            // Handle filter popup key events
            _ if self.filter_popup.visible => {
                let action = self.filter_popup.handle_key(
                    key,
                    &mut self.squeue_options,
                    &self.available_states,
                    &self.available_partitions,
                    &self.available_qos,
                );

                match action {
                    FilterAction::Close => {
                        self.filter_popup.visible = false;
                    }
                    FilterAction::Apply => {
                        self.filter_popup.visible = false;
                        if let Err(e) = self.apply_filters() {
                            self.set_status_message(format!("Failed to apply filters: {}", e), 3);
                        }
                    }
                    FilterAction::None => {}
                }
            }

            // Job detail view
            (_, KeyCode::Enter)
                if !self.filter_popup.visible
                    && !self.script_view.visible
                    && !self.columns_popup.visible
                    && !self.log_view.visible =>
            {
                if let Some(job) = self.jobs_list.selected_job() {
                    // Show job script in detail view
                    self.script_view.show(job.id.clone());
                }
            }

            _ if self.script_view.visible => {
                // If script view is visible, handle script view specific keys
                self.script_view.handle_key(key);
            }

            // Close job detail view
            (_, KeyCode::Enter) if self.script_view.visible => {
                self.script_view.visible = false;
            }

            // Show log view
            (_, KeyCode::Char('v'))
                if !self.filter_popup.visible
                    && !self.script_view.visible
                    && !self.columns_popup.visible
                    && !self.log_view.visible =>
            {
                if let Some(job) = self.jobs_list.selected_job() {
                    self.log_view.show(job.id.clone());
                }
            }

            // Change job for log view
            (KeyModifiers::SHIFT, KeyCode::Up) if self.log_view.visible => {
                // If Shift is pressed, switch to previous job and show its logs
                if self.jobs_list.previous() {
                    if let Some(job) = self.jobs_list.selected_job() {
                        self.log_view.change_job(job.id.clone());
                    }
                }
            }
            (KeyModifiers::SHIFT, KeyCode::Down) if self.log_view.visible => {
                // If Shift is pressed, switch to next job and show its logs
                if self.jobs_list.next() {
                    if let Some(job) = self.jobs_list.selected_job() {
                        self.log_view.change_job(job.id.clone());
                    }
                }
            }

            // Handle log view keys events
            _ if self.log_view.visible => {
                // If log view is visible, handle log view specific keys
                self.log_view.handle_key(key);
            }

            // Handle columns popup key events
            _ if self.columns_popup.visible => {
                let action = self.columns_popup.handle_key(key);

                match action {
                    ColumnsAction::Close => {
                        self.columns_popup.visible = false;
                    }
                    ColumnsAction::Apply => {
                        self.columns_popup.visible = false;
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
                        self.columns_popup.visible = false;
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
                if !self.filter_popup.visible
                    && !self.script_view.visible
                    && !self.columns_popup.visible =>
            {
                if let Err(e) = self.refresh_jobs() {
                    self.set_status_message(format!("Failed to refresh: {}", e), 3);
                }
            }

            _ => {}
        }
    }

    /// Handle mouse events
    fn handle_mouse_event(&mut self, _mouse: MouseEvent) {
        // TODO: Implement mouse event handling for TUI interactions
        // seems unnecessary?
    }

    /// Handle tick events (called periodically)
    fn handle_tick(&mut self) {
        // Check if it's time to auto-refresh
        if !self.filter_popup.visible
            && !self.script_view.visible
            && !self.columns_popup.visible
            && self.last_refresh.elapsed().as_secs() >= self.job_refresh_interval
        {
            if let Err(e) = self.refresh_jobs() {
                self.set_status_message(format!("Auto-refresh failed: {}", e), 3);
            }
        }

        // Check for log view updates and refresh content
        if self.log_view.visible {
            self.log_view.check_refresh();
        }
    }

    /// Set a temporary status message
    fn set_status_message(&mut self, message: String, duration_secs: u64) {
        self.status_message = message;
        self.status_timeout = Some(Instant::now() + Duration::from_secs(duration_secs));
    }

    /// Set the auto-refresh interval in seconds
    /// TODO: maybe used it in the future
    fn _set_refresh_interval(&mut self, seconds: u64) {
        self.job_refresh_interval = seconds;
        self.set_status_message(format!("Auto-refresh interval set to {}s", seconds), 3);
    }

    /// Apply all filter changes and refresh jobs
    fn apply_filters(&mut self) -> Result<()> {
        self.filter_popup.visible = false;
        self.set_status_message("Applying filters...".to_string(), 3);

        // Ensure we refresh the jobs with the updated filters
        let result = self.refresh_jobs();

        // Display feedback about the filter application
        if result.is_ok() {
            let filter_desc = self.get_filter_description();
            if !filter_desc.is_empty() {
                let loaded_count = self.jobs_list.jobs.len();
                self.set_status_message(
                    format!(
                        "Filters applied: {} ({} jobs shown)",
                        filter_desc, loaded_count
                    ),
                    3,
                );
            } else {
                let loaded_count = self.jobs_list.jobs.len();
                self.set_status_message(
                    format!("Filters cleared ({} jobs shown)", loaded_count),
                    3,
                );
            }
        }

        result
    }

    /// Get a human-readable description of the current filters
    fn get_filter_description(&self) -> String {
        let mut parts = Vec::new();

        // User filter
        if let Some(user) = &self.squeue_options.user {
            parts.push(format!("user={}", user));
        }

        // State filters
        if !self.squeue_options.states.is_empty() {
            let states = self
                .squeue_options
                .states
                .iter()
                .map(|s| s.to_string())
                .collect::<Vec<_>>()
                .join(",");
            parts.push(format!("state={}", states));
        }

        // Partition filters
        if !self.squeue_options.partitions.is_empty() {
            let partitions = self.squeue_options.partitions.join(",");
            parts.push(format!("partition={}", partitions));
        }

        // QoS filters
        if !self.squeue_options.qos.is_empty() {
            let qos = self.squeue_options.qos.join(",");
            parts.push(format!("qos={}", qos));
        }

        // Name filter (regex)
        if let Some(name) = &self.squeue_options.name_filter {
            parts.push(format!("name_regex={}", name));
        }

        // Node filter (regex)
        if let Some(node) = &self.squeue_options.node_filter {
            parts.push(format!("node_regex={}", node));
        }

        parts.join(", ")
    }

    /// Set running to false to quit the application
    fn quit(&mut self) {
        self.running = false;
    }

    /// Update the squeue format string and sort options based on selected columns
    fn update_squeue_format(&mut self) {
        // Ensure we have at least one column selected
        // if self.selected_columns.is_empty() {
        //     // Use default columns if none are selected
        //     self.selected_columns = JobColumn::defaults();
        // }

        // Generate format string for squeue based on column selection
        let format_string = self
            .selected_columns
            .iter()
            .map(|col| col.format_code())
            .collect::<Vec<&str>>()
            .join("|");
        self.squeue_options.format = format_string;

        // Build sort string based on sort columns
        // remove any existing sort columns
        self.squeue_options.sorts.clear();
        if !self.sort_columns.is_empty() {
            // Add sort colsumns to the squeue options
            for sort_col in &self.sort_columns {
                // get the format code for the column, removing any leading '%'
                let sort_code = sort_col.column.format_code().trim_start_matches('%');
                // set the sort order
                let is_ascending = matches!(sort_col.order, SortOrder::Ascending);

                // add to the squeue options
                self.squeue_options
                    .sorts
                    .insert(sort_code.to_string(), is_ascending);
            }

            // Set the first sort column as the primary sort
            if let Some(first_sort) = self.sort_columns.first() {
                let sort_column_index = self
                    .selected_columns
                    .iter()
                    .position(|col| {
                        std::mem::discriminant(col) == std::mem::discriminant(&first_sort.column)
                    })
                    .unwrap_or(0);

                // Set the jobs list sort column and order
                self.jobs_list.sort_column = sort_column_index;
                self.jobs_list.sort_ascending = matches!(first_sort.order, SortOrder::Ascending);
            }
        } else {
            self.squeue_options.sorts.insert("i".to_string(), true);
            self.jobs_list.sort_column = 0;
            self.jobs_list.sort_ascending = true;
        }
    }

    fn cancel_selected_jobs(&mut self) {
        // Get selected job IDs
        let selected_jobs = self.jobs_list.get_selected_jobs();
        let selecteed_count = selected_jobs.len();
        let _ = self
            .runtime
            .block_on(async { execute_scancel(selected_jobs).await });
        // refresh the jobs list after cancellation
        if let Err(e) = self.refresh_jobs() {
            self.set_status_message(format!("Failed to refresh after cancel: {}", e), 3);
        } else {
            self.set_status_message(format!("Cancelled {} job(s)", selecteed_count), 3);
        }
    }
}
