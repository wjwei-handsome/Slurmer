use crossterm::event::KeyModifiers;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Position, Rect},
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Tabs},
};

use crate::slurm::{JobState, squeue::SqueueOptions};

/// Filter popup state
pub struct FilterPopup {
    /// Current tab index
    pub tab_index: usize,
    /// Username filter
    pub username: String,
    /// Input mode - is the user typing?
    pub input_mode: bool,
    /// Filter is focused on which field
    pub focus: FilterFocus,
    /// Job state filter list state
    pub state_list_state: ListState,
    /// Partition filter list state
    pub partition_list_state: ListState,
    /// QoS filter list state
    pub qos_list_state: ListState,
    /// Job name regex filter
    pub name_filter: String,
    /// Whether the current regex is valid
    pub regex_valid: Option<bool>,
}

/// Which field is currently focused in the filter popup
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterFocus {
    Username,
    States,
    Partitions,
    QoS,
    NameFilter,
    ApplyButton,
    CancelButton,
}

impl FilterPopup {
    /// Create a new filter popup
    pub fn new() -> Self {
        let mut state_list_state = ListState::default();
        state_list_state.select(Some(0));

        let mut partition_list_state = ListState::default();
        partition_list_state.select(Some(0));

        let mut qos_list_state = ListState::default();
        qos_list_state.select(Some(0));

        Self {
            tab_index: 0,
            username: String::new(),
            input_mode: false,
            focus: FilterFocus::Username,
            state_list_state,
            partition_list_state,
            qos_list_state,
            name_filter: String::new(),
            regex_valid: None,
        }
    }

    /// Initialize filter popup with current options
    pub fn initialize(&mut self, options: &SqueueOptions) {
        self.username = options.user.clone().unwrap_or_default();
        self.name_filter = options.name_filter.clone().unwrap_or_default();

        // Validate regex if name_filter is not empty
        if !self.name_filter.is_empty() {
            self.validate_regex();
        } else {
            self.regex_valid = None;
        }
    }

    /// Validate the current regex pattern
    fn validate_regex(&mut self) {
        if self.name_filter.is_empty() {
            self.regex_valid = None;
            return;
        }

        match regex::Regex::new(&self.name_filter) {
            Ok(_) => self.regex_valid = Some(true),
            Err(_) => self.regex_valid = Some(false),
        }
    }

    /// Render the filter popup
    pub fn render(
        &mut self,
        frame: &mut Frame,
        area: Rect,
        options: &SqueueOptions,
        all_states: &[JobState],
        all_partitions: &[String],
        all_qos: &[String],
    ) {
        frame.render_widget(Clear, area);
        // Create a block for the popup
        let block = Block::default()
            .title("Filter Jobs")
            .borders(Borders::ALL)
            .style(Style::default().bg(Color::Black));

        // Render the popup block
        frame.render_widget(block.clone(), area);

        // Create the inner area for the content
        let inner_area = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([
                Constraint::Length(3), // Tabs
                Constraint::Min(2),    // Content
                Constraint::Length(3), // Buttons
            ])
            .split(area);

        // Create the tabs
        let tabs = Tabs::new(vec![
            Line::from("User & Name"),
            Line::from("States"),
            Line::from("Partitions"),
            Line::from("QoS"),
        ])
        .block(Block::default().borders(Borders::BOTTOM))
        .select(self.tab_index)
        .highlight_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );

        frame.render_widget(tabs, inner_area[0]);

        // Render the content based on the selected tab
        match self.tab_index {
            0 => self.render_user_tab(frame, inner_area[1]),
            1 => self.render_states_tab(frame, inner_area[1], options, all_states),
            2 => self.render_partitions_tab(frame, inner_area[1], options, all_partitions),
            3 => self.render_qos_tab(frame, inner_area[1], options, all_qos),
            _ => {}
        }

        // Render the buttons
        self.render_buttons(frame, inner_area[2]);
    }

    /// Render the user and name filter tab
    fn render_user_tab(&self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([
                Constraint::Length(3), // Username
                Constraint::Length(1), // Spacer
                Constraint::Length(3), // Job name filter
            ])
            .split(area);

        // Username field
        let username_block = Block::default()
            .title("Username")
            .borders(Borders::ALL)
            .style(if self.focus == FilterFocus::Username {
                Style::default().fg(Color::Cyan)
            } else {
                Style::default()
            });

        let username_text = Paragraph::new(self.username.clone()).block(username_block);

        frame.render_widget(username_text, chunks[0]);

        // Job name filter field
        // Show validation status in the title
        let title = match self.regex_valid {
            Some(true) => "Job Name Filter (regex) ✓",
            Some(false) => "Job Name Filter (regex) ✗ Invalid",
            None => "Job Name Filter (regex)",
        };

        // Set color based on validation status
        let block_style = match (self.focus == FilterFocus::NameFilter, self.regex_valid) {
            (true, _) => Style::default().fg(Color::Cyan),
            (false, Some(true)) => Style::default(),
            (false, Some(false)) => Style::default().fg(Color::Red),
            (false, None) => Style::default(),
        };

        let name_filter_block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .style(block_style);

        let name_filter_text = Paragraph::new(self.name_filter.clone()).block(name_filter_block);

        frame.render_widget(name_filter_text, chunks[2]);

        // Show cursor when in input mode
        if self.input_mode {
            let cursor_position = match self.focus {
                FilterFocus::Username => (
                    chunks[0].x + 1 + self.username.len() as u16,
                    chunks[0].y + 1,
                ),
                FilterFocus::NameFilter => (
                    chunks[2].x + 1 + self.name_filter.len() as u16,
                    chunks[2].y + 1,
                ),
                _ => (0, 0),
            };

            if cursor_position != (0, 0) {
                frame.set_cursor_position(Position {
                    x: cursor_position.0,
                    y: cursor_position.1,
                });
            }
        }
    }

    /// Render the job states filter tab
    fn render_states_tab(
        &mut self,
        frame: &mut Frame,
        area: Rect,
        options: &SqueueOptions,
        all_states: &[JobState],
    ) {
        let state_block = Block::default()
            .title("Job States")
            .borders(Borders::ALL)
            .style(if self.focus == FilterFocus::States {
                Style::default().fg(Color::Cyan)
            } else {
                Style::default()
            });

        let state_items: Vec<ListItem> = all_states
            .iter()
            .map(|state| {
                let is_selected = options.states.contains(state);
                let prefix = if is_selected { "[X] " } else { "[ ] " };
                ListItem::new(Line::from(format!("{}{}", prefix, state))).style(
                    Style::default().fg(if is_selected {
                        Color::Green
                    } else {
                        Color::White
                    }),
                )
            })
            .collect();

        let state_list = List::new(state_items)
            .block(state_block)
            .highlight_style(Style::default().add_modifier(Modifier::BOLD));

        frame.render_stateful_widget(state_list, area, &mut self.state_list_state);
    }

    /// Render the partitions filter tab
    fn render_partitions_tab(
        &mut self,
        frame: &mut Frame,
        area: Rect,
        options: &SqueueOptions,
        all_partitions: &[String],
    ) {
        let partition_block = Block::default()
            .title("Partitions")
            .borders(Borders::ALL)
            .style(if self.focus == FilterFocus::Partitions {
                Style::default().fg(Color::Cyan)
            } else {
                Style::default()
            });

        let partition_items: Vec<ListItem> = all_partitions
            .iter()
            .map(|partition| {
                let is_selected = options.partitions.contains(partition);
                let prefix = if is_selected { "[X] " } else { "[ ] " };
                ListItem::new(Line::from(format!("{}{}", prefix, partition))).style(
                    Style::default().fg(if is_selected {
                        Color::Green
                    } else {
                        Color::White
                    }),
                )
            })
            .collect();

        let partition_list = List::new(partition_items)
            .block(partition_block)
            .highlight_style(Style::default().add_modifier(Modifier::BOLD));

        frame.render_stateful_widget(partition_list, area, &mut self.partition_list_state);
    }

    /// Render the QoS filter tab
    fn render_qos_tab(
        &mut self,
        frame: &mut Frame,
        area: Rect,
        options: &SqueueOptions,
        all_qos: &[String],
    ) {
        let qos_block = Block::default()
            .title("Quality of Service")
            .borders(Borders::ALL)
            .style(if self.focus == FilterFocus::QoS {
                Style::default().fg(Color::Cyan)
            } else {
                Style::default()
            });

        let qos_items: Vec<ListItem> = all_qos
            .iter()
            .map(|qos| {
                let is_selected = options.qos.contains(qos);
                let prefix = if is_selected { "[X] " } else { "[ ] " };
                ListItem::new(Line::from(format!("{}{}", prefix, qos))).style(Style::default().fg(
                    if is_selected {
                        Color::Green
                    } else {
                        Color::White
                    },
                ))
            })
            .collect();

        let qos_list = List::new(qos_items)
            .block(qos_block)
            .highlight_style(Style::default().add_modifier(Modifier::BOLD));

        frame.render_stateful_widget(qos_list, area, &mut self.qos_list_state);
    }

    /// Render the apply and cancel buttons
    fn render_buttons(&self, frame: &mut Frame, area: Rect) {
        let button_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area);

        // Apply button
        let apply_style = if self.focus == FilterFocus::ApplyButton {
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Green)
        };

        let apply_button = Paragraph::new("Apply (F10 or Ctrl+A)")
            .style(apply_style)
            .block(Block::default().borders(Borders::ALL));

        frame.render_widget(apply_button, button_chunks[0]);

        // Cancel button
        let cancel_style = if self.focus == FilterFocus::CancelButton {
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Red)
        };

        let cancel_button = Paragraph::new("Cancel (Esc)")
            .style(cancel_style)
            .block(Block::default().borders(Borders::ALL));

        frame.render_widget(cancel_button, button_chunks[1]);
    }

    /// Handle key events for the filter popup
    pub fn handle_key(
        &mut self,
        key: crossterm::event::KeyEvent,
        options: &mut SqueueOptions,
        all_states: &[JobState],
        all_partitions: &[String],
        all_qos: &[String],
    ) -> FilterAction {
        use crossterm::event::KeyCode;

        // Handle global keys first
        match key.code {
            KeyCode::Esc => return FilterAction::Close,
            KeyCode::Tab => {
                if self.input_mode {
                    // Exit input mode on Tab
                    self.input_mode = false;
                } else {
                    // Cycle through focusable elements
                    self.cycle_focus();
                }
                return FilterAction::None;
            }
            // F10 or Ctrl+A to apply filters
            KeyCode::F(10) => return FilterAction::Apply,
            KeyCode::Char('a') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                return FilterAction::Apply;
            }
            _ => {}
        }

        // Handle input mode separately
        if self.input_mode {
            return self.handle_input_mode(key, options);
        }

        // Normal navigation mode
        match key.code {
            KeyCode::Enter => {
                match self.focus {
                    FilterFocus::Username | FilterFocus::NameFilter => {
                        self.input_mode = true;
                        FilterAction::None
                    }
                    FilterFocus::States => {
                        if let Some(idx) = self.state_list_state.selected() {
                            if idx < all_states.len() {
                                let state = all_states[idx];
                                // Toggle the state
                                if options.states.contains(&state) {
                                    options.states.retain(|s| s != &state);
                                } else {
                                    options.states.push(state);
                                }
                            }
                        }
                        FilterAction::None
                    }
                    FilterFocus::Partitions => {
                        if let Some(idx) = self.partition_list_state.selected() {
                            if idx < all_partitions.len() {
                                let partition = all_partitions[idx].clone();
                                // Toggle the partition
                                if options.partitions.contains(&partition) {
                                    options.partitions.retain(|p| p != &partition);
                                } else {
                                    options.partitions.push(partition);
                                }
                            }
                        }
                        FilterAction::None
                    }
                    FilterFocus::QoS => {
                        if let Some(idx) = self.qos_list_state.selected() {
                            if idx < all_qos.len() {
                                let qos = all_qos[idx].clone();
                                // Toggle the QoS
                                if options.qos.contains(&qos) {
                                    options.qos.retain(|q| q != &qos);
                                } else {
                                    options.qos.push(qos);
                                }
                            }
                        }
                        FilterAction::None
                    }
                    FilterFocus::ApplyButton => {
                        // Apply any pending changes from input fields
                        if !self.username.is_empty() {
                            options.user = Some(self.username.clone());
                        } else {
                            options.user = None;
                        }

                        // Validate regex pattern before applying
                        if !self.name_filter.is_empty() {
                            // Check if pattern is valid
                            match regex::Regex::new(&self.name_filter) {
                                Ok(_) => {
                                    // Valid regex pattern
                                    options.name_filter = Some(self.name_filter.clone());
                                    self.regex_valid = Some(true);
                                    FilterAction::Apply
                                }
                                Err(_) => {
                                    // Invalid regex pattern - don't apply filters
                                    self.regex_valid = Some(false);
                                    // Return None to stay in the filter popup
                                    FilterAction::None
                                }
                            }
                        } else {
                            options.name_filter = None;
                            self.regex_valid = None;
                            FilterAction::Apply
                        }
                    }
                    FilterFocus::CancelButton => FilterAction::Close,
                }
            }
            KeyCode::Up => {
                match self.focus {
                    FilterFocus::States => {
                        let selected = self.state_list_state.selected().unwrap_or(0);
                        if selected > 0 {
                            self.state_list_state.select(Some(selected - 1));
                        }
                    }
                    FilterFocus::Partitions => {
                        let selected = self.partition_list_state.selected().unwrap_or(0);
                        if selected > 0 {
                            self.partition_list_state.select(Some(selected - 1));
                        }
                    }
                    FilterFocus::QoS => {
                        let selected = self.qos_list_state.selected().unwrap_or(0);
                        if selected > 0 {
                            self.qos_list_state.select(Some(selected - 1));
                        }
                    }
                    _ => {}
                }
                FilterAction::None
            }
            KeyCode::Down => {
                match self.focus {
                    FilterFocus::States => {
                        let selected = self.state_list_state.selected().unwrap_or(0);
                        if selected < all_states.len() - 1 {
                            self.state_list_state.select(Some(selected + 1));
                        }
                    }
                    FilterFocus::Partitions => {
                        let selected = self.partition_list_state.selected().unwrap_or(0);
                        if selected < all_partitions.len() - 1 {
                            self.partition_list_state.select(Some(selected + 1));
                        }
                    }
                    FilterFocus::QoS => {
                        let selected = self.qos_list_state.selected().unwrap_or(0);
                        if selected < all_qos.len() - 1 {
                            self.qos_list_state.select(Some(selected + 1));
                        }
                    }
                    _ => {}
                }
                FilterAction::None
            }
            KeyCode::Left => {
                // Change tab
                if self.tab_index > 0 {
                    self.tab_index -= 1;
                    self.update_focus_for_tab();
                }
                FilterAction::None
            }
            KeyCode::Right => {
                // Change tab
                if self.tab_index < 3 {
                    self.tab_index += 1;
                    self.update_focus_for_tab();
                }
                FilterAction::None
            }
            _ => FilterAction::None,
        }
    }

    /// Handle input mode (text editing)
    fn handle_input_mode(
        &mut self,
        key: crossterm::event::KeyEvent,
        options: &mut SqueueOptions,
    ) -> FilterAction {
        use crossterm::event::KeyCode;

        match key.code {
            KeyCode::Enter => {
                // Apply the text input
                match self.focus {
                    FilterFocus::Username => {
                        if self.username.is_empty() {
                            options.user = None;
                        } else {
                            options.user = Some(self.username.clone());
                        }
                    }
                    FilterFocus::NameFilter => {
                        // Only set name_filter if regex is valid or empty
                        if self.name_filter.is_empty() {
                            options.name_filter = None;
                            self.regex_valid = None;
                        } else if self.regex_valid == Some(true) {
                            options.name_filter = Some(self.name_filter.clone());
                        }
                        // If invalid, leave the existing filter unchanged
                    }
                    _ => {}
                }
                self.input_mode = false;
                // Return None instead of immediately applying filters
                // This allows the user to continue modifying other filters
                FilterAction::None
            }
            KeyCode::Char(c) => {
                // Add character to input
                match self.focus {
                    FilterFocus::Username => self.username.push(c),
                    FilterFocus::NameFilter => {
                        self.name_filter.push(c);
                        self.validate_regex();
                    }
                    _ => {}
                }
                FilterAction::None
            }
            KeyCode::Backspace => {
                // Remove character from input
                match self.focus {
                    FilterFocus::Username => {
                        let _ = self.username.pop();
                    }
                    FilterFocus::NameFilter => {
                        let _ = self.name_filter.pop();
                        self.validate_regex();
                    }
                    _ => {}
                }
                FilterAction::None
            }
            _ => FilterAction::None,
        }
    }

    /// Cycle through focusable elements
    fn cycle_focus(&mut self) {
        self.focus = match self.focus {
            FilterFocus::Username => FilterFocus::NameFilter,
            FilterFocus::NameFilter => FilterFocus::ApplyButton,
            FilterFocus::ApplyButton => FilterFocus::CancelButton,
            FilterFocus::CancelButton => FilterFocus::States,
            FilterFocus::States => FilterFocus::Partitions,
            FilterFocus::Partitions => FilterFocus::QoS,
            FilterFocus::QoS => FilterFocus::Username,
        };

        // Make sure the focus is valid for the current tab
        self.update_focus_for_tab();
    }

    /// Update focus to match the current tab
    fn update_focus_for_tab(&mut self) {
        self.focus = match self.tab_index {
            0 => match self.focus {
                FilterFocus::Username | FilterFocus::NameFilter => self.focus,
                _ => FilterFocus::Username,
            },
            1 => FilterFocus::States,
            2 => FilterFocus::Partitions,
            3 => FilterFocus::QoS,
            _ => self.focus,
        };
    }
}

/// Action to take after handling a key in the filter popup
pub enum FilterAction {
    /// Do nothing
    None,
    /// Close the filter popup
    Close,
    /// Apply filters and close
    Apply,
}
