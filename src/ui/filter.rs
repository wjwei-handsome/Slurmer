use crossterm::event::KeyModifiers;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Position, Rect},
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Tabs},
};
use regex::Regex;

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
    /// Node regex filter
    pub node_filter: String,
    /// Whether the name regex is valid
    pub name_regex_valid: Option<bool>,
    /// Whether the node regex is valid
    pub node_regex_valid: Option<bool>,
}

/// Which field is currently focused in the filter popup
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterFocus {
    Username,
    States,
    Partitions,
    QoS,
    NameFilter,
    NodeFilter,
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
            node_filter: String::new(),
            name_regex_valid: None,
            node_regex_valid: None,
        }
    }

    /// Initialize filter popup with current options
    pub fn initialize(&mut self, options: &SqueueOptions) {
        self.username = options.user.clone().unwrap_or_default();
        self.name_filter = options.name_filter.clone().unwrap_or_default();
        self.node_filter = options.node_filter.clone().unwrap_or_default();

        // Validate regex if name_filter is not empty
        if !self.name_filter.is_empty() {
            self.validate_name_regex();
        } else {
            self.name_regex_valid = None;
        }

        // Validate regex if node_filter is not empty
        if !self.node_filter.is_empty() {
            self.validate_node_regex();
        } else {
            self.node_regex_valid = None;
        }
    }

    /// Validate the current name regex pattern
    fn validate_name_regex(&mut self) {
        if self.name_filter.is_empty() {
            self.name_regex_valid = None;
            return;
        }

        match Regex::new(&self.name_filter) {
            Ok(_) => self.name_regex_valid = Some(true),
            Err(_) => self.name_regex_valid = Some(false),
        }
    }

    /// Validate the current node regex pattern
    fn validate_node_regex(&mut self) {
        if self.node_filter.is_empty() {
            self.node_regex_valid = None;
            return;
        }

        match Regex::new(&self.node_filter) {
            Ok(_) => self.node_regex_valid = Some(true),
            Err(_) => self.node_regex_valid = Some(false),
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

        // Create an inner area for the content
        let inner_area = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([
                Constraint::Length(3), // Tabs
                Constraint::Min(5),    // Content area
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
        .highlight_style(Style::default().fg(Color::Yellow));

        frame.render_widget(tabs, inner_area[0]);

        // Render the appropriate tab content
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
                Constraint::Length(1), // Spacer
                Constraint::Length(3), // Node filter
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
        // Show validation status in the title for name filter
        let name_title = match self.name_regex_valid {
            Some(true) => "Job Name Filter (regex) ✓",
            Some(false) => "Job Name Filter (regex) ✗ Invalid",
            None => "Job Name Filter (regex)",
        };

        // Set color based on validation status for name filter
        let name_block_style = match (self.focus == FilterFocus::NameFilter, self.name_regex_valid)
        {
            (true, _) => Style::default().fg(Color::Cyan),
            (false, Some(true)) => Style::default(),
            (false, Some(false)) => Style::default().fg(Color::Red),
            (false, None) => Style::default(),
        };

        let name_filter_block = Block::default()
            .title(name_title)
            .borders(Borders::ALL)
            .style(name_block_style);

        let name_filter_text = Paragraph::new(self.name_filter.clone()).block(name_filter_block);

        frame.render_widget(name_filter_text, chunks[2]);

        // Node filter field
        // Show validation status in the title for node filter
        let node_title = match self.node_regex_valid {
            Some(true) => "Node Filter (regex) ✓",
            Some(false) => "Node Filter (regex) ✗ Invalid",
            None => "Node Filter (regex)",
        };

        // Set color based on validation status for node filter
        let node_block_style = match (self.focus == FilterFocus::NodeFilter, self.node_regex_valid)
        {
            (true, _) => Style::default().fg(Color::Cyan),
            (false, Some(true)) => Style::default(),
            (false, Some(false)) => Style::default().fg(Color::Red),
            (false, None) => Style::default(),
        };

        let node_filter_block = Block::default()
            .title(node_title)
            .borders(Borders::ALL)
            .style(node_block_style);

        let node_filter_text = Paragraph::new(self.node_filter.clone()).block(node_filter_block);

        frame.render_widget(node_filter_text, chunks[4]);

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
                FilterFocus::NodeFilter => (
                    chunks[4].x + 1 + self.node_filter.len() as u16,
                    chunks[4].y + 1,
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
                    // self.cycle_focus();
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
                    FilterFocus::Username | FilterFocus::NameFilter | FilterFocus::NodeFilter => {
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
                        // Check if both regexes are valid (or empty)
                        let name_valid =
                            self.name_filter.is_empty() || self.name_regex_valid == Some(true);
                        let node_valid =
                            self.node_filter.is_empty() || self.node_regex_valid == Some(true);

                        if !name_valid || !node_valid {
                            // At least one regex is invalid - don't apply filters
                            return FilterAction::None;
                        }

                        // Apply user filter
                        if !self.username.is_empty() {
                            options.user = Some(self.username.clone());
                        } else {
                            options.user = None;
                        }

                        // Apply name filter if valid
                        if !self.name_filter.is_empty() {
                            options.name_filter = Some(self.name_filter.clone());
                        } else {
                            options.name_filter = None;
                        }

                        // Apply node filter if valid
                        if !self.node_filter.is_empty() {
                            options.node_filter = Some(self.node_filter.clone());
                        } else {
                            options.node_filter = None;
                        }

                        FilterAction::Apply
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
                    FilterFocus::NodeFilter => {
                        // Move focus to the name filter if up is pressed in the node filter
                        self.focus = FilterFocus::NameFilter;
                    }
                    FilterFocus::NameFilter => {
                        // Move focus to the username if up is pressed in the name filter
                        self.focus = FilterFocus::Username;
                    }
                    FilterFocus::ApplyButton | FilterFocus::CancelButton => {
                        // Move focus to node filter from buttons
                        self.focus = FilterFocus::NodeFilter;
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
                    FilterFocus::Username => {
                        // Move focus to the name filter if down is pressed in the username field
                        if self.input_mode {
                            self.input_mode = false;
                        }
                        self.focus = FilterFocus::NameFilter;
                    }
                    FilterFocus::NameFilter => {
                        // Move focus to the node filter if down is pressed in the name filter
                        if self.input_mode {
                            self.input_mode = false;
                        }
                        self.focus = FilterFocus::NodeFilter;
                    }
                    FilterFocus::NodeFilter => {
                        // Move focus to the apply button if down is pressed in the node filter
                        if self.input_mode {
                            self.input_mode = false;
                        }
                        self.focus = FilterFocus::ApplyButton;
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
                            self.name_regex_valid = None;
                        } else if self.name_regex_valid == Some(true) {
                            options.name_filter = Some(self.name_filter.clone());
                        }
                        // If invalid, leave the existing filter unchanged
                    }
                    FilterFocus::NodeFilter => {
                        // Only set node_filter if regex is valid or empty
                        if self.node_filter.is_empty() {
                            options.node_filter = None;
                            self.node_regex_valid = None;
                        } else if self.node_regex_valid == Some(true) {
                            options.node_filter = Some(self.node_filter.clone());
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
                        self.validate_name_regex();
                    }
                    FilterFocus::NodeFilter => {
                        self.node_filter.push(c);
                        self.validate_node_regex();
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
                        self.validate_name_regex();
                    }
                    FilterFocus::NodeFilter => {
                        let _ = self.node_filter.pop();
                        self.validate_node_regex();
                    }
                    _ => {}
                }
                FilterAction::None
            }
            _ => FilterAction::None,
        }
    }

    /// Cycle focus between the focusable elements in the current tab
    fn cycle_focus(&mut self) {
        match self.tab_index {
            0 => {
                // User tab
                match self.focus {
                    FilterFocus::Username => self.focus = FilterFocus::NameFilter,
                    FilterFocus::NameFilter => self.focus = FilterFocus::NodeFilter,
                    FilterFocus::NodeFilter => self.focus = FilterFocus::ApplyButton,
                    FilterFocus::ApplyButton => self.focus = FilterFocus::CancelButton,
                    FilterFocus::CancelButton => self.focus = FilterFocus::Username,
                    _ => self.focus = FilterFocus::Username,
                }
            }
            1 => {
                // States tab
                match self.focus {
                    FilterFocus::States => self.focus = FilterFocus::ApplyButton,
                    FilterFocus::ApplyButton => self.focus = FilterFocus::CancelButton,
                    FilterFocus::CancelButton => self.focus = FilterFocus::States,
                    _ => self.focus = FilterFocus::States,
                }
            }
            2 => {
                // Partitions tab
                match self.focus {
                    FilterFocus::Partitions => self.focus = FilterFocus::ApplyButton,
                    FilterFocus::ApplyButton => self.focus = FilterFocus::CancelButton,
                    FilterFocus::CancelButton => self.focus = FilterFocus::Partitions,
                    _ => self.focus = FilterFocus::Partitions,
                }
            }
            3 => {
                // QoS tab
                match self.focus {
                    FilterFocus::QoS => self.focus = FilterFocus::ApplyButton,
                    FilterFocus::ApplyButton => self.focus = FilterFocus::CancelButton,
                    FilterFocus::CancelButton => self.focus = FilterFocus::QoS,
                    _ => self.focus = FilterFocus::QoS,
                }
            }
            _ => {}
        }
    }

    /// Update focus when tab changes
    fn update_focus_for_tab(&mut self) {
        match self.tab_index {
            0 => {
                // User tab
                match self.focus {
                    FilterFocus::States | FilterFocus::Partitions | FilterFocus::QoS => {
                        self.focus = FilterFocus::Username;
                    }
                    _ => {}
                }
            }
            1 => {
                // States tab
                match self.focus {
                    FilterFocus::Username
                    | FilterFocus::NameFilter
                    | FilterFocus::NodeFilter
                    | FilterFocus::Partitions
                    | FilterFocus::QoS => {
                        self.focus = FilterFocus::States;
                    }
                    _ => {}
                }
            }
            2 => {
                // Partitions tab
                match self.focus {
                    FilterFocus::Username
                    | FilterFocus::NameFilter
                    | FilterFocus::NodeFilter
                    | FilterFocus::States
                    | FilterFocus::QoS => {
                        self.focus = FilterFocus::Partitions;
                    }
                    _ => {}
                }
            }
            3 => {
                // QoS tab
                match self.focus {
                    FilterFocus::Username
                    | FilterFocus::NameFilter
                    | FilterFocus::NodeFilter
                    | FilterFocus::States
                    | FilterFocus::Partitions => {
                        self.focus = FilterFocus::QoS;
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }
}

/// Action to take after handling a key in the filter popup
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterAction {
    /// Do nothing
    None,
    /// Close the filter popup without applying changes
    Close,
    /// Apply the filter changes and close the popup
    Apply,
}
