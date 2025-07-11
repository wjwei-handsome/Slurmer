use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Tabs},
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

    // Footer will be drawn by the caller

    // Draw the footer (empty status text, will be updated by caller)
    draw_footer(frame, chunks[2], "");

    // Split the main content area
    // let main_chunks = Layout::default()
    //     .direction(Direction::Vertical)
    //     .constraints([
    //         Constraint::Percentage(70), // Job list
    //         Constraint::Percentage(30), // Details or filters
    //     ])
    //     .split(chunks[1]);
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
            Constraint::Percentage(30), // Title
            Constraint::Percentage(70), // Status
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
        "Status: {} | Refresh: {}s ago (auto: {}s)",
        status_text,
        time_since_refresh.as_secs(),
        refresh_interval
    );

    let status = Paragraph::new(status_info)
        .block(Block::default().borders(Borders::ALL))
        .style(Style::default());

    frame.render_widget(status, header_chunks[1]);
}

/// Draws tabs for different views
pub fn draw_tabs(frame: &mut Frame, area: Rect, titles: &[&str], active_tab: usize) {
    let tab_titles: Vec<Line> = titles
        .iter()
        .map(|t| Line::from(Span::styled(*t, Style::default().fg(Color::White))))
        .collect();

    let tabs = Tabs::new(tab_titles)
        .block(Block::default().borders(Borders::ALL))
        .select(active_tab)
        .highlight_style(Style::default().fg(Color::Cyan).bold());

    frame.render_widget(tabs, area);
}

/// Draws the application footer with help text and status
pub fn draw_footer(frame: &mut Frame, area: Rect, status_text: &str) {
    // Render controls directly in the footer area
    // Controls (lower part of footer)
    let footer_text = vec![
        Span::styled("q", Style::default().fg(Color::Cyan)),
        Span::raw(": Quit | "),
        Span::styled("↑/↓", Style::default().fg(Color::Cyan)),
        Span::raw(": Navigate | "),
        Span::styled("Enter", Style::default().fg(Color::Cyan)),
        Span::raw(": Select | "),
        Span::styled("f", Style::default().fg(Color::Cyan)),
        Span::raw(": Filter | "),
        Span::styled("c", Style::default().fg(Color::Cyan)),
        Span::raw(": Columns | "),
        Span::styled("v", Style::default().fg(Color::Cyan)),
        Span::raw(": ViewLog | "),
        Span::styled("a", Style::default().fg(Color::Cyan)),
        Span::raw(": SelectAll | "),
        Span::styled("r", Style::default().fg(Color::Cyan)),
        Span::raw(": Refresh"),
    ];

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
