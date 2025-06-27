use color_eyre::Result;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Row, Table, TableState},
};

use crate::slurm::{Job, JobState};

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
        match self.sort_column {
            0 => self.jobs.sort_by(|a, b| {
                let a_id = a.id.parse::<u32>().unwrap_or(0);
                let b_id = b.id.parse::<u32>().unwrap_or(0);
                if self.sort_ascending {
                    a_id.cmp(&b_id)
                } else {
                    b_id.cmp(&a_id)
                }
            }),
            1 => self.jobs.sort_by(|a, b| {
                if self.sort_ascending {
                    a.name.cmp(&b.name)
                } else {
                    b.name.cmp(&a.name)
                }
            }),
            2 => self.jobs.sort_by(|a, b| {
                if self.sort_ascending {
                    a.user.cmp(&b.user)
                } else {
                    b.user.cmp(&a.user)
                }
            }),
            3 => self.jobs.sort_by(|a, b| {
                if self.sort_ascending {
                    format!("{:?}", a.state).cmp(&format!("{:?}", b.state))
                } else {
                    format!("{:?}", b.state).cmp(&format!("{:?}", a.state))
                }
            }),
            4 => self.jobs.sort_by(|a, b| {
                if self.sort_ascending {
                    a.partition.cmp(&b.partition)
                } else {
                    b.partition.cmp(&a.partition)
                }
            }),
            5 => self.jobs.sort_by(|a, b| {
                if self.sort_ascending {
                    a.qos.cmp(&b.qos)
                } else {
                    b.qos.cmp(&a.qos)
                }
            }),
            6 => self.jobs.sort_by(|a, b| {
                if self.sort_ascending {
                    a.nodes.cmp(&b.nodes)
                } else {
                    b.nodes.cmp(&a.nodes)
                }
            }),
            7 => self.jobs.sort_by(|a, b| {
                if self.sort_ascending {
                    a.cpus.cmp(&b.cpus)
                } else {
                    b.cpus.cmp(&a.cpus)
                }
            }),
            _ => {}
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
            self.sort_ascending = !self.sort_ascending;
        } else {
            self.sort_column = column;
            self.sort_ascending = true;
        }
        self.sort_jobs();
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
    pub fn render(&mut self, frame: &mut Frame, area: Rect) {
        // Define table headers
        let headers = [
            "ID",
            "Name",
            "User",
            "State",
            "Partition",
            "QoS",
            "Nodes",
            "CPUs",
            "Time",
        ];

        // Create header cells with appropriate styling
        let header_cells = headers.iter().enumerate().map(|(i, &h)| {
            let header_style = if i == self.sort_column {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD)
            };

            let sort_indicator = if i == self.sort_column {
                if self.sort_ascending { " ↑" } else { " ↓" }
            } else {
                ""
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

            let cells = [
                Cell::from(job.id.clone()),
                Cell::from(job.name.clone()),
                Cell::from(job.user.clone()),
                Cell::from(format!("{}", job.state)),
                Cell::from(job.partition.clone()),
                Cell::from(job.qos.clone()),
                Cell::from(job.nodes.to_string()),
                Cell::from(job.cpus.to_string()),
                Cell::from(job.time.clone()),
            ];

            Row::new(cells).style(style).height(1)
        });

        // Create constraints for columns
        let constraints = [
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
