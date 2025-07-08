use color_eyre::Result;
use crossbeam::channel::{Receiver, Sender, unbounded};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    process::Command,
    time::{Duration, Instant},
};

use crate::utils::file_watcher::{FileWatcherError, FileWatcherHandle};

/// Type of log to view
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogTab {
    StdOut,
    StdErr,
}

impl LogTab {
    pub fn toggle(&mut self) {
        *self = match self {
            LogTab::StdOut => LogTab::StdErr,
            LogTab::StdErr => LogTab::StdOut,
        };
    }

    fn as_str(&self) -> &'static str {
        match self {
            LogTab::StdOut => "stdout",
            LogTab::StdErr => "stderr",
        }
    }
}

/// LogView widget for displaying job output logs
pub struct LogView {
    pub visible: bool,
    pub job_id: Option<String>,
    pub current_tab: LogTab,
    pub content: String,
    pub scroll_position: usize,
    pub stdout_path: Option<String>,
    pub stderr_path: Option<String>,
    file_watcher: Option<FileWatcherHandle>,
    file_receiver: Option<Receiver<Result<String, FileWatcherError>>>,
    last_refresh: Option<Instant>,
    refresh_interval: Duration,
}

impl LogView {
    pub fn new() -> Self {
        Self {
            visible: false,
            job_id: None,
            current_tab: LogTab::StdOut,
            content: String::new(),
            scroll_position: 0,
            stdout_path: None,
            stderr_path: None,
            file_watcher: None,
            file_receiver: None,
            last_refresh: None,
            refresh_interval: Duration::from_secs(2),
        }
    }

    /// Show the log view for a specific job
    pub fn show(&mut self, job_id: String) {
        self.visible = true;
        self.change_job(job_id);
    }

    /// Hide the log view
    pub fn hide(&mut self) {
        self.visible = false;
        // Stop watching files when hiding the view
        if let Some(watcher) = &mut self.file_watcher {
            watcher.set_file_path(None);
        }
    }

    /// Change the job being viewed
    pub fn change_job(&mut self, job_id: String) {
        self.job_id = Some(job_id);
        self.stdout_path = None;
        self.stderr_path = None;
        self.content = String::new();
        self.scroll_position = 0;

        // Fetch the log file paths
        self.fetch_log_paths();

        // Setup file watcher if needed
        if self.file_watcher.is_none() {
            let (sender, receiver) = unbounded();
            self.file_watcher = Some(FileWatcherHandle::new(sender, self.refresh_interval));
            self.file_receiver = Some(receiver);
        }

        // Update the watched file based on current tab
        self.update_watched_file();
    }

    /// Toggle between stdout and stderr logs
    pub fn toggle_tab(&mut self) {
        self.current_tab.toggle();
        self.content = String::new();
        self.scroll_position = 0;
        self.update_watched_file();
    }

    /// Update the file being watched based on job_id and current_tab
    fn update_watched_file(&mut self) {
        if let Some(watcher) = &mut self.file_watcher {
            let path = match self.current_tab {
                LogTab::StdOut => self.stdout_path.clone(),
                LogTab::StdErr => self.stderr_path.clone(),
            };

            match path {
                Some(p) if !p.is_empty() => {
                    watcher.set_file_path(Some(PathBuf::from(&p)));
                }
                _ => {
                    // Either no path or empty path
                    watcher.set_file_path(None);
                    self.content = format!(
                        "No {} log file found for job {}",
                        self.current_tab.as_str(),
                        self.job_id.as_deref().unwrap_or("unknown")
                    );
                }
            }
        }
    }

    /// Check for file updates and refresh content
    pub fn check_refresh(&mut self) {
        // Check if we need to refresh
        let should_refresh = match self.last_refresh {
            Some(instant) => instant.elapsed() >= self.refresh_interval,
            None => true,
        };

        if should_refresh && self.file_receiver.is_some() {
            let receiver = self.file_receiver.as_ref().unwrap();

            // Check for new content from the file watcher
            while let Ok(result) = receiver.try_recv() {
                match result {
                    Ok(content) => {
                        if !content.is_empty() {
                            self.content = content;
                        }
                    }
                    Err(e) => {
                        self.content = format!("Error watching file: {}", e);
                    }
                }
            }

            self.last_refresh = Some(Instant::now());
        }
    }

    /// Scroll the log view up
    pub fn scroll_up(&mut self) {
        if self.scroll_position > 0 {
            self.scroll_position -= 1;
        }
    }

    /// Scroll the log view down
    pub fn scroll_down(&mut self) {
        // Need to calculate max scroll based on content and view size
        // This is a simplification - in a real implementation you would
        // calculate this based on content height and view height
        let line_count = self.content.lines().count();
        if self.scroll_position < line_count.saturating_sub(1) {
            self.scroll_position += 1;
        }
    }

    /// Page up in the log view
    pub fn page_up(&mut self) {
        // Move up by a page (10 lines)
        self.scroll_position = self.scroll_position.saturating_sub(10);
    }

    /// Page down in the log view
    pub fn page_down(&mut self) {
        // Move down by a page (10 lines)
        let line_count = self.content.lines().count();
        let new_scroll = self.scroll_position + 10;
        self.scroll_position = if new_scroll < line_count {
            new_scroll
        } else {
            line_count.saturating_sub(1)
        };
    }

    /// Render the log view
    pub fn render(&self, frame: &mut Frame, area: Rect) {
        if !self.visible {
            return;
        }

        // Create a centered popup area for the log view
        let log_area = self.create_popup_area(area);

        let title = match (&self.job_id, self.current_tab) {
            (Some(id), LogTab::StdOut) => format!("Job {} - stdout", id),
            (Some(id), LogTab::StdErr) => format!("Job {} - stderr", id),
            (None, _) => "Log View".to_string(),
        };

        let help_text = " [↑/↓] Scroll | [o] Toggle stdout/stderr | [q] Close ";

        let log_text = if self.content.is_empty() {
            match self.current_tab {
                LogTab::StdOut => format!(
                    "No stdout log available for job {}",
                    self.job_id.as_deref().unwrap_or("unknown")
                ),
                LogTab::StdErr => format!(
                    "No stderr log available for job {}",
                    self.job_id.as_deref().unwrap_or("unknown")
                ),
            }
        } else {
            self.content.clone()
        };

        let log_paragraph = Paragraph::new(log_text)
            .style(Style::default().fg(Color::White))
            .block(
                Block::default()
                    .title(format!("{}{}", title, help_text))
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Cyan)),
            )
            .wrap(Wrap { trim: false })
            .scroll((self.scroll_position as u16, 0));

        frame.render_widget(log_paragraph, log_area);
    }

    // Create a popup area for the log view
    fn create_popup_area(&self, area: Rect) -> Rect {
        // Use 80% of the screen width and height
        let width_percentage = 80;
        let height_percentage = 80;

        let popup_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                [
                    Constraint::Percentage((100 - height_percentage) / 2),
                    Constraint::Percentage(height_percentage),
                    Constraint::Percentage((100 - height_percentage) / 2),
                ]
                .as_ref(),
            )
            .split(area);

        Layout::default()
            .direction(Direction::Horizontal)
            .constraints(
                [
                    Constraint::Percentage((100 - width_percentage) / 2),
                    Constraint::Percentage(width_percentage),
                    Constraint::Percentage((100 - width_percentage) / 2),
                ]
                .as_ref(),
            )
            .split(popup_layout[1])[1]
    }

    /// Fetch the stdout and stderr paths for the current job
    fn fetch_log_paths(&mut self) {
        if let Some(job_id) = &self.job_id {
            let output = Command::new("scontrol")
                .args(["show", "job", job_id, "-o"])
                .output();

            if let Ok(output) = output {
                if output.status.success() {
                    let output_str = String::from_utf8_lossy(&output.stdout);
                    let key_value_pairs = parse_scontrol_output(&output_str);

                    self.stdout_path = key_value_pairs.get("StdOut").map(|s| s.to_string());
                    self.stderr_path = key_value_pairs.get("StdErr").map(|s| s.to_string());
                }
            }
        }
    }
}

fn parse_scontrol_output(output: &str) -> HashMap<String, String> {
    let mut result = HashMap::new();

    for part in output.split_whitespace() {
        if let Some(index) = part.find('=') {
            let key = &part[0..index];
            let value = &part[(index + 1)..];
            result.insert(key.to_string(), value.to_string());
        }
    }

    result
}
