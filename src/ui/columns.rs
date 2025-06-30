use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Tabs},
};

use crate::slurm::squeue::SqueueOptions;

/// Available columns for display in job list
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum JobColumn {
    Id,
    Name,
    User,
    State,
    Partition,
    QoS,
    Nodes,
    CPUs,
    Time,
    Memory,
    Account,
    Priority,
    WorkDir,
    SubmitTime,
    StartTime,
    EndTime,
}

impl JobColumn {
    /// Get the title for this column
    pub fn title(&self) -> &'static str {
        match self {
            JobColumn::Id => "ID",
            JobColumn::Name => "Name",
            JobColumn::User => "User",
            JobColumn::State => "State",
            JobColumn::Partition => "Partition",
            JobColumn::QoS => "QoS",
            JobColumn::Nodes => "Nodes",
            JobColumn::CPUs => "CPUs",
            JobColumn::Time => "Time",
            JobColumn::Memory => "Memory",
            JobColumn::Account => "Account",
            JobColumn::Priority => "Priority",
            JobColumn::WorkDir => "WorkDir",
            JobColumn::SubmitTime => "Submit",
            JobColumn::StartTime => "Start",
            JobColumn::EndTime => "End",
        }
    }

    /// Get the format code for this column
    pub fn format_code(&self) -> &'static str {
        match self {
            JobColumn::Id => "%A",         // Job ID - using %A for array job ID
            JobColumn::Name => "%j",       // Job name
            JobColumn::User => "%u",       // User name
            JobColumn::State => "%T",      // Job state
            JobColumn::Partition => "%P",  // Partition
            JobColumn::QoS => "%q",        // Quality of Service
            JobColumn::Nodes => "%D",      // Node count
            JobColumn::CPUs => "%C",       // CPU count
            JobColumn::Time => "%M",       // Time limit
            JobColumn::Memory => "%m",     // Memory
            JobColumn::Account => "%a",    // Account
            JobColumn::Priority => "%Q",   // Priority
            JobColumn::WorkDir => "%Z",    // Working directory
            JobColumn::SubmitTime => "%V", // Submission time
            JobColumn::StartTime => "%S",  // Start time
            JobColumn::EndTime => "%e",    // End time
        }
    }

    /// Get the default width constraint for this column
    pub fn default_width(&self) -> Constraint {
        match self {
            JobColumn::Id => Constraint::Length(8),
            JobColumn::Name => Constraint::Percentage(20),
            JobColumn::User => Constraint::Length(10),
            JobColumn::State => Constraint::Length(10),
            JobColumn::Partition => Constraint::Length(10),
            JobColumn::QoS => Constraint::Length(8),
            JobColumn::Nodes => Constraint::Length(6),
            JobColumn::CPUs => Constraint::Length(6),
            JobColumn::Time => Constraint::Length(10),
            JobColumn::Memory => Constraint::Length(8),
            JobColumn::Account => Constraint::Length(10),
            JobColumn::Priority => Constraint::Length(8),
            JobColumn::WorkDir => Constraint::Percentage(15),
            JobColumn::SubmitTime => Constraint::Length(16),
            JobColumn::StartTime => Constraint::Length(16),
            JobColumn::EndTime => Constraint::Length(16),
        }
    }

    /// Get all available columns
    pub fn all() -> Vec<JobColumn> {
        vec![
            JobColumn::Id,
            JobColumn::Name,
            JobColumn::User,
            JobColumn::State,
            JobColumn::Partition,
            JobColumn::QoS,
            JobColumn::Nodes,
            JobColumn::CPUs,
            JobColumn::Time,
            JobColumn::Memory,
            JobColumn::Account,
            JobColumn::Priority,
            JobColumn::WorkDir,
            JobColumn::SubmitTime,
            JobColumn::StartTime,
            JobColumn::EndTime,
        ]
    }

    /// Default columns to display
    pub fn defaults() -> Vec<JobColumn> {
        // These MUST match the defaults in App::new()
        vec![
            JobColumn::Id,
            JobColumn::Name,
            JobColumn::User,
            JobColumn::State,
            JobColumn::Partition,
            JobColumn::QoS,
            JobColumn::Nodes,
            JobColumn::CPUs,
            JobColumn::Time,
        ]
    }
}

/// Sort order for columns
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortOrder {
    Ascending,
    Descending,
}

impl SortOrder {
    /// Toggle the sort order
    pub fn toggle(&self) -> Self {
        match self {
            SortOrder::Ascending => SortOrder::Descending,
            SortOrder::Descending => SortOrder::Ascending,
        }
    }

    /// Get the sort indicator
    pub fn indicator(&self) -> &'static str {
        match self {
            SortOrder::Ascending => "↑",
            SortOrder::Descending => "↓",
        }
    }
}

/// A column with its sort order
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SortColumn {
    pub column: JobColumn,
    pub order: SortOrder,
}

/// Which part of the columns popup is focused
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColumnsFocus {
    AvailableColumns,
    SelectedColumns,
    SortColumns,
    SaveButton,
    ApplyButton,
    CancelButton,
}

/// Columns management popup state
pub struct ColumnsPopup {
    /// Current tab index
    pub tab_index: usize,
    /// Focus in the popup
    pub focus: ColumnsFocus,
    /// Available columns list state
    pub available_columns_state: ListState,
    /// Selected columns list state
    pub selected_columns_state: ListState,
    /// Sort columns list state
    pub sort_columns_state: ListState,
    /// Available columns (those not selected)
    pub available_columns: Vec<JobColumn>,
    /// Selected columns (to display)
    pub selected_columns: Vec<JobColumn>,
    /// Sort columns with their order
    pub sort_columns: Vec<SortColumn>,
}

impl ColumnsPopup {
    /// Create a new columns popup
    pub fn new(selected_columns: Vec<JobColumn>, sort_columns: Vec<SortColumn>) -> Self {
        let mut available_columns = JobColumn::all();
        available_columns.retain(|col| !selected_columns.contains(col));

        let mut available_columns_state = ListState::default();
        if !available_columns.is_empty() {
            available_columns_state.select(Some(0));
        }

        let mut selected_columns_state = ListState::default();
        if !selected_columns.is_empty() {
            selected_columns_state.select(Some(0));
        }

        let mut sort_columns_state = ListState::default();
        if !sort_columns.is_empty() {
            sort_columns_state.select(Some(0));
        }

        Self {
            tab_index: 0,
            focus: ColumnsFocus::SelectedColumns,
            available_columns_state,
            selected_columns_state,
            sort_columns_state,
            available_columns,
            selected_columns,
            sort_columns,
        }
    }

    /// Render the columns popup
    pub fn render(&mut self, frame: &mut Frame, area: Rect) {
        // Create a block for the popup
        let block = Block::default()
            .title("Column Management")
            .borders(Borders::ALL)
            .style(Style::default().bg(Color::Black));

        frame.render_widget(block.clone(), area);

        // Create the inner area for the content
        let inner_area = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([
                Constraint::Length(3), // Tabs
                Constraint::Min(5),    // Content
                Constraint::Length(3), // Buttons
            ])
            .split(area);

        // Create the tabs
        let tabs = Tabs::new(vec![
            Line::from("[1] Available/Selected"),
            Line::from("[2] Sort Order"),
        ])
        .block(Block::default().borders(Borders::BOTTOM))
        .select(self.tab_index)
        .highlight_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );

        frame.render_widget(tabs, inner_area[0]);

        // Render content based on selected tab
        match self.tab_index {
            0 => self.render_columns_tab(frame, inner_area[1]),
            1 => self.render_sort_tab(frame, inner_area[1]),
            _ => {}
        }

        // Render buttons
        self.render_buttons(frame, inner_area[2]);
    }

    /// Render the columns tab (available and selected columns)
    fn render_columns_tab(&mut self, frame: &mut Frame, area: Rect) {
        // Split the area into two columns
        let columns = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area);

        // Available columns list
        let available_block = Block::default()
            .title("Available Columns")
            .borders(Borders::ALL)
            .style(if self.focus == ColumnsFocus::AvailableColumns {
                Style::default().fg(Color::Cyan)
            } else {
                Style::default()
            });

        let available_items: Vec<ListItem> = self
            .available_columns
            .iter()
            .map(|col| ListItem::new(col.title()))
            .collect();

        let available_list = List::new(available_items)
            .block(available_block)
            .highlight_style(Style::default().add_modifier(Modifier::BOLD));

        frame.render_stateful_widget(
            available_list,
            columns[0],
            &mut self.available_columns_state,
        );

        // Selected columns list
        let selected_block = Block::default()
            .title("Selected Columns")
            .borders(Borders::ALL)
            .style(if self.focus == ColumnsFocus::SelectedColumns {
                Style::default().fg(Color::Cyan)
            } else {
                Style::default()
            });

        let selected_items: Vec<ListItem> = self
            .selected_columns
            .iter()
            .map(|col| ListItem::new(col.title()))
            .collect();

        let selected_list = List::new(selected_items)
            .block(selected_block)
            .highlight_style(Style::default().add_modifier(Modifier::BOLD));

        frame.render_stateful_widget(selected_list, columns[1], &mut self.selected_columns_state);

        // Show help text
        let help_area = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(2)])
            .split(area)[1];

        let help_text = "Enter: Add/Remove column | ↑/↓: Navigate | ←/→: Switch lists | Space: Move up/down | t: Switch tabs";
        let help = Paragraph::new(help_text).style(Style::default().fg(Color::Gray));

        frame.render_widget(help, help_area);
    }

    /// Render the sort tab
    fn render_sort_tab(&mut self, frame: &mut Frame, area: Rect) {
        // Sort columns list
        let sort_block = Block::default()
            .title("Sort Order")
            .borders(Borders::ALL)
            .style(if self.focus == ColumnsFocus::SortColumns {
                Style::default().fg(Color::Cyan)
            } else {
                Style::default()
            });

        let sort_items: Vec<ListItem> = self
            .sort_columns
            .iter()
            .map(|sort_col| {
                ListItem::new(format!(
                    "{} {}",
                    sort_col.column.title(),
                    sort_col.order.indicator()
                ))
            })
            .collect();

        let sort_list = List::new(sort_items)
            .block(sort_block)
            .highlight_style(Style::default().add_modifier(Modifier::BOLD));

        frame.render_stateful_widget(sort_list, area, &mut self.sort_columns_state);

        // Show help text at the bottom
        // Show help text at the bottom
        let help_area = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(2)])
            .split(area)[1];

        let help_text = "Enter: Add column to sort | Space: Toggle sort order | Delete: Remove column from sort | t: Switch tabs";
        let help = Paragraph::new(help_text).style(Style::default().fg(Color::Gray));

        frame.render_widget(help, help_area);
    }

    /// Render the buttons
    fn render_buttons(&self, frame: &mut Frame, area: Rect) {
        let buttons_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(33),
                Constraint::Percentage(33),
                Constraint::Percentage(34),
            ])
            .split(area);

        // Save button
        let save_style = if self.focus == ColumnsFocus::SaveButton {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Yellow)
        };

        let save_button = Paragraph::new("Save as Default")
            .style(save_style)
            .block(Block::default().borders(Borders::ALL));

        frame.render_widget(save_button, buttons_layout[0]);

        // Apply button
        let apply_style = if self.focus == ColumnsFocus::ApplyButton {
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Green)
        };

        let apply_button = Paragraph::new("Apply")
            .style(apply_style)
            .block(Block::default().borders(Borders::ALL));

        frame.render_widget(apply_button, buttons_layout[1]);

        // Cancel button
        let cancel_style = if self.focus == ColumnsFocus::CancelButton {
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Red)
        };

        let cancel_button = Paragraph::new("Cancel")
            .style(cancel_style)
            .block(Block::default().borders(Borders::ALL));

        frame.render_widget(cancel_button, buttons_layout[2]);
    }

    /// Handle key events
    pub fn handle_key(&mut self, key: crossterm::event::KeyEvent) -> ColumnsAction {
        use crossterm::event::KeyCode;

        // Handle global keys first
        match key.code {
            KeyCode::Esc => return ColumnsAction::Close,
            KeyCode::Tab => {
                self.cycle_focus();
                return ColumnsAction::None;
            }
            // Handle tab switching with dedicated keys (Tab key won't work in TUI)
            KeyCode::Char('t') => {
                // Toggle between tabs
                self.tab_index = (self.tab_index + 1) % 2;
                self.update_focus_for_tab();
                return ColumnsAction::None;
            }
            KeyCode::Left | KeyCode::Right => {
                if (self.focus == ColumnsFocus::AvailableColumns
                    || self.focus == ColumnsFocus::SelectedColumns)
                    && self.tab_index == 0
                {
                    // Switch between available and selected columns
                    if self.focus == ColumnsFocus::AvailableColumns && key.code == KeyCode::Right {
                        self.focus = ColumnsFocus::SelectedColumns;
                    } else if self.focus == ColumnsFocus::SelectedColumns
                        && key.code == KeyCode::Left
                    {
                        self.focus = ColumnsFocus::AvailableColumns;
                    }
                    return ColumnsAction::None;
                }
            }
            // F10 or Ctrl+S to save settings
            KeyCode::F(10) => return ColumnsAction::SaveAndApply,
            KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                return ColumnsAction::SaveAndApply;
            }
            // Number keys to switch tabs directly
            KeyCode::Char('1') => {
                self.tab_index = 0;
                self.update_focus_for_tab();
                return ColumnsAction::None;
            }
            KeyCode::Char('2') => {
                self.tab_index = 1;
                self.update_focus_for_tab();
                return ColumnsAction::None;
            }
            _ => {}
        }

        // Handle tab-specific actions
        match self.tab_index {
            0 => self.handle_columns_tab_key(key),
            1 => self.handle_sort_tab_key(key),
            _ => ColumnsAction::None,
        }
    }

    /// Handle keys in the columns tab
    fn handle_columns_tab_key(&mut self, key: crossterm::event::KeyEvent) -> ColumnsAction {
        use crossterm::event::KeyCode;

        match (self.focus, key.code) {
            // Navigate available columns
            (ColumnsFocus::AvailableColumns, KeyCode::Up) => {
                if let Some(selected) = self.available_columns_state.selected() {
                    if selected > 0 {
                        self.available_columns_state.select(Some(selected - 1));
                    }
                }
                ColumnsAction::None
            }
            (ColumnsFocus::AvailableColumns, KeyCode::Down) => {
                if let Some(selected) = self.available_columns_state.selected() {
                    if selected < self.available_columns.len().saturating_sub(1) {
                        self.available_columns_state.select(Some(selected + 1));
                    }
                }
                ColumnsAction::None
            }
            // Add column to selected
            (ColumnsFocus::AvailableColumns, KeyCode::Enter) => {
                if let Some(selected) = self.available_columns_state.selected() {
                    if selected < self.available_columns.len() {
                        let column = self.available_columns.remove(selected);
                        self.selected_columns.push(column);

                        // Adjust selection if needed
                        if self.available_columns.is_empty() {
                            self.available_columns_state.select(None);
                        } else if selected >= self.available_columns.len() {
                            self.available_columns_state
                                .select(Some(self.available_columns.len() - 1));
                        }

                        // Select the newly added column in the selected list
                        self.selected_columns_state
                            .select(Some(self.selected_columns.len() - 1));
                    }
                }
                ColumnsAction::None
            }

            // Navigate selected columns
            (ColumnsFocus::SelectedColumns, KeyCode::Up) => {
                if let Some(selected) = self.selected_columns_state.selected() {
                    if selected > 0 {
                        self.selected_columns_state.select(Some(selected - 1));
                    }
                }
                ColumnsAction::None
            }
            (ColumnsFocus::SelectedColumns, KeyCode::Down) => {
                if let Some(selected) = self.selected_columns_state.selected() {
                    if selected < self.selected_columns.len().saturating_sub(1) {
                        self.selected_columns_state.select(Some(selected + 1));
                    }
                }
                ColumnsAction::None
            }
            // Remove column from selected
            (ColumnsFocus::SelectedColumns, KeyCode::Enter) => {
                if let Some(selected) = self.selected_columns_state.selected() {
                    if !self.selected_columns.is_empty() && selected < self.selected_columns.len() {
                        let column = self.selected_columns.remove(selected);
                        self.available_columns.push(column);

                        // Adjust selection if needed
                        if self.selected_columns.is_empty() {
                            self.selected_columns_state.select(None);
                        } else if selected >= self.selected_columns.len() {
                            self.selected_columns_state
                                .select(Some(self.selected_columns.len() - 1));
                        }

                        // Select the newly added column in the available list
                        self.available_columns_state
                            .select(Some(self.available_columns.len() - 1));
                    }
                }
                ColumnsAction::None
            }
            // Move selected column up/down
            (ColumnsFocus::SelectedColumns, KeyCode::Char(' ')) => {
                if let Some(selected) = self.selected_columns_state.selected() {
                    if key.modifiers.contains(KeyModifiers::SHIFT) {
                        // Move up
                        if selected > 0 {
                            self.selected_columns.swap(selected, selected - 1);
                            self.selected_columns_state.select(Some(selected - 1));
                        }
                    } else {
                        // Move down
                        if selected < self.selected_columns.len().saturating_sub(1) {
                            self.selected_columns.swap(selected, selected + 1);
                            self.selected_columns_state.select(Some(selected + 1));
                        }
                    }
                }
                ColumnsAction::None
            }

            // Handle button actions
            (ColumnsFocus::SaveButton, KeyCode::Enter) => ColumnsAction::SaveAndApply,
            (ColumnsFocus::ApplyButton, KeyCode::Enter) => ColumnsAction::Apply,
            (ColumnsFocus::CancelButton, KeyCode::Enter) => ColumnsAction::Close,

            _ => ColumnsAction::None,
        }
    }

    /// Handle keys in the sort tab
    fn handle_sort_tab_key(&mut self, key: crossterm::event::KeyEvent) -> ColumnsAction {
        use crossterm::event::KeyCode;

        match (self.focus, key.code) {
            // Navigate sort columns
            (ColumnsFocus::SortColumns, KeyCode::Up) => {
                if let Some(selected) = self.sort_columns_state.selected() {
                    if selected > 0 {
                        self.sort_columns_state.select(Some(selected - 1));
                    }
                }
                ColumnsAction::None
            }
            (ColumnsFocus::SortColumns, KeyCode::Down) => {
                if let Some(selected) = self.sort_columns_state.selected() {
                    if selected < self.sort_columns.len().saturating_sub(1) {
                        self.sort_columns_state.select(Some(selected + 1));
                    }
                }
                ColumnsAction::None
            }
            // Toggle sort order
            (ColumnsFocus::SortColumns, KeyCode::Char(' ')) => {
                if let Some(selected) = self.sort_columns_state.selected() {
                    if selected < self.sort_columns.len() {
                        self.sort_columns[selected].order =
                            self.sort_columns[selected].order.toggle();
                    }
                }
                ColumnsAction::None
            }
            // Remove sort column
            (ColumnsFocus::SortColumns, KeyCode::Delete | KeyCode::Backspace) => {
                if let Some(selected) = self.sort_columns_state.selected() {
                    if !self.sort_columns.is_empty() && selected < self.sort_columns.len() {
                        self.sort_columns.remove(selected);

                        // Adjust selection if needed
                        if self.sort_columns.is_empty() {
                            self.sort_columns_state.select(None);
                        } else if selected >= self.sort_columns.len() {
                            self.sort_columns_state
                                .select(Some(self.sort_columns.len() - 1));
                        }
                    }
                }
                ColumnsAction::None
            }
            // Add column to sort
            (ColumnsFocus::SortColumns, KeyCode::Enter) => {
                // Show a submenu to select which column to add for sorting
                // For now, we'll just add the first selected column if it's not already in sort
                if !self.selected_columns.is_empty() {
                    if let Some(selected) = self.selected_columns_state.selected() {
                        if selected < self.selected_columns.len() {
                            let column = self.selected_columns[selected];

                            // Check if the column is already in sort_columns
                            if !self.sort_columns.iter().any(|sc| sc.column == column) {
                                self.sort_columns.push(SortColumn {
                                    column,
                                    order: SortOrder::Ascending,
                                });

                                // Select the newly added sort column
                                self.sort_columns_state
                                    .select(Some(self.sort_columns.len() - 1));
                            }
                        }
                    }
                }
                ColumnsAction::None
            }

            // Handle button actions
            (ColumnsFocus::SaveButton, KeyCode::Enter) => ColumnsAction::SaveAndApply,
            (ColumnsFocus::ApplyButton, KeyCode::Enter) => ColumnsAction::Apply,
            (ColumnsFocus::CancelButton, KeyCode::Enter) => ColumnsAction::Close,

            _ => ColumnsAction::None,
        }
    }

    /// Cycle through focusable elements
    fn cycle_focus(&mut self) {
        match self.focus {
            ColumnsFocus::AvailableColumns => self.focus = ColumnsFocus::SelectedColumns,
            ColumnsFocus::SelectedColumns => self.focus = ColumnsFocus::SortColumns,
            ColumnsFocus::SortColumns => self.focus = ColumnsFocus::SaveButton,
            ColumnsFocus::SaveButton => self.focus = ColumnsFocus::ApplyButton,
            ColumnsFocus::ApplyButton => self.focus = ColumnsFocus::CancelButton,
            ColumnsFocus::CancelButton => self.focus = ColumnsFocus::AvailableColumns,
        }

        self.update_focus_for_tab();
    }

    /// Update focus to match the current tab
    fn update_focus_for_tab(&mut self) {
        match self.tab_index {
            0 => {
                // In the columns tab, only available/selected columns or buttons are valid
                match self.focus {
                    ColumnsFocus::SortColumns => self.focus = ColumnsFocus::SelectedColumns,
                    _ => {}
                }
            }
            1 => {
                // In the sort tab, only sort columns or buttons are valid
                match self.focus {
                    ColumnsFocus::AvailableColumns | ColumnsFocus::SelectedColumns => {
                        self.focus = ColumnsFocus::SortColumns
                    }
                    _ => {}
                }
            }
            _ => {}
        }

        // Ensure we have a valid selection in list states based on current focus
        if self.focus == ColumnsFocus::AvailableColumns
            && self.available_columns.len() > 0
            && self.available_columns_state.selected().is_none()
        {
            self.available_columns_state.select(Some(0));
        } else if self.focus == ColumnsFocus::SelectedColumns
            && self.selected_columns.len() > 0
            && self.selected_columns_state.selected().is_none()
        {
            self.selected_columns_state.select(Some(0));
        } else if self.focus == ColumnsFocus::SortColumns
            && self.sort_columns.len() > 0
            && self.sort_columns_state.selected().is_none()
        {
            self.sort_columns_state.select(Some(0));
        }
    }

    /// Get the squeue format string for the selected columns
    pub fn get_format_string(&self) -> String {
        if self.selected_columns.is_empty() {
            // Provide a default format if none selected
            return "%A|%j|%u|%T|%P|%q|%D|%C|%M".to_string();
        }

        self.selected_columns
            .iter()
            .map(|col| col.format_code())
            .collect::<Vec<_>>()
            .join("|")
    }

    /// Get the squeue sort string for the sort columns
    pub fn get_sort_string(&self) -> Option<String> {
        if self.sort_columns.is_empty() {
            // Default sort by job ID if none specified
            return Some("A".to_string());
        }

        Some(
            self.sort_columns
                .iter()
                .map(|sort_col| {
                    let prefix = match sort_col.order {
                        SortOrder::Ascending => "",
                        SortOrder::Descending => "-",
                    };
                    // Extract the format code without the % and ensure it's valid
                    let code = sort_col.column.format_code().trim_start_matches('%');
                    format!("{}{}", prefix, code)
                })
                .collect::<Vec<_>>()
                .join(","),
        )
    }

    /// Get the column constraints for the table
    pub fn get_column_constraints(&self) -> Vec<Constraint> {
        if self.selected_columns.is_empty() {
            // Return default constraints if no columns selected
            return vec![
                Constraint::Length(8),      // ID
                Constraint::Percentage(20), // Name
                Constraint::Length(10),     // User
                Constraint::Length(10),     // State
                Constraint::Length(10),     // Partition
                Constraint::Length(8),      // QoS
                Constraint::Length(6),      // Nodes
                Constraint::Length(6),      // CPUs
                Constraint::Length(10),     // Time
            ];
        }

        self.selected_columns
            .iter()
            .map(|col| col.default_width())
            .collect()
    }
}

/// Action to take after handling a key in the columns popup
pub enum ColumnsAction {
    /// Do nothing
    None,
    /// Close the columns popup
    Close,
    /// Apply changes and close
    Apply,
    /// Save changes as default and apply
    SaveAndApply,
}
