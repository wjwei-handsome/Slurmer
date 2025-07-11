use color_eyre::Result;
use crossbeam::channel::{Receiver, Sender, unbounded};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
};
use std::{
    collections::HashMap,
    iter::once,
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
    /// Indicates the status of the current log file
    file_status: LogFileStatus,
}

/// Status of the log file being watched
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LogFileStatus {
    /// No file found or not set
    NotFound,
    /// File exists but waiting for content
    Waiting,
    /// File content has been loaded
    Loaded,
    /// Error occurred when accessing the file
    Error,
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
            file_status: LogFileStatus::NotFound,
        }
    }

    /// Show the log view for a specific job
    pub fn show(&mut self, job_id: String) {
        self.change_job(job_id);
        self.visible = true;
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
        self.file_status = LogFileStatus::NotFound;

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
        self.check_refresh();
    }

    /// Toggle between stdout and stderr logs
    pub fn toggle_tab(&mut self) {
        self.current_tab.toggle();
        self.content = String::new();
        self.scroll_position = 0;
        self.file_status = LogFileStatus::NotFound;
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
                    // File path exists, set status to waiting for content
                    watcher.set_file_path(Some(PathBuf::from(&p)));
                    self.file_status = LogFileStatus::Waiting;
                }
                _ => {
                    // Either no path or empty path
                    watcher.set_file_path(None);
                    self.file_status = LogFileStatus::NotFound;
                    self.content = String::new();
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
                            self.file_status = LogFileStatus::Loaded;
                        } else if self.file_status == LogFileStatus::Waiting {
                            // Got empty content but file exists, keep waiting
                            self.file_status = LogFileStatus::Waiting;
                        }
                    }
                    Err(e) => {
                        self.content = format!("Error watching file: {}", e);
                        self.file_status = LogFileStatus::Error;
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

        let log_area = area;
        frame.render_widget(Clear, log_area);
        let title = match (&self.job_id, self.current_tab) {
            (Some(id), LogTab::StdOut) => format!("Job {} - stdout", id),
            (Some(id), LogTab::StdErr) => format!("Job {} - stderr", id),
            (None, _) => "Log View".to_string(),
        };

        let help_text = " [↑/↓] Scroll | [o] Toggle stdout/stderr | [q] Close ";

        let log_text = match (self.file_status, self.content.is_empty()) {
            (LogFileStatus::NotFound, _) => match self.current_tab {
                LogTab::StdOut => format!(
                    "No stdout log file found for job {}",
                    self.job_id.as_deref().unwrap_or("unknown")
                ),
                LogTab::StdErr => format!(
                    "No stderr log file found for job {}",
                    self.job_id.as_deref().unwrap_or("unknown")
                ),
            },
            (LogFileStatus::Waiting, true) => format!(
                "Loading {} log content for job {}...",
                self.current_tab.as_str(),
                self.job_id.as_deref().unwrap_or("unknown")
            ),
            (LogFileStatus::Error, _) => self.content.clone(),
            _ => self.content.clone(),
        };

        let fit_text = Self::fit_text(
            &log_text,
            log_area.height as usize,
            log_area.width as usize,
            self.scroll_position,
            false,
        );
        eprintln!("fit_text: {}", log_text);

        let log_paragraph = Paragraph::new(fit_text)
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

    fn fit_text(s: &str, lines: usize, cols: usize, offset: usize, _wrap: bool) -> Text {
        // Process text by handling carriage returns
        let processed_lines: Vec<String> = s
            .lines()
            .map(|line| {
                // For each line, if it contains carriage returns, keep only the content after the last one
                if line.contains('\r') {
                    let parts: Vec<&str> = line.split('\r').collect();
                    parts.last().unwrap_or(&"").to_string()
                } else {
                    line.to_string()
                }
            })
            .collect();

        // Join the processed lines back together
        let processed_text = processed_lines.join("\n");

        // Process the clean text (without intermediate carriage returns)
        let lines_iter = processed_text.lines();

        // Create the iterator for processing text lines
        let line_spans = lines_iter
            .rev()
            .skip(offset)
            .flat_map(|l| {
                let chunks = Self::chunked_string(l, cols, cols.saturating_sub(2));
                chunks
                    .into_iter()
                    .enumerate()
                    .map(|(i, chunk)| {
                        if i == 0 {
                            Line::raw(chunk.to_string())
                        } else {
                            Line::default().spans(vec![
                                Span::styled("↪ ", Style::default().add_modifier(Modifier::DIM)),
                                Span::raw(chunk.to_string()),
                            ])
                        }
                    })
                    .rev()
            })
            .take(lines);

        // Collect and build the final Text widget
        let collected_lines: Vec<Line> = line_spans.collect();
        Text::from(collected_lines.into_iter().rev().collect::<Vec<Line>>())
    }

    fn chunked_string(s: &str, first_chunk_size: usize, chunk_size: usize) -> Vec<&str> {
        let stepped_indices = s
            .char_indices()
            .map(|(i, _)| i)
            .enumerate()
            .filter(|&(i, _)| {
                if i > (first_chunk_size) {
                    chunk_size > 0 && (i - first_chunk_size) % chunk_size == 0
                } else {
                    i == 0 || i == first_chunk_size
                }
            })
            .map(|(_, e)| e)
            .collect::<Vec<_>>();
        let windows = stepped_indices.windows(2).collect::<Vec<_>>();

        let iter = windows.iter().map(|w| &s[w[0]..w[1]]);
        let last_index = *stepped_indices.last().unwrap_or(&0);
        iter.chain(once(&s[last_index..])).collect()
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

                    // Check if we have valid paths for the current tab
                    let has_path = match self.current_tab {
                        LogTab::StdOut => {
                            self.stdout_path.is_some()
                                && !self.stdout_path.as_ref().unwrap().is_empty()
                        }
                        LogTab::StdErr => {
                            self.stderr_path.is_some()
                                && !self.stderr_path.as_ref().unwrap().is_empty()
                        }
                    };

                    if has_path {
                        self.file_status = LogFileStatus::Waiting;
                    } else {
                        self.file_status = LogFileStatus::NotFound;
                    }
                } else {
                    self.file_status = LogFileStatus::Error;
                }
            } else {
                self.file_status = LogFileStatus::Error;
            }
        } else {
            self.file_status = LogFileStatus::NotFound;
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
