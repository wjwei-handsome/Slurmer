use color_eyre::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseEvent};
use ratatui::{Frame, layout::Rect};
use std::{error::Error, time::Duration};
use tokio::runtime::Runtime;

use crate::{
    slurm::{
        Job,
        squeue::{self, SqueueOptions, run_squeue},
    },
    ui::{
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
    /// Is the filter popup visible?
    pub show_filter_popup: bool,
    /// Is the job detail popup visible?
    pub show_job_detail: bool,
    /// Application message to display to the user
    pub message: Option<String>,
    /// Message display timeout
    pub message_timeout: Option<Duration>,
    /// Active tab index
    pub active_tab: usize,
    /// Available partitions
    pub available_partitions: Vec<String>,
    /// Available QOS options
    pub available_qos: Vec<String>,
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

        // Get available partitions and QOS (placeholder values for now)
        let available_partitions = squeue::available_partitions()?;
        let available_qos = squeue::available_qos()?;

        Ok(Self {
            running: true,
            event_handler: EventHandler::new(EventConfig::default()),
            jobs_list: JobsList::new(),
            squeue_options,
            runtime,
            show_filter_popup: false,
            show_job_detail: false,
            message: None,
            message_timeout: None,
            active_tab: 0,
            available_partitions,
            available_qos,
        })
    }

    /// Run the application's main loop
    pub fn run<B: ratatui::backend::Backend>(
        &mut self,
        terminal: &mut ratatui::Terminal<B>,
    ) -> Result<()> {
        // Initial job loading
        self.refresh_jobs()?;

        while self.running {
            terminal.draw(|frame| self.render(frame))?;
            self.handle_events()?;
        }

        Ok(())
    }

    /// Refresh the jobs list from Slurm
    pub fn refresh_jobs(&mut self) -> Result<()> {
        let options = self.squeue_options.clone();

        let jobs = self
            .runtime
            .block_on(async { run_squeue(&options).await })?;

        self.jobs_list.update_jobs(jobs);
        Ok(())
    }

    /// Render the application UI
    pub fn render(&mut self, frame: &mut Frame) {
        let areas = draw_main_layout(frame);

        // Header area is areas[0], already drawn in layout

        // Draw jobs list in the main content area
        self.jobs_list.render(frame, areas[1]);

        // If filter popup is visible, draw it
        if self.show_filter_popup {
            let popup_area = centered_popup_area(frame.size(), 80, 60);
            self.render_filter_popup(frame, popup_area);
        }

        // If job detail popup is visible, draw it
        if self.show_job_detail {
            let popup_area = centered_popup_area(frame.size(), 80, 60);
            self.render_job_detail(frame, popup_area);
        }

        // Display message if present
        if let Some(message) = &self.message {
            let popup_area = centered_popup_area(frame.size(), 50, 20);
            self.render_message(frame, popup_area, message);
        }
    }

    /// Render the filter popup
    fn render_filter_popup(&self, frame: &mut Frame, area: Rect) {
        // TODO: Implement filter popup rendering
        // This will include UI elements for:
        // - User filter
        // - State filter checkboxes
        // - Partition selection
        // - QOS selection
        // - Job name regex filter
    }

    /// Render job detail popup
    fn render_job_detail(&self, frame: &mut Frame, area: Rect) {
        // TODO: Implement job detail popup rendering
        // This will show detailed information about the selected job
    }

    /// Render a message popup
    fn render_message(&self, frame: &mut Frame, area: Rect, message: &str) {
        // TODO: Implement message popup rendering
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
                self.quit();
            }

            // Filter toggle
            (_, KeyCode::Char('f')) if !self.show_job_detail => {
                self.show_filter_popup = !self.show_filter_popup;
            }

            // Navigation
            (_, KeyCode::Up) if !self.show_filter_popup && !self.show_job_detail => {
                self.jobs_list.previous();
            }
            (_, KeyCode::Down) if !self.show_filter_popup && !self.show_job_detail => {
                self.jobs_list.next();
            }

            // Selection
            (_, KeyCode::Char(' ')) if !self.show_filter_popup && !self.show_job_detail => {
                self.jobs_list.toggle_select();
            }

            // Job detail view
            (_, KeyCode::Enter) if !self.show_filter_popup && !self.show_job_detail => {
                if self.jobs_list.selected_job().is_some() {
                    self.show_job_detail = true;
                }
            }

            // Refresh jobs
            (_, KeyCode::Char('r')) => {
                let _ = self.refresh_jobs();
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
        // TODO: Update any time-based components

        // Check if we should clear the message
        if let Some(timeout) = self.message_timeout {
            if timeout <= Duration::from_secs(0) {
                self.message = None;
                self.message_timeout = None;
            } else {
                self.message_timeout = Some(timeout - Duration::from_millis(250));
            }
        }
    }

    /// Set a temporary message to display to the user
    pub fn set_message(&mut self, message: String, duration_secs: u64) {
        self.message = Some(message);
        self.message_timeout = Some(Duration::from_secs(duration_secs));
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
        self.refresh_jobs()
    }

    /// Set running to false to quit the application
    pub fn quit(&mut self) {
        self.running = false;
    }
}
