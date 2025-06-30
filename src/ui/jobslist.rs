use color_eyre::Result;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState},
};

use crate::slurm::{Job, JobState};
use crate::ui::columns::{JobColumn, SortColumn};

/// Struct to manage the jobs list view
pub struct JobsList {
    pub state: TableState,
    pub jobs: Vec<Job>,
    pub selected_jobs: Vec<usize>,
    pub sort_column: usize,
    pub sort_ascending: bool,
}

impl JobsList {
    pub fn new() -> Self {
        Self {
            state: TableState::default(),
            jobs: Vec::new(),
            selected_jobs: Vec::new(),
            sort_column: 0, // Default sort by job ID
            sort_ascending: true,
        }
    }

    /// Update the list of jobs
    pub fn update_jobs(&mut self, jobs: Vec<Job>) {
        self.jobs = jobs;
        self.sort_jobs();

        // Reset selection if out of bounds
        if let Some(selected) = self.state.selected() {
            if selected >= self.jobs.len() {
                self.state.select(Some(0));
            }
        } else if !self.jobs.is_empty() {
            self.state.select(Some(0));
        }
    }

    /// Sort jobs based on current sort column and direction
    pub fn sort_jobs(&mut self) {
        // Safety check for column index
        if self.sort_column >= 16 {
            return;
        }

        // Define a sort key function to handle different column types consistently
        let sort_key = |job: &Job, column: usize| -> String {
            match column {
                0 => job.id.clone(),
                1 => job.name.clone(),
                2 => job.user.clone(),
                3 => format!("{:?}", job.state),
                4 => job.partition.clone(),
                5 => job.qos.clone(),
                6 => job.nodes.to_string(),
                7 => job.cpus.to_string(),
                8 => job.time.clone(),
                9 => job.memory.clone(),
                // Default to empty string for any other columns
                _ => String::new(),
            }
        };

        // Special case for numeric ID sorting
        if self.sort_column == 0 {
            self.jobs.sort_by(|a, b| {
                let a_id = a.id.parse::<u32>().unwrap_or(0);
                let b_id = b.id.parse::<u32>().unwrap_or(0);
                if self.sort_ascending {
                    a_id.cmp(&b_id)
                } else {
                    b_id.cmp(&a_id)
                }
            });
        } else {
            // Sort by the selected column using the sort_key function
            self.jobs.sort_by(|a, b| {
                let key_a = sort_key(a, self.sort_column);
                let key_b = sort_key(b, self.sort_column);

                if self.sort_ascending {
                    key_a.cmp(&key_b)
                } else {
                    key_b.cmp(&key_a)
                }
            });
        }
    }

    /// Toggle job selection
    pub fn toggle_select(&mut self) {
        if let Some(selected) = self.state.selected() {
            if self.selected_jobs.contains(&selected) {
                self.selected_jobs.retain(|&i| i != selected);
            } else {
                self.selected_jobs.push(selected);
            }
        }
    }

    /// Select all jobs
    pub fn select_all(&mut self) {
        self.selected_jobs = (0..self.jobs.len()).collect();
    }

    /// Clear all selections
    pub fn clear_selection(&mut self) {
        self.selected_jobs.clear();
    }

    /// Change sort column
    pub fn sort_by(&mut self, column: usize) {
        if self.sort_column == column {
            // Toggle sort direction if already sorting by this column
            self.sort_ascending = !self.sort_ascending;
        } else {
            // Change to new sort column with default ascending order
            self.sort_column = column;
            self.sort_ascending = true;
        }
        self.sort_jobs();
    }

    /// Update sort configuration based on SortColumn settings
    pub fn update_sort(&mut self, columns: &[JobColumn], sort_columns: &[SortColumn]) {
        if let Some(first_sort) = sort_columns.first() {
            // Find the index of the column in the displayed columns list
            let column_index = columns
                .iter()
                .position(|col| {
                    std::mem::discriminant(col) == std::mem::discriminant(&first_sort.column)
                })
                .unwrap_or(0);

            self.sort_column = column_index;
            self.sort_ascending =
                matches!(first_sort.order, crate::ui::columns::SortOrder::Ascending);
            self.sort_jobs();
        }
    }

    /// Navigate to next job
    pub fn next(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.jobs.len().saturating_sub(1) {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    /// Navigate to previous job
    pub fn previous(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.jobs.len().saturating_sub(1)
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    /// Draw the jobs list widget
    pub fn render(
        &mut self,
        frame: &mut Frame,
        area: Rect,
        columns: &[JobColumn],
        sort_columns: &[SortColumn],
    ) {
        // Update sorting if needed based on sort_columns
        if !sort_columns.is_empty() {
            self.update_sort(columns, sort_columns);
        }

        // Check if columns are empty, show warning if so
        if columns.is_empty() {
            let warning = Paragraph::new("No columns selected. Press 'c' to configure columns.")
                .style(Style::default().fg(Color::Yellow))
                .block(Block::default().title("Warning").borders(Borders::ALL));
            frame.render_widget(warning, area);
            return;
        }

        // Create headers based on selected columns
        let headers: Vec<&str> = columns.iter().map(|col| col.title()).collect();

        // Create header cells with appropriate styling
        let header_cells = headers.iter().enumerate().map(|(i, &h)| {
            // Check if this column is in the sort list
            let is_sort_column = sort_columns.iter().any(|sc| sc.column.title() == h);
            let sort_indicator = if is_sort_column {
                let sort_col = sort_columns
                    .iter()
                    .find(|sc| sc.column.title() == h)
                    .unwrap();
                match sort_col.order {
                    crate::ui::columns::SortOrder::Ascending => " ↑",
                    crate::ui::columns::SortOrder::Descending => " ↓",
                }
            } else {
                ""
            };

            let header_style = if is_sort_column {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD)
            };

            Cell::from(format!("{}{}", h, sort_indicator)).style(header_style)
        });

        let header = Row::new(header_cells)
            .style(Style::default().bg(Color::DarkGray))
            .height(1);

        // Create rows for each job
        let rows = self.jobs.iter().enumerate().map(|(i, job)| {
            let is_selected = self.selected_jobs.contains(&i);
            let color = match job.state {
                JobState::Pending => Color::Yellow,
                JobState::Running => Color::Green,
                JobState::Completed => Color::Blue,
                JobState::Failed | JobState::Timeout | JobState::NodeFail | JobState::Boot => {
                    Color::Red
                }
                JobState::Cancelled => Color::Magenta,
                _ => Color::White,
            };

            let style = if is_selected {
                Style::default().fg(color).add_modifier(Modifier::REVERSED)
            } else {
                Style::default().fg(color)
            };

            // Create cells based on selected columns
            let cells: Vec<Cell> = columns
                .iter()
                .map(|col| {
                    let content = match col {
                        JobColumn::Id => job.id.clone(),
                        JobColumn::Name => {
                            // Truncate name if too long
                            if job.name.len() > 30 {
                                format!("{}...", &job.name[0..27])
                            } else {
                                job.name.clone()
                            }
                        }
                        JobColumn::User => job.user.clone(),
                        JobColumn::State => job.state.to_string(),
                        JobColumn::Partition => job.partition.clone(),
                        JobColumn::QoS => job.qos.clone(),
                        JobColumn::Nodes => job.nodes.to_string(),
                        JobColumn::CPUs => job.cpus.to_string(),
                        JobColumn::Time => job.time.clone(),
                        JobColumn::Memory => job.memory.clone(),
                        JobColumn::Account => "-".to_string(), // Placeholder with better format
                        JobColumn::Priority => "-".to_string(), // Placeholder with better format
                        JobColumn::WorkDir => "-".to_string(), // Placeholder with better format
                        JobColumn::SubmitTime => "-".to_string(), // Placeholder with better format
                        JobColumn::StartTime => "-".to_string(), // Placeholder with better format
                        JobColumn::EndTime => "-".to_string(), // Placeholder with better format
                    };
                    Cell::from(content)
                })
                .collect();

            Row::new(cells).style(style).height(1)
        });

        // Calculate total available width
        let available_width = area.width.saturating_sub(2); // Subtract 2 for borders

        // Get constraints for columns with improved layout
        let constraints: Vec<Constraint> = columns
            .iter()
            .map(|col| match col {
                JobColumn::Id => Constraint::Length(10),
                JobColumn::Name => Constraint::Min(15),
                JobColumn::User => Constraint::Length(10),
                JobColumn::State => Constraint::Length(12),
                JobColumn::Partition => Constraint::Length(12),
                JobColumn::QoS => Constraint::Length(10),
                JobColumn::Nodes => Constraint::Length(7),
                JobColumn::CPUs => Constraint::Length(6),
                JobColumn::Time => Constraint::Length(12),
                JobColumn::Memory => Constraint::Length(10),
                JobColumn::Account => Constraint::Length(12),
                JobColumn::Priority => Constraint::Length(10),
                JobColumn::WorkDir => Constraint::Min(20),
                JobColumn::SubmitTime | JobColumn::StartTime | JobColumn::EndTime => {
                    Constraint::Length(19)
                }
            })
            .collect();

        // Create the table
        let table = Table::new(rows, constraints)
            .header(header)
            .block(Block::default().borders(Borders::ALL).title("Jobs"))
            .highlight_style(Style::default().add_modifier(Modifier::BOLD))
            .highlight_symbol(" > ");

        // Render the table
        frame.render_stateful_widget(table, area, &mut self.state);
    }

    /// Get the currently selected job, if any
    pub fn selected_job(&self) -> Option<&Job> {
        self.state.selected().and_then(|i| self.jobs.get(i))
    }

    /// Get all selected jobs
    pub fn get_selected_jobs(&self) -> Vec<&Job> {
        self.selected_jobs
            .iter()
            .filter_map(|&i| self.jobs.get(i))
            .collect()
    }
}
