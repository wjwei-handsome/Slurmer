use ratatui::{
    Frame,
    layout::{Constraint, Rect},
    style::{Color, Modifier, Style},
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
        // Jobs are already sorted by the squeue command

        // Reset selection if out of bounds
        if let Some(selected) = self.state.selected() {
            if selected >= self.jobs.len() {
                self.state.select(Some(0));
            }
        } else if !self.jobs.is_empty() {
            self.state.select(Some(0));
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

    /// Judge if all jobs are selected
    pub fn all_selected(&self) -> bool {
        self.selected_jobs.len() == self.jobs.len()
    }

    /// Select all jobs
    pub fn select_all(&mut self) {
        self.selected_jobs = (0..self.jobs.len()).collect();
    }

    /// Clear all selections
    pub fn clear_selection(&mut self) {
        self.selected_jobs.clear();
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
            // No need to sort jobs as sorting is handled by squeue
        }
    }

    /// Navigate to next job
    /// Returns true if selection changed, false otherwise
    pub fn next(&mut self) -> bool {
        if self.jobs.is_empty() {
            return false;
        }

        let old_selection = self.state.selected();
        let i = match old_selection {
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
        old_selection != Some(i)
    }

    /// Navigate to previous job
    /// Returns true if selection changed, false otherwise
    pub fn previous(&mut self) -> bool {
        if self.jobs.is_empty() {
            return false;
        }

        let old_selection = self.state.selected();
        let i = match old_selection {
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
        old_selection != Some(i)
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
        let header_cells = headers.iter().enumerate().map(|(_i, &h)| {
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
                        JobColumn::Node => job.node.clone().unwrap_or_else(|| "-".to_string()),
                        JobColumn::CPUs => job.cpus.to_string(),
                        JobColumn::Time => job.time.clone(),
                        JobColumn::Memory => job.memory.clone(),
                        JobColumn::Account => {
                            job.account.clone().unwrap_or_else(|| "-".to_string())
                        }
                        JobColumn::Priority => job
                            .priority
                            .map(|p| p.to_string())
                            .unwrap_or_else(|| "-".to_string()),
                        JobColumn::WorkDir => {
                            job.work_dir.clone().unwrap_or_else(|| "-".to_string())
                        }
                        JobColumn::SubmitTime => {
                            job.submit_time.clone().unwrap_or_else(|| "-".to_string())
                        }
                        JobColumn::StartTime => {
                            job.start_time.clone().unwrap_or_else(|| "-".to_string())
                        }
                        JobColumn::EndTime => {
                            job.end_time.clone().unwrap_or_else(|| "-".to_string())
                        }
                        JobColumn::PReason => job
                            .pending_reason
                            .clone()
                            .unwrap_or_else(|| "-".to_string()),
                    };
                    Cell::from(content)
                })
                .collect();

            Row::new(cells).style(style).height(1)
        });

        // Calculate total available width
        // let available_width = area.width.saturating_sub(2); // Subtract 2 for borders

        // Get constraints for columns using the default_width method from JobColumn
        let constraints: Vec<Constraint> = columns
            .iter()
            .map(|col| {
                // Use the default_width from JobColumn, but with some specific overrides
                // for better display in the jobs list context
                match col {
                    // Override specific columns that need different constraints in the jobs list
                    JobColumn::Name => Constraint::Min(15),
                    JobColumn::WorkDir => Constraint::Min(20),
                    // For time-related columns, we use a slightly longer constraint
                    JobColumn::SubmitTime | JobColumn::StartTime | JobColumn::EndTime => {
                        Constraint::Length(19)
                    }
                    // Use the default_width for all other columns
                    _ => col.default_width(),
                }
            })
            .collect();

        // Create the table
        let table = Table::new(rows, constraints)
            .header(header)
            .block(Block::default().borders(Borders::ALL).title("Jobs"))
            .row_highlight_style(Style::default().add_modifier(Modifier::BOLD))
            .highlight_symbol(" > ");

        // Render the table
        frame.render_stateful_widget(table, area, &mut self.state);
    }

    /// Get the currently selected job, if any
    pub fn selected_job(&self) -> Option<&Job> {
        self.state.selected().and_then(|i| self.jobs.get(i))
    }

    /// Get all selected jobs
    pub fn get_selected_jobs(&self) -> Vec<String> {
        self.selected_jobs
            .iter()
            .filter_map(|&i| self.jobs.get(i))
            .map(|job| job.id.clone())
            .collect()
    }
}
