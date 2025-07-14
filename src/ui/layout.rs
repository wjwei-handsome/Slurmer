use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use std::time::Duration;

/// Defines the main layout of the application
pub fn draw_main_layout(frame: &mut Frame) -> Vec<Rect> {
    let size = frame.area();

    // Create the main layout
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header area with status
            Constraint::Min(10),   // Main content area
            Constraint::Length(3), // Footer area with controls
        ])
        .split(size);

    let main_chunk = chunks[1];

    vec![chunks[0], main_chunk, chunks[2]]
}

/// Draws the application header with status information
pub fn draw_header(
    frame: &mut Frame,
    area: Rect,
    status_text: &str,
    time_since_refresh: Duration,
    refresh_interval: u64,
) {
    // Split the header area into title and status
    let header_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(20), // Title
            Constraint::Percentage(80), // Status
        ])
        .split(area);

    // Render the title part
    let title = Paragraph::new(Text::from(vec![Line::from(vec![
        Span::styled("SLURMER", Style::default().fg(Color::Cyan).bold()),
        Span::raw(" - "),
        Span::styled("Slurm Terminal UI", Style::default().fg(Color::White)),
    ])]))
    .block(Block::default().borders(Borders::ALL));

    frame.render_widget(title, header_chunks[0]);

    // Render the status part
    let status_info = format!(
        "{} | Refresh: {}s ago (auto: {}s)",
        status_text,
        time_since_refresh.as_secs(),
        refresh_interval
    );

    let status = Paragraph::new(status_info)
        .block(Block::default().borders(Borders::ALL))
        .style(Style::default());

    frame.render_widget(status, header_chunks[1]);
}

/// Draws the application footer with help text and status
pub fn draw_footer(frame: &mut Frame, area: Rect, job_stat: (usize, usize, usize)) {
    // Controls help (lower part of footer)
    let color_style = Style::default().fg(Color::Cyan);
    let text_hashmap = [
        ("q", "Quit"),
        ("↑/↓", "Navigate"),
        ("Space", "Select"),
        ("Enter", "Script"),
        ("f", "Filter"),
        ("c", "Columns"),
        ("v", "Log"),
        ("a", "SelectAll"),
        ("r", "Refresh"),
        ("x", "Cancel"),
    ];

    let mut footer_text: Vec<Span> = text_hashmap
        .iter()
        .flat_map(|(key, description)| {
            vec![
                Span::styled(*key, color_style),
                Span::raw(": "),
                Span::raw(*description),
                Span::raw(" "),
            ]
        })
        .collect();

    footer_text.push(Span::styled("Job Stat: ", Style::default().fg(Color::Cyan)));
    footer_text.push(Span::styled(
        format!("P[ {} ] ", job_stat.0),
        Style::default().fg(Color::Yellow),
    ));
    footer_text.push(Span::styled(
        format!("R[ {} ] ", job_stat.1),
        Style::default().fg(Color::Green),
    ));
    footer_text.push(Span::styled(
        format!("Other[ {} ]", job_stat.2),
        Style::default().fg(Color::Blue),
    ));

    let footer =
        Paragraph::new(Line::from(footer_text)).block(Block::default().borders(Borders::ALL));

    frame.render_widget(footer, area);
}

/// Creates a popup area in the center of the screen
pub fn centered_popup_area(frame_size: Rect, percent_x: u16, percent_y: u16) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(frame_size);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
