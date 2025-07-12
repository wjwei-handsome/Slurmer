use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
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
    pub use_bat: bool, // If bat exists, use it for syntax highlighting
}

impl JobScript {
    pub fn new() -> Self {
        let use_bat = is_bat_installed();
        Self {
            visible: false,
            job_id: None,
            content: String::new(),
            scroll_position: 0,
            script_path: None,
            use_bat,
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

        let help_text = " [↑/↓] Scroll | [Ctrl+u/d] PageUp/Down | [q] Close ";

        // Create text with line numbers if enabled
        let text = self.create_display_text();

        let script_paragraph = Paragraph::new(text)
            // .style(Style::default().fg(Color::White))
            // .style(Style::default().bg(Color::LightCyan))
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
    fn create_display_text(&self) -> Text {
        if self.use_bat {
            let lines = parse_ansi_to_spans(&self.content);
            return Text::from(lines);
        }

        let content_lines: Vec<&str> = self.content.lines().collect();
        let total_lines = content_lines.len();

        // Calculate the width needed for line numbers
        let line_num_width = total_lines.to_string().len() + 1; // +1 for the separator

        // Create lines with line numbers
        let mut numbered_lines: Vec<Line> = Vec::new();

        for (i, line) in content_lines.iter().enumerate() {
            let line_num = i + 1;
            let mut spans = Vec::new();

            spans.push(Span::styled(
                format!("{:>width$} ", line_num, width = line_num_width - 1),
                Style::default().fg(Color::DarkGray),
            ));

            spans.push(Span::raw(*line));
            numbered_lines.push(Line::from(spans));
        }

        Text::from(numbered_lines)
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

                        if self.use_bat {
                            // If bat is installed, use it to create a syntax-highlighted version
                            if let Some(bat_output) = create_bat_out_string(script_path) {
                                self.content = bat_output;
                                return;
                            }
                        }
                        // If bat is not available, read the script directly
                        self.use_bat = false;
                        // Now read the script content
                        if let Ok(script_content) = std::fs::read_to_string(script_path) {
                            self.content = script_content;
                        } else {
                            self.content =
                                format!("Failed to read script from path: {}", script_path);
                        }
                    } else {
                        self.content =
                            String::from("No script found for this job. Maybe it's wrapped");
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

/// Use bat to create a syntax-highlighted version of the script
fn create_bat_out_string(path: &str) -> Option<String> {
    let output = Command::new("bat")
        .arg("--style=numbers,grid")
        .arg("--color=always")
        .arg("--theme=GitHub")
        .arg("--terminal-width=100")
        .arg(path)
        .output();
    if let Ok(output) = output {
        if output.status.success() {
            eprintln!("output:{}", String::from_utf8_lossy(&output.stdout));
            Some(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            None
        }
    } else {
        None
    }
}

/// Parse ANSI escape sequences into ratatui spans
fn parse_ansi_to_spans(ansi_text: &str) -> Vec<Line> {
    use regex::Regex;

    // Regex to match ANSI color escape sequences
    let ansi_escape_re = Regex::new(r"\x1B\[([0-9;]*)m").unwrap();
    let lines = ansi_text.lines();
    let mut result_lines = Vec::new();

    let mut current_style = Style::default();

    for line in lines {
        let mut spans = Vec::new();
        let mut last_index = 0;

        // Find all ANSI escape sequences in the line
        for cap in ansi_escape_re.captures_iter(line) {
            let full_match = cap.get(0).unwrap();
            let code_match = cap.get(1).unwrap();

            // Get the text before this escape sequence
            if full_match.start() > last_index {
                let text = &line[last_index..full_match.start()];
                spans.push(Span::styled(text, current_style));
            }

            // Update style based on the ANSI code
            let code = code_match.as_str();
            current_style = parse_ansi_code(code, current_style);

            last_index = full_match.end();
        }

        // Add remaining text after the last escape sequence
        if last_index < line.len() {
            let text = &line[last_index..];
            if !text.is_empty() {
                spans.push(Span::styled(text, current_style));
            }
        }

        // Reset style at end of line
        current_style = Style::default();

        // Add the line if it has spans
        if !spans.is_empty() {
            result_lines.push(Line::from(spans));
        } else {
            // Add empty line
            result_lines.push(Line::default());
        }
    }

    result_lines
}

/// Convert ANSI color code to ratatui Style
fn parse_ansi_code(code: &str, mut style: Style) -> Style {
    // Split by semicolons
    let parts: Vec<&str> = code.split(';').collect();

    // 创建迭代器以处理连续的参数
    let mut iter = parts.iter();

    while let Some(&part) = iter.next() {
        match part {
            "0" => {
                // Reset all attributes
                style = Style::default();
            }
            "1" => {
                // Bold
                style = style.add_modifier(Modifier::BOLD);
            }
            "3" => {
                // Italic
                style = style.add_modifier(Modifier::ITALIC);
            }
            "4" => {
                // Underline
                style = style.add_modifier(Modifier::UNDERLINED);
            }
            // 处理 256 色模式的前景色 (38;5;n)
            "38" => {
                if let Some(&"5") = iter.next() {
                    if let Some(&color_idx_str) = iter.next() {
                        if let Ok(color_idx) = color_idx_str.parse::<u8>() {
                            style = style.fg(Color::Indexed(color_idx));
                        }
                    }
                }
            }
            // 处理 256 色模式的背景色 (48;5;n)
            "48" => {
                if let Some(&"5") = iter.next() {
                    if let Some(&color_idx_str) = iter.next() {
                        if let Ok(color_idx) = color_idx_str.parse::<u8>() {
                            style = style.bg(Color::Indexed(color_idx));
                        }
                    }
                }
            }
            // 基本 16 色前景色 (30-37, 90-97)
            s if s.len() <= 3 && s.starts_with("3") => {
                if let Ok(idx) = s[1..].parse::<u8>() {
                    style = style.fg(Color::Indexed(idx));
                }
            }
            s if s.len() <= 3 && s.starts_with("9") => {
                if let Ok(idx) = s[1..].parse::<u8>() {
                    // 亮色从 ANSI 90-97 映射到 索引 8-15
                    style = style.fg(Color::Indexed(idx + 8));
                }
            }
            // 基本 16 色背景色 (40-47, 100-107)
            s if s.len() <= 3 && s.starts_with("4") && s != "48" => {
                if let Ok(idx) = s[1..].parse::<u8>() {
                    style = style.bg(Color::Indexed(idx));
                }
            }
            s if s.len() <= 4 && s.starts_with("10") => {
                if let Ok(idx) = s[2..].parse::<u8>() {
                    // 亮背景色从 ANSI 100-107 映射到 索引 8-15
                    style = style.bg(Color::Indexed(idx + 8));
                }
            }
            _ => {}
        }
    }

    style
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

/// Check if bat is installed on the system
fn is_bat_installed() -> bool {
    let output = Command::new("which").arg("bat").output();

    match output {
        Ok(output) => output.status.success(),
        Err(_) => {
            // Try the Windows "where" command as fallback
            let windows_output = Command::new("where").arg("bat").output();

            matches!(windows_output, Ok(output) if output.status.success())
        }
    }
}
