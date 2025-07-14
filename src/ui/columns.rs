use crossterm::event::KeyModifiers;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
};

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
    Node,
    CPUs,
    Time,
    Memory,
    Account,
    Priority,
    WorkDir,
    SubmitTime,
    StartTime,
    EndTime,
    PReason,
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
            JobColumn::Node => "Node",
            JobColumn::CPUs => "CPUs",
            JobColumn::Time => "Time",
            JobColumn::Memory => "Memory",
            JobColumn::Account => "Account",
            JobColumn::Priority => "Priority",
            JobColumn::WorkDir => "WorkDir",
            JobColumn::SubmitTime => "Submit",
            JobColumn::StartTime => "Start",
            JobColumn::EndTime => "End",
            JobColumn::PReason => "Reason", // Pending reason
        }
    }

    /// Get the format code for this column
    pub fn format_code(&self) -> &'static str {
        match self {
            JobColumn::Id => "%i",         // Job ID - using %A for array job ID
            JobColumn::Name => "%j",       // Job name
            JobColumn::User => "%u",       // User name
            JobColumn::State => "%T",      // Job state
            JobColumn::Partition => "%P",  // Partition
            JobColumn::QoS => "%q",        // Quality of Service
            JobColumn::Nodes => "%D",      // Node count
            JobColumn::Node => "%N",       // Node list
            JobColumn::CPUs => "%C",       // CPU count
            JobColumn::Time => "%M",       // Time limit
            JobColumn::Memory => "%m",     // Memory
            JobColumn::Account => "%a",    // Account
            JobColumn::Priority => "%Q",   // Priority
            JobColumn::WorkDir => "%Z",    // Working directory
            JobColumn::SubmitTime => "%V", // Submission time
            JobColumn::StartTime => "%S",  // Start time
            JobColumn::EndTime => "%e",    // End time
            JobColumn::PReason => "%R",    // Pending reason
        }
    }

    /// Get the default width constraint for this column
    pub fn default_width(&self) -> Constraint {
        match self {
            JobColumn::Id => Constraint::Length(10),
            JobColumn::Name => Constraint::Percentage(20),
            JobColumn::User => Constraint::Length(10),
            JobColumn::State => Constraint::Length(12),
            JobColumn::Partition => Constraint::Length(12),
            JobColumn::QoS => Constraint::Length(10),
            JobColumn::Nodes => Constraint::Length(7),
            JobColumn::Node => Constraint::Percentage(12), // Node list can be long
            JobColumn::CPUs => Constraint::Length(6),
            JobColumn::Time => Constraint::Length(12),
            JobColumn::Memory => Constraint::Length(10),
            JobColumn::Account => Constraint::Length(12),
            JobColumn::Priority => Constraint::Length(10),
            JobColumn::WorkDir => Constraint::Percentage(15),
            JobColumn::SubmitTime => Constraint::Length(19),
            JobColumn::StartTime => Constraint::Length(19),
            JobColumn::EndTime => Constraint::Length(19),
            JobColumn::PReason => Constraint::Percentage(20), // Pending reason can be long
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
            JobColumn::Node,
            JobColumn::CPUs,
            JobColumn::Time,
            JobColumn::Memory,
            JobColumn::Account,
            JobColumn::Priority,
            JobColumn::WorkDir,
            JobColumn::SubmitTime,
            JobColumn::StartTime,
            JobColumn::EndTime,
            JobColumn::PReason,
        ]
    }

    /// Default columns to display
    pub fn defaults() -> Vec<JobColumn> {
        // These MUST match the defaults in App::new()
        // "%i|%j|%u|%T|%M|%N|%C|%m|%P|%q".to_string(), // JobID|Name|User|State|Time|Nodes|CPUs|Memory|Partition|QOS
        vec![
            JobColumn::Id,
            JobColumn::Name,
            JobColumn::User,
            JobColumn::State,
            JobColumn::Time,
            JobColumn::Node,
            JobColumn::CPUs,
            JobColumn::Memory,
            JobColumn::Partition,
            JobColumn::QoS,
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
    /// If show
    pub visible: bool,
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
            focus: ColumnsFocus::SelectedColumns,
            available_columns_state,
            selected_columns_state,
            sort_columns_state,
            available_columns,
            selected_columns,
            sort_columns,
            visible: false,
        }
    }

    /// Render the columns popup
    pub fn render(&mut self, frame: &mut Frame, area: Rect) {
        // Render Clear first
        frame.render_widget(Clear, area);
        // Create a block for the popup
        let block = Block::default()
            .title(Line::from("Column Management").centered())
            .borders(Borders::NONE)
            .style(Style::default().bg(Color::Black));

        frame.render_widget(block.clone(), area);

        // Create the inner area for the content
        let inner_area = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([
                Constraint::Min(5),    // Content
                Constraint::Length(3), // Help text
            ])
            .split(area);

        // Render the unified three-column view
        self.render_unified_columns_view(frame, inner_area[0]);

        // Render help text
        self.render_help_text(frame, inner_area[1]);
    }

    /// Render the columns tab (available and selected columns)
    fn render_unified_columns_view(&mut self, frame: &mut Frame, area: Rect) {
        // Split the area into three columns
        let columns = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(33),
                Constraint::Percentage(33),
                Constraint::Percentage(34),
            ])
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

        frame.render_stateful_widget(sort_list, columns[2], &mut self.sort_columns_state);
    }

    fn render_help_text(&self, frame: &mut Frame, area: Rect) {
        let base_help_text = match self.focus {
            ColumnsFocus::AvailableColumns => {
                "↑/↓: Navigate | ←/→: Switch lists | Enter: Add to Selected"
            }
            ColumnsFocus::SelectedColumns => {
                "↑/↓: Navigate | ←/→: Switch lists | Enter: Add to Sort | Del: Remove | Ctrl+↑/↓: Move up/down"
            }
            ColumnsFocus::SortColumns => {
                "↑/↓: Navigate | ←/→: Switch lists | Enter: Toggle order | Del: Remove | Ctrl+↑/↓: Move up/down"
            }
            _ => "",
        };

        let full_help_text = format!("{} | Ctrl+a: Apply | Esc: Close", base_help_text);

        let help = Paragraph::new(full_help_text)
            .style(Style::default().fg(Color::Gray))
            .block(Block::default().borders(Borders::ALL));

        frame.render_widget(help, area);
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
            KeyCode::Left => {
                // Navigate left between columns
                match self.focus {
                    ColumnsFocus::SelectedColumns => {
                        self.focus = ColumnsFocus::AvailableColumns;
                        self.update_selections();
                        return ColumnsAction::None;
                    }
                    ColumnsFocus::SortColumns => {
                        self.focus = ColumnsFocus::SelectedColumns;
                        self.update_selections();
                        return ColumnsAction::None;
                    }
                    _ => {}
                }
            }
            KeyCode::Right => {
                // Navigate right between columns
                match self.focus {
                    ColumnsFocus::AvailableColumns => {
                        self.focus = ColumnsFocus::SelectedColumns;
                        self.update_selections();
                        return ColumnsAction::None;
                    }
                    ColumnsFocus::SelectedColumns => {
                        self.focus = ColumnsFocus::SortColumns;
                        self.update_selections();
                        return ColumnsAction::None;
                    }
                    _ => {}
                }
            }

            // Ctrl+A to apply changes
            KeyCode::Char('a') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                return ColumnsAction::Apply;
            }

            // F10 or Ctrl+S to save settings
            // KeyCode::F(10) => return ColumnsAction::SaveAndApply,
            // KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            //     return ColumnsAction::SaveAndApply;
            // }
            _ => {}
        }

        // Handle actions based on which column has focus
        match self.focus {
            ColumnsFocus::AvailableColumns => self.handle_available_columns_key(key),
            ColumnsFocus::SelectedColumns => self.handle_selected_columns_key(key),
            ColumnsFocus::SortColumns => self.handle_sort_columns_key(key),
            _ => self.handle_button_key(key),
        }
    }

    /// Handle keys for available columns
    fn handle_available_columns_key(&mut self, key: crossterm::event::KeyEvent) -> ColumnsAction {
        use crossterm::event::KeyCode;

        match key.code {
            // Navigate available columns
            KeyCode::Up => {
                if let Some(selected) = self.available_columns_state.selected() {
                    if selected > 0 {
                        self.available_columns_state.select(Some(selected - 1));
                    }
                }
                ColumnsAction::None
            }
            KeyCode::Down => {
                if let Some(selected) = self.available_columns_state.selected() {
                    if selected < self.available_columns.len().saturating_sub(1) {
                        self.available_columns_state.select(Some(selected + 1));
                    }
                }
                ColumnsAction::None
            }
            // Add column to selected
            KeyCode::Enter => {
                if let Some(selected) = self.available_columns_state.selected() {
                    if !self.available_columns.is_empty() && selected < self.available_columns.len()
                    {
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
            _ => ColumnsAction::None,
        }
    }

    /// Handle keys for selected columns
    fn handle_selected_columns_key(&mut self, key: crossterm::event::KeyEvent) -> ColumnsAction {
        use crossterm::event::KeyCode;
        use crossterm::event::KeyModifiers;

        match key.code {
            // Navigate selected columns
            KeyCode::Up => {
                if let Some(selected) = self.selected_columns_state.selected() {
                    if selected > 0 {
                        if key.modifiers.contains(KeyModifiers::SHIFT) {
                            // Move column up
                            self.selected_columns.swap(selected, selected - 1);
                        }
                        self.selected_columns_state.select(Some(selected - 1));
                    }
                }
                ColumnsAction::None
            }
            KeyCode::Down => {
                if let Some(selected) = self.selected_columns_state.selected() {
                    if selected < self.selected_columns.len().saturating_sub(1) {
                        if key.modifiers.contains(KeyModifiers::SHIFT) {
                            // Move column down
                            self.selected_columns.swap(selected, selected + 1);
                        }
                        self.selected_columns_state.select(Some(selected + 1));
                    }
                }
                ColumnsAction::None
            }
            // Add column to sort
            KeyCode::Enter => {
                if let Some(selected) = self.selected_columns_state.selected() {
                    if !self.selected_columns.is_empty() && selected < self.selected_columns.len() {
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
                ColumnsAction::None
            }
            // Remove column from selected
            KeyCode::Delete | KeyCode::Backspace => {
                if let Some(selected) = self.selected_columns_state.selected() {
                    if !self.selected_columns.is_empty() && selected < self.selected_columns.len() {
                        let column = self.selected_columns.remove(selected);
                        self.available_columns.push(column);

                        // Also remove from sort if present
                        self.sort_columns.retain(|sc| sc.column != column);

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

            _ => ColumnsAction::None,
        }
    }

    /// Handle keys for sort columns
    fn handle_sort_columns_key(&mut self, key: crossterm::event::KeyEvent) -> ColumnsAction {
        use crossterm::event::KeyCode;
        use crossterm::event::KeyModifiers;

        match key.code {
            // Navigate sort columns
            KeyCode::Up => {
                if let Some(selected) = self.sort_columns_state.selected() {
                    if selected > 0 {
                        if key.modifiers.contains(KeyModifiers::SHIFT) {
                            // Move column up
                            self.sort_columns.swap(selected, selected - 1);
                        }
                        self.sort_columns_state.select(Some(selected - 1));
                    }
                }
                ColumnsAction::None
            }
            KeyCode::Down => {
                if let Some(selected) = self.sort_columns_state.selected() {
                    if selected < self.sort_columns.len().saturating_sub(1) {
                        if key.modifiers.contains(KeyModifiers::SHIFT) {
                            // Move column down
                            self.sort_columns.swap(selected, selected + 1);
                        }
                        self.sort_columns_state.select(Some(selected + 1));
                    }
                }
                ColumnsAction::None
            }
            // Toggle sort order
            KeyCode::Char(' ') | KeyCode::Enter => {
                if let Some(selected) = self.sort_columns_state.selected() {
                    if selected < self.sort_columns.len() {
                        self.sort_columns[selected].order =
                            self.sort_columns[selected].order.toggle();
                    }
                }
                ColumnsAction::None
            }
            // Remove sort column
            KeyCode::Delete | KeyCode::Backspace => {
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
            _ => ColumnsAction::None,
        }
    }

    /// Handle button key events
    fn handle_button_key(&mut self, key: crossterm::event::KeyEvent) -> ColumnsAction {
        use crossterm::event::KeyCode;

        match (self.focus, key.code) {
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

        self.update_selections();
    }

    /// Update selections when focus changes
    fn update_selections(&mut self) {
        // Update list selections based on focus
        if self.focus == ColumnsFocus::AvailableColumns
            && !self.available_columns.is_empty()
            && self.available_columns_state.selected().is_none()
        {
            self.available_columns_state.select(Some(0));
        } else if self.focus == ColumnsFocus::SelectedColumns
            && !self.selected_columns.is_empty()
            && self.selected_columns_state.selected().is_none()
        {
            self.selected_columns_state.select(Some(0));
        } else if self.focus == ColumnsFocus::SortColumns
            && !self.sort_columns.is_empty()
            && self.sort_columns_state.selected().is_none()
        {
            self.sort_columns_state.select(Some(0));
        }
    }

    // /// Get the column constraints for the table
    // pub fn get_column_constraints(&self) -> Vec<Constraint> {
    //     if self.selected_columns.is_empty() {
    //         // Return default constraints if no columns selected
    //         return JobColumn::defaults()
    //             .iter()
    //             .map(|col| col.default_width())
    //             .collect();
    //     }

    //     self.selected_columns
    //         .iter()
    //         .map(|col| col.default_width())
    //         .collect()
    // }
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
