use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
};
use std::{collections::HashMap, process::Command};

/// JobScript viewer widget for displaying job batch scripts with syntax highlighting
pub struct JobScript {
    pub visible: bool,
    pub job_id: Option<String>,
    pub content: String,
    pub scroll_position: usize,
    pub script_path: Option<String>,
    pub show_line_numbers: bool,
}

impl JobScript {
    pub fn new() -> Self {
        Self {
            visible: false,
            job_id: None,
            content: String::new(),
            scroll_position: 0,
            script_path: None,
            show_line_numbers: true, // Enable line numbers by default
        }
    }

    /// Show the job script view for a specific job
    pub fn show(&mut self, job_id: String) {
        self.change_job(job_id);
        self.visible = true;
    }

    /// Hide the job script view
    pub fn hide(&mut self) {
        self.visible = false;
    }

    /// Change the job being viewed
    pub fn change_job(&mut self, job_id: String) {
        self.job_id = Some(job_id);
        self.script_path = None;
        self.scroll_position = 0;

        // Fetch the script content
        self.fetch_script_content();
    }

    /// Toggle line numbers display
    pub fn toggle_line_numbers(&mut self) {
        self.show_line_numbers = !self.show_line_numbers;
    }

    /// Scroll the script view up
    pub fn scroll_up(&mut self) {
        if self.scroll_position > 0 {
            self.scroll_position -= 1;
        }
    }

    /// Scroll the script view down
    pub fn scroll_down(&mut self) {
        // We'll use transformed_lines.len() as an upper bound
        let approx_max_lines = self.content.lines().count() * 2;

        if self.scroll_position < approx_max_lines {
            self.scroll_position += 1;
        }
    }

    /// Page up in the script view
    pub fn page_up(&mut self) {
        // Move up by a page (10 lines)
        self.scroll_position = self.scroll_position.saturating_sub(10);
    }

    /// Page down in the script view
    pub fn page_down(&mut self) {
        // Move down by a page (10 lines)
        let approx_max_lines = self.content.lines().count() * 2;

        let new_scroll = self.scroll_position + 10;
        self.scroll_position = if new_scroll < approx_max_lines {
            new_scroll
        } else {
            approx_max_lines
        };
    }

    /// Render the job script view
    pub fn render(&self, frame: &mut Frame, area: Rect) {
        if !self.visible {
            return;
        }

        frame.render_widget(Clear, area);

        let title = match &self.job_id {
            Some(id) => format!("Job Script: {}", id),
            None => String::from("Job Script"),
        };

        let help_text = " [↑/↓] Scroll | [l] Toggle Line Numbers | [q] Close ";

        // Create text with line numbers if enabled
        let text = self.create_display_text(area.height as usize - 2, area.width as usize - 2);

        let script_paragraph = Paragraph::new(text)
            .style(Style::default().fg(Color::White))
            .block(
                Block::default()
                    .title(format!("{}{}", title, help_text))
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Cyan)),
            )
            .wrap(Wrap { trim: false })
            .scroll((self.scroll_position as u16, 0));

        frame.render_widget(script_paragraph, area);
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> () {
        match (key.modifiers, key.code) {
            (_, KeyCode::Char('l')) => {
                // Toggle line numbers
                self.toggle_line_numbers();
            }
            (_, KeyCode::Char('q')) => {
                // Close the script view
                self.hide();
            }
            (_, KeyCode::Up) => {
                // Scroll up
                self.scroll_up();
            }
            (_, KeyCode::Down) => {
                // Scroll down
                self.scroll_down();
            }
            (_, KeyCode::PageUp) | (KeyModifiers::CONTROL, KeyCode::Char('u')) => {
                // Page up
                self.page_up();
            }
            (_, KeyCode::PageDown) | (KeyModifiers::CONTROL, KeyCode::Char('d')) => {
                // Page down
                self.page_down();
            }
            _ => {
                // Ignore other keys
            }
        }
    }

    /// Create display text with optional line numbers
    fn create_display_text(&self, height: usize, width: usize) -> Text {
        let content_lines: Vec<&str> = self.content.lines().collect();
        let total_lines = content_lines.len();

        // Calculate the width needed for line numbers
        let line_num_width = if self.show_line_numbers {
            total_lines.to_string().len() + 1 // +1 for the separator
        } else {
            0
        };

        // Create lines with line numbers
        let mut numbered_lines: Vec<Line> = Vec::new();

        for (i, line) in content_lines.iter().enumerate() {
            let line_num = i + 1;
            let mut spans = Vec::new();

            if self.show_line_numbers {
                spans.push(Span::styled(
                    format!("{:>width$} ", line_num, width = line_num_width - 1),
                    Style::default().fg(Color::DarkGray),
                ));
            }

            spans.push(Span::raw(*line));
            numbered_lines.push(Line::from(spans));
        }

        Text::from(numbered_lines)
    }

    /// Split a long line into chunks of specified width
    fn split_line(line: &str, max_width: usize) -> Vec<String> {
        if line.len() <= max_width {
            return vec![line.to_string()];
        }

        let mut chunks = Vec::new();
        let mut remaining = line;

        while !remaining.is_empty() {
            if remaining.len() <= max_width {
                chunks.push(remaining.to_string());
                break;
            }

            // Find a good split point - preferably at whitespace
            let mut split_at = max_width;
            while split_at > 0 && !remaining.is_char_boundary(split_at) {
                split_at -= 1;
            }

            chunks.push(remaining[..split_at].to_string());
            remaining = &remaining[split_at..];
        }

        chunks
    }

    /// Fetch the job script content using scontrol
    fn fetch_script_content(&mut self) {
        if let Some(job_id) = &self.job_id {
            // First get job details to find BatchScript path
            let output = Command::new("scontrol")
                .args(["show", "job", job_id, "-o"])
                .output();

            if let Ok(output) = output {
                if output.status.success() {
                    let output_str = String::from_utf8_lossy(&output.stdout);
                    let key_value_pairs = parse_scontrol_output(&output_str);

                    // Get the BatchScript path
                    if let Some(script_path) = key_value_pairs.get("Command") {
                        self.script_path = Some(script_path.to_string());

                        // Now read the script content
                        if let Ok(script_content) = std::fs::read_to_string(script_path) {
                            self.content = script_content;
                        } else {
                            self.content =
                                format!("Failed to read script from path: {}", script_path);
                        }
                    } else {
                        self.content = String::from("No batch script found for this job");
                    }
                } else {
                    self.content = String::from("Error retrieving job information");
                }
            } else {
                self.content = String::from("Failed to execute scontrol command");
            }
        } else {
            self.content = String::new();
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
