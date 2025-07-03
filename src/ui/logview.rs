use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;
use std::time::{Duration, Instant};

use crate::utils::file_watcher::{FileContent, FileWatcherError, FileWatcherHandle};
use crossbeam::channel::{Receiver, Sender, unbounded};

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, Paragraph, Tabs},
};

pub struct LogView {
    pub visible: bool,
    pub job_id: Option<String>,
    pub stdout_path: Option<String>,
    pub stderr_path: Option<String>,
    pub current_tab: LogTab,
    pub scroll_position: usize,
    pub log_content: Vec<String>,
    pub last_refresh: Instant,
    pub refresh_interval: Duration,
    pub lines_to_show: usize,
    pub auto_scroll: bool,
    pub page_size: usize,
    pub live_mode: bool,
    pub file_watcher: Option<FileWatcherHandle>,
    pub file_watcher_rx: Option<Receiver<Result<FileContent, FileWatcherError>>>,
    pub file_changed_notification: Option<String>,
    pub truncated: bool,
}

#[derive(PartialEq, Clone, Copy)]
pub enum LogTab {
    StdOut,
    StdErr,
}

impl LogView {
    pub fn new() -> Self {
        // Create channels for communication with the file watcher
        let (sender, receiver) = unbounded();

        // Create the file watcher
        let file_watcher = FileWatcherHandle::new(sender, Duration::from_millis(500));

        Self {
            visible: false,
            job_id: None,
            stdout_path: None,
            stderr_path: None,
            current_tab: LogTab::StdOut,
            scroll_position: 0,
            log_content: Vec::new(),
            last_refresh: Instant::now(),
            refresh_interval: Duration::from_secs(2),
            lines_to_show: 20,
            auto_scroll: true,
            page_size: 10,
            live_mode: true,
            file_watcher: Some(file_watcher),
            file_watcher_rx: Some(receiver),
            file_changed_notification: None,
            truncated: false,
        }
    }

    pub fn show(&mut self, job_id: String) {
        self.visible = true;
        self.change_job(job_id);
    }

    pub fn change_job(&mut self, job_id: String) {
        self.job_id = Some(job_id.clone());
        self.stdout_path = None;
        self.stderr_path = None;
        self.scroll_position = 0;
        self.log_content.clear();
        self.current_tab = LogTab::StdOut;
        self.auto_scroll = true;
        self.file_changed_notification = None;

        // Get stdout and stderr paths from scontrol
        self.fetch_log_paths();

        // Start watching the current log file
        self.watch_current_path();
    }

    pub fn hide(&mut self) {
        self.visible = false;
        // Stop watching the file when hiding the log view
        if let Some(watcher) = &mut self.file_watcher {
            watcher.set_file_path(None);
        }
    }

    pub fn toggle_tab(&mut self) {
        self.current_tab = match self.current_tab {
            LogTab::StdOut => LogTab::StdErr,
            LogTab::StdErr => LogTab::StdOut,
        };
        self.scroll_position = 0;
        self.log_content.clear();
        self.file_changed_notification = None;

        // Start watching the new log file based on the current tab
        self.watch_current_path();
    }

    pub fn scroll_up(&mut self) {
        if self.scroll_position > 0 {
            self.scroll_position -= 1;
            self.auto_scroll = false;
        }
    }

    pub fn scroll_down(&mut self) {
        if self.scroll_position < self.log_content.len().saturating_sub(1) {
            self.scroll_position += 1;

            // If we've scrolled to the bottom, re-enable auto-scroll
            if self.scroll_position >= self.log_content.len().saturating_sub(1) {
                self.auto_scroll = true;
            }
        }
    }

    pub fn page_up(&mut self) {
        self.scroll_position = self.scroll_position.saturating_sub(self.page_size);
        self.auto_scroll = false;
    }

    pub fn page_down(&mut self) {
        let max_scroll = self.log_content.len().saturating_sub(1);
        self.scroll_position = (self.scroll_position + self.page_size).min(max_scroll);

        // If we've scrolled to the bottom, re-enable auto-scroll
        if self.scroll_position >= max_scroll {
            self.auto_scroll = true;
        }
    }

    pub fn toggle_auto_scroll(&mut self) {
        self.auto_scroll = !self.auto_scroll;
        if self.auto_scroll {
            // Move to the bottom when auto-scroll is enabled
            self.scroll_position = self.log_content.len().saturating_sub(1);
        }
    }
    pub fn toggle_live_mode(&mut self) {
        self.live_mode = !self.live_mode;
        self.file_changed_notification = None;

        // Refresh the log content immediately after changing mode
        if !self.live_mode {
            self.refresh_log_content();
        }
    }

    pub fn check_refresh(&mut self) {
        if self.visible {
            // Check for new content from the file watcher
            if let Some(rx) = &self.file_watcher_rx {
                // Try to receive all pending messages without blocking
                while let Ok(result) = rx.try_recv() {
                    match result {
                        Ok(file_content) => {
                            // New content received from file watcher
                            if !file_content.content.is_empty() {
                                // Check if file was truncated
                                if file_content.is_truncated {
                                    self.truncated = true;
                                    self.log_content
                                        .push("--- File was truncated or rotated ---".to_string());
                                }

                                // Parse lines and add to log content
                                for line in file_content.content.lines() {
                                    self.log_content.push(line.to_string());
                                }

                                // Limit the number of lines to keep memory usage reasonable
                                let max_lines = 1000; // Reasonable buffer size
                                if self.log_content.len() > max_lines {
                                    self.log_content = self
                                        .log_content
                                        .split_off(self.log_content.len() - max_lines);
                                }

                                // If auto-scroll is enabled, scroll to the bottom
                                if self.auto_scroll {
                                    self.scroll_position = self.log_content.len().saturating_sub(1);
                                }
                            }
                            self.last_refresh = Instant::now();
                        }
                        Err(e) => {
                            // Error from file watcher
                            self.file_changed_notification = Some(format!("Error: {}", e));
                        }
                    }
                }
            }

            // For static mode or backup polling, also refresh periodically
            if !self.live_mode && self.last_refresh.elapsed() >= self.refresh_interval {
                self.refresh_log_content();
                self.last_refresh = Instant::now();
            }
        }
    }

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

    fn watch_current_path(&mut self) {
        let path = match self.current_tab {
            LogTab::StdOut => self.stdout_path.clone(),
            LogTab::StdErr => self.stderr_path.clone(),
        };

        if let Some(watcher) = &mut self.file_watcher {
            if let Some(path_str) = path {
                // Convert string path to PathBuf and send to watcher
                let path_buf = PathBuf::from(path_str);
                watcher.set_file_path(Some(path_buf));
            } else {
                // No path available, stop watching
                watcher.set_file_path(None);
            }
        }

        // Also do an initial content fetch for immediate display
        self.refresh_log_content();
    }

    fn refresh_log_content(&mut self) {
        // Only used in static mode or for initial content
        if self.live_mode {
            return;
        }

        let path = match self.current_tab {
            LogTab::StdOut => &self.stdout_path,
            LogTab::StdErr => &self.stderr_path,
        };

        if let Some(path) = path {
            let output = Command::new("tail")
                .args(["-n", &self.lines_to_show.to_string(), path])
                .output();

            if let Ok(output) = output {
                if output.status.success() {
                    let content = String::from_utf8_lossy(&output.stdout);
                    self.log_content = content.lines().map(|line| line.to_string()).collect();

                    // If auto-scroll is enabled, scroll to the bottom
                    if self.auto_scroll {
                        self.scroll_position = self.log_content.len().saturating_sub(1);
                    }
                } else {
                    eprintln!("tail failed , fuck")
                }
            }
        }
    }

    pub fn render(&self, f: &mut Frame, area: Rect) {
        if !self.visible {
            return;
        }

        let popup_area = crate::ui::layout::centered_popup_area(area, 80, 80);

        // Clear the area
        f.render_widget(Clear, popup_area);

        // Create the popup block
        let job_id = self.job_id.clone().unwrap_or_default();
        let mode_indicator = if self.live_mode { "[LIVE]" } else { "[STATIC]" };
        let title = match self.current_tab {
            LogTab::StdOut => format!("Job {} - Standard Output {}", job_id, mode_indicator),
            LogTab::StdErr => format!("Job {} - Standard Error {}", job_id, mode_indicator),
        };

        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .style(Style::default().bg(Color::Black));

        // Create the layout
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([
                Constraint::Length(2), // Tabs
                Constraint::Min(1),    // Content
                Constraint::Length(1), // Help text
            ])
            .split(popup_area);

        // Render tabs
        let tab_titles = vec!["[o] StdOut", "[e] StdErr"];
        let tabs = Tabs::new(
            tab_titles
                .iter()
                .cloned()
                .map(Line::from)
                .collect::<Vec<_>>(),
        )
        .select(match self.current_tab {
            LogTab::StdOut => 0,
            LogTab::StdErr => 1,
        })
        .block(Block::default().borders(Borders::BOTTOM))
        .style(Style::default())
        .highlight_style(Style::default().add_modifier(Modifier::BOLD));

        f.render_widget(block, popup_area);
        f.render_widget(tabs, chunks[0]);

        // Render log content
        let content_text = if self.log_content.is_empty() {
            if self.stdout_path.is_none() && self.stderr_path.is_none() {
                Text::from("Loading log paths...")
            } else {
                let path_info = match self.current_tab {
                    LogTab::StdOut => {
                        format!("No output found or file is empty: {:?}", self.stdout_path)
                    }
                    LogTab::StdErr => format!(
                        "No error output found or file is empty: {:?}",
                        self.stderr_path
                    ),
                };
                Text::from(path_info)
            }
        } else {
            // Check if we have a file change notification to display
            let mut content = self.log_content.clone();
            if let Some(notification) = &self.file_changed_notification {
                content.push(format!("--- {} ---", notification));
            }

            let visible_content =
                get_visible_content(&content, self.scroll_position, chunks[1].height as usize);
            Text::from(visible_content.join("\n"))
        };

        let content = Paragraph::new(content_text).style(Style::default());

        f.render_widget(content, chunks[1]);

        // Render help text
        let help_text = vec![
            Span::styled("[o] ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw("Toggle Output/Error | "),
            Span::styled("[↑/↓] ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw("Scroll | "),
            Span::styled(
                "[Shift+↑/↓] ",
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::raw("Switch job | "),
            Span::styled(
                "[Ctrl+u/Ctrl+d] ",
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::raw("Page scroll | "),
            Span::styled("[a] ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw("Toggle auto-scroll | "),
            Span::styled("[l] ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw("Toggle live mode | "),
            Span::styled("[Esc/q] ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw("Close"),
        ];

        let help = Paragraph::new(Line::from(help_text)).style(Style::default());

        f.render_widget(help, chunks[2]);
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

fn get_visible_content(
    content: &[String],
    scroll_position: usize,
    max_height: usize,
) -> Vec<String> {
    if content.is_empty() {
        return vec![];
    }

    // Calculate start position based on scroll position and max height
    let start_pos = if content.len() <= max_height {
        0
    } else if scroll_position + max_height > content.len() {
        content.len() - max_height
    } else {
        scroll_position
    };

    content[start_pos..]
        .iter()
        .take(max_height)
        .cloned()
        .collect() // Return only the visible lines
}
