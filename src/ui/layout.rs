use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Tabs},
};

/// Defines the main layout of the application
pub fn draw_main_layout(frame: &mut Frame) -> Vec<Rect> {
    let size = frame.size();

    // Create the main layout
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header area
            Constraint::Min(10),   // Main content area
            Constraint::Length(3), // Footer area
        ])
        .split(size);

    // Draw the header
    draw_header(frame, chunks[0]);

    // Draw the footer
    draw_footer(frame, chunks[2]);

    // Split the main content area
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(70), // Job list
            Constraint::Percentage(30), // Details or filters
        ])
        .split(chunks[1]);

    vec![chunks[0], main_chunks[0], main_chunks[1], chunks[2]]
}

/// Draws the application header
fn draw_header(frame: &mut Frame, area: Rect) {
    let header = Paragraph::new(Text::from(vec![Line::from(vec![
        Span::styled("SLURMER", Style::default().fg(Color::Cyan).bold()),
        Span::raw(" - "),
        Span::styled("Slurm Terminal UI", Style::default().fg(Color::White)),
    ])]))
    .block(Block::default().borders(Borders::ALL));

    frame.render_widget(header, area);
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

/// Draws the application footer with help text
fn draw_footer(frame: &mut Frame, area: Rect) {
    let footer_text = vec![
        Span::styled("q", Style::default().fg(Color::Cyan)),
        Span::raw(": Quit | "),
        Span::styled("↑/↓", Style::default().fg(Color::Cyan)),
        Span::raw(": Navigate | "),
        Span::styled("Enter", Style::default().fg(Color::Cyan)),
        Span::raw(": Select | "),
        Span::styled("f", Style::default().fg(Color::Cyan)),
        Span::raw(": Filter"),
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
