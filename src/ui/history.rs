use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Table, TableState},
    Frame,
};
use std::cmp::Ordering;

use crate::slurm::sacct::{HistoryJob, HistoryWindow};
use crate::ui::jobslist::state_color;

/// What the history view wants the app to do after a key press
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HistoryAction {
    None,
    Close,
    /// Re-run `sacct`, e.g. because the time window changed
    Refresh,
    /// Hand over to the shared filter popup
    OpenFilter,
}

/// Field the history table is sorted on
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HistorySort {
    EndTime,
    CpuEfficiency,
    MemEfficiency,
    Elapsed,
    JobId,
}

impl HistorySort {
    fn all() -> Vec<HistorySort> {
        vec![
            HistorySort::EndTime,
            HistorySort::CpuEfficiency,
            HistorySort::MemEfficiency,
            HistorySort::Elapsed,
            HistorySort::JobId,
        ]
    }

    fn label(&self) -> &'static str {
        match self {
            HistorySort::EndTime => "End",
            HistorySort::CpuEfficiency => "CPU%",
            HistorySort::MemEfficiency => "Mem%",
            HistorySort::Elapsed => "Elapsed",
            HistorySort::JobId => "ID",
        }
    }

    fn next(&self) -> Self {
        let all = Self::all();
        let i = all.iter().position(|s| s == self).unwrap_or(0);
        all[(i + 1) % all.len()]
    }
}

/// Past jobs from the accounting database, with how much of their allocation
/// they actually used.
pub struct HistoryView {
    pub visible: bool,
    pub jobs: Vec<HistoryJob>,
    pub state: TableState,
    pub window: HistoryWindow,
    pub sort: HistorySort,
    pub descending: bool,
    /// Why the last `sacct` call failed, if it did
    pub error: Option<String>,
}

impl Default for HistoryView {
    fn default() -> Self {
        Self {
            visible: false,
            jobs: Vec::new(),
            state: TableState::default(),
            window: HistoryWindow::default(),
            sort: HistorySort::EndTime,
            descending: true,
            error: None,
        }
    }
}

impl HistoryView {
    pub fn new() -> Self {
        Self::default()
    }

    /// Replace the job list and re-apply the current sort
    pub fn update_jobs(&mut self, jobs: Vec<HistoryJob>) {
        self.jobs = jobs;
        self.error = None;
        self.apply_sort();

        if self.jobs.is_empty() {
            self.state.select(None);
        } else {
            let selected = self.state.selected().unwrap_or(0);
            self.state.select(Some(selected.min(self.jobs.len() - 1)));
        }
    }

    fn apply_sort(&mut self) {
        let descending = self.descending;
        match self.sort {
            HistorySort::EndTime => self
                .jobs
                .sort_by(|a, b| directed(a.end_time.cmp(&b.end_time), descending)),
            HistorySort::JobId => self.jobs.sort_by(|a, b| {
                let ordering = numeric_id(&a.id)
                    .cmp(&numeric_id(&b.id))
                    .then_with(|| a.id.cmp(&b.id));
                directed(ordering, descending)
            }),
            HistorySort::Elapsed => self
                .jobs
                .sort_by(|a, b| directed(a.elapsed_secs.cmp(&b.elapsed_secs), descending)),
            HistorySort::CpuEfficiency => self
                .jobs
                .sort_by(|a, b| by_metric(a.cpu_efficiency(), b.cpu_efficiency(), descending)),
            HistorySort::MemEfficiency => self.jobs.sort_by(|a, b| {
                by_metric(a.memory_efficiency(), b.memory_efficiency(), descending)
            }),
        }
    }

    /// Move to another time window, re-querying only if it actually changed
    fn change_window(&mut self, window: HistoryWindow) -> HistoryAction {
        if window == self.window {
            return HistoryAction::None;
        }
        self.window = window;
        HistoryAction::Refresh
    }

    pub fn next(&mut self) {
        if self.jobs.is_empty() {
            return;
        }
        let i = match self.state.selected() {
            Some(i) if i >= self.jobs.len() - 1 => 0,
            Some(i) => i + 1,
            None => 0,
        };
        self.state.select(Some(i));
    }

    pub fn previous(&mut self) {
        if self.jobs.is_empty() {
            return;
        }
        let i = match self.state.selected() {
            Some(0) | None => self.jobs.len() - 1,
            Some(i) => i - 1,
        };
        self.state.select(Some(i));
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> HistoryAction {
        match key.code {
            KeyCode::Esc | KeyCode::Char('h') => HistoryAction::Close,
            KeyCode::Down => {
                self.next();
                HistoryAction::None
            }
            KeyCode::Up => {
                self.previous();
                HistoryAction::None
            }
            // A different window needs fresh `sacct` output
            KeyCode::Right => self.change_window(self.window.wider()),
            KeyCode::Left => self.change_window(self.window.narrower()),
            // Sorting is local to what we already fetched
            KeyCode::Char('s') => {
                self.sort = self.sort.next();
                self.apply_sort();
                HistoryAction::None
            }
            KeyCode::Char('o') => {
                self.descending = !self.descending;
                self.apply_sort();
                HistoryAction::None
            }
            KeyCode::Char('r') => HistoryAction::Refresh,
            KeyCode::Char('f') => HistoryAction::OpenFilter,
            _ => HistoryAction::None,
        }
    }

    pub fn render(&mut self, frame: &mut Frame, area: Rect) {
        frame.render_widget(Clear, area);
        frame.render_widget(
            Block::default()
                .title(Line::from("Job History").centered())
                .borders(Borders::NONE)
                .style(Style::default().bg(Color::Black)),
            area,
        );

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([
                Constraint::Min(5),    // job table
                Constraint::Length(5), // selected job, totals and help
            ])
            .split(area);

        self.render_table(frame, chunks[0]);
        self.render_summary(frame, chunks[1]);
    }

    fn render_table(&mut self, frame: &mut Frame, area: Rect) {
        if let Some(error) = &self.error {
            let message = Paragraph::new(format!(
                "Could not read job history:\n{}\n\nThe history view needs `sacct` and Slurm job accounting on this cluster.",
                error
            ))
            .style(Style::default().fg(Color::Red))
            .block(Block::default().borders(Borders::ALL).title("Error"));
            frame.render_widget(message, area);
            return;
        }

        let headers = [
            "ID", "Name", "State", "Elapsed", "CPU", "CPU%", "Req", "Used", "Mem%", "Time%", "End",
        ];
        let header = Row::new(headers.iter().map(|h| {
            let style = Style::default()
                .fg(if self.sort.label() == *h {
                    Color::Cyan
                } else {
                    Color::White
                })
                .add_modifier(Modifier::BOLD);
            Cell::from(*h).style(style)
        }))
        .style(Style::default().bg(Color::DarkGray))
        .height(1);

        let rows = self.jobs.iter().map(|job| {
            let cpu = job.cpu_efficiency();
            let mem = job.memory_efficiency();
            let time = job.time_efficiency();

            Row::new(vec![
                Cell::from(job.id.clone()),
                Cell::from(job.name.clone()),
                Cell::from(job.state.to_string())
                    .style(Style::default().fg(state_color(job.state))),
                Cell::from(job.elapsed.clone()),
                Cell::from(job.cpus.to_string()),
                Cell::from(percent(cpu)).style(Style::default().fg(efficiency_color(cpu))),
                Cell::from(job.req_mem_display()),
                Cell::from(job.max_rss_display()),
                Cell::from(percent(mem)).style(Style::default().fg(efficiency_color(mem))),
                Cell::from(percent(time)).style(Style::default().fg(efficiency_color(time))),
                Cell::from(job.end_time.clone()),
            ])
            .height(1)
        });

        let constraints = [
            Constraint::Length(10), // ID
            Constraint::Min(10),    // Name
            Constraint::Length(13), // State, long enough for OUT_OF_MEMORY
            Constraint::Length(9),  // Elapsed
            Constraint::Length(4),  // CPU count
            Constraint::Length(6),  // CPU efficiency
            Constraint::Length(7),  // requested memory
            Constraint::Length(7),  // peak memory used
            Constraint::Length(6),  // memory efficiency
            Constraint::Length(6),  // walltime efficiency
            Constraint::Length(16), // end time
        ];

        let table = Table::new(rows, constraints)
            .header(header)
            .block(Block::default().borders(Borders::ALL).title(format!(
                "{} Jobs (last {})",
                self.jobs.len(),
                self.window.label()
            )))
            .row_highlight_style(Style::default().add_modifier(Modifier::BOLD))
            .highlight_symbol(" ▶ ");

        frame.render_stateful_widget(table, area, &mut self.state);
    }

    /// The fields that do not fit in the table, for the highlighted job
    fn selected_detail(&self) -> Line<'static> {
        let label = Style::default().fg(Color::Cyan);
        let Some(job) = self.state.selected().and_then(|i| self.jobs.get(i)) else {
            return Line::from(Span::styled(
                "No job selected",
                Style::default().fg(Color::DarkGray),
            ));
        };

        Line::from(vec![
            Span::styled("Job ", label),
            Span::raw(job.id.clone()),
            Span::styled("  User: ", label),
            Span::raw(or_dash(&job.user)),
            Span::styled("  Partition: ", label),
            Span::raw(or_dash(&job.partition)),
            Span::styled("  QoS: ", label),
            Span::raw(or_dash(&job.qos)),
            Span::styled("  Nodes: ", label),
            Span::raw(or_dash(&job.node_list)),
            Span::styled("  Exit: ", label),
            Span::raw(or_dash(&job.exit_code)),
            Span::styled("  Started: ", label),
            Span::raw(or_dash(&job.start_time)),
        ])
    }

    fn render_summary(&self, frame: &mut Frame, area: Rect) {
        let mean_cpu = mean(self.jobs.iter().filter_map(|j| j.cpu_efficiency()));
        let mean_mem = mean(self.jobs.iter().filter_map(|j| j.memory_efficiency()));
        let label = Style::default().fg(Color::Cyan);

        let summary = Line::from(vec![
            Span::styled("Mean CPU: ", label),
            Span::styled(
                percent(mean_cpu),
                Style::default().fg(efficiency_color(mean_cpu)),
            ),
            Span::styled("  Mean Mem: ", label),
            Span::styled(
                percent(mean_mem),
                Style::default().fg(efficiency_color(mean_mem)),
            ),
            Span::styled("  Window: ", label),
            Span::raw(self.window.label()),
            Span::styled("  Sort: ", label),
            Span::raw(format!(
                "{} {}",
                self.sort.label(),
                if self.descending { "↓" } else { "↑" }
            )),
        ]);

        let help = Line::from(vec![
            Span::styled("↑/↓", label),
            Span::raw(": Navigate  "),
            Span::styled("←/→", label),
            Span::raw(": Window  "),
            Span::styled("s", label),
            Span::raw(": Sort  "),
            Span::styled("o", label),
            Span::raw(": Order  "),
            Span::styled("f", label),
            Span::raw(": Filter  "),
            Span::styled("r", label),
            Span::raw(": Refresh  "),
            Span::styled("Esc", label),
            Span::raw(": Close"),
        ]);

        frame.render_widget(
            Paragraph::new(vec![self.selected_detail(), summary, help])
                .block(Block::default().borders(Borders::ALL)),
            area,
        );
    }
}

fn or_dash(value: &str) -> String {
    if value.is_empty() {
        "-".to_string()
    } else {
        value.to_string()
    }
}

fn directed(ordering: Ordering, descending: bool) -> Ordering {
    if descending {
        ordering.reverse()
    } else {
        ordering
    }
}

/// Order two metrics in the chosen direction, always listing jobs that have
/// no value (still running, or never started) last.
fn by_metric(a: Option<f64>, b: Option<f64>, descending: bool) -> Ordering {
    match (a, b) {
        (Some(a), Some(b)) => directed(a.total_cmp(&b), descending),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    }
}

/// Leading number of a job id, so `99` sorts before `100` and array tasks
/// stay next to their parent.
fn numeric_id(id: &str) -> u64 {
    id.chars()
        .take_while(|c| c.is_ascii_digit())
        .collect::<String>()
        .parse()
        .unwrap_or(0)
}

fn mean(values: impl Iterator<Item = f64>) -> Option<f64> {
    let values: Vec<f64> = values.collect();
    if values.is_empty() {
        return None;
    }

    Some(values.iter().sum::<f64>() / values.len() as f64)
}

fn percent(value: Option<f64>) -> String {
    match value {
        Some(v) => format!("{:.0}%", v),
        None => "-".to_string(),
    }
}

/// Green for a well used allocation, red for a wasted one, cyan when the job
/// went past what it reserved.
fn efficiency_color(value: Option<f64>) -> Color {
    match value {
        None => Color::DarkGray,
        Some(v) if v > 100.0 => Color::Cyan,
        Some(v) if v >= 75.0 => Color::Green,
        Some(v) if v >= 40.0 => Color::Yellow,
        Some(_) => Color::Red,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::slurm::JobState;

    fn job(id: &str, cpu_secs: f64, alloc_cpu_secs: u64, end: &str) -> HistoryJob {
        HistoryJob {
            id: id.to_string(),
            total_cpu_secs: cpu_secs,
            cpu_time_secs: alloc_cpu_secs,
            end_time: end.to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn sorts_by_cpu_efficiency_worst_first() {
        let mut view = HistoryView::new();
        view.sort = HistorySort::CpuEfficiency;
        view.descending = false;
        view.update_jobs(vec![
            job("1", 90.0, 100, "2024-01-01T10:00:00"),
            job("2", 10.0, 100, "2024-01-01T11:00:00"),
            job("3", 50.0, 100, "2024-01-01T12:00:00"),
        ]);

        let ids: Vec<&str> = view.jobs.iter().map(|j| j.id.as_str()).collect();
        assert_eq!(ids, ["2", "3", "1"]);
    }

    /// Worst-first (ascending) is the useful order for spotting waste, so
    /// jobs with nothing to score must not crowd the top of it.
    #[test]
    fn keeps_jobs_without_metrics_at_the_bottom_in_both_orders() {
        for descending in [true, false] {
            let mut view = HistoryView::new();
            view.sort = HistorySort::CpuEfficiency;
            view.descending = descending;
            view.update_jobs(vec![
                // never ran, so there is no efficiency to report
                job("1", 0.0, 0, "2024-01-01T10:00:00"),
                job("2", 50.0, 100, "2024-01-01T11:00:00"),
                job("3", 90.0, 100, "2024-01-01T12:00:00"),
            ]);

            let ids: Vec<&str> = view.jobs.iter().map(|j| j.id.as_str()).collect();
            let expected = if descending {
                ["3", "2", "1"]
            } else {
                ["2", "3", "1"]
            };
            assert_eq!(ids, expected, "descending = {}", descending);
        }
    }

    #[test]
    fn sorts_job_ids_numerically() {
        let mut view = HistoryView::new();
        view.sort = HistorySort::JobId;
        view.descending = false;
        view.update_jobs(vec![
            job("100", 1.0, 1, ""),
            job("99", 1.0, 1, ""),
            job("1000", 1.0, 1, ""),
        ]);

        let ids: Vec<&str> = view.jobs.iter().map(|j| j.id.as_str()).collect();
        assert_eq!(ids, ["99", "100", "1000"]);
    }

    #[test]
    fn clamps_the_selection_when_the_list_shrinks() {
        let mut view = HistoryView::new();
        view.update_jobs(vec![job("1", 1.0, 1, ""), job("2", 1.0, 1, "")]);
        view.next();
        assert_eq!(view.state.selected(), Some(1));

        view.update_jobs(vec![job("1", 1.0, 1, "")]);
        assert_eq!(view.state.selected(), Some(0));

        view.update_jobs(Vec::new());
        assert_eq!(view.state.selected(), None);
    }

    /// Draw the view into an off-screen buffer so the table is checked
    /// without a Slurm cluster to talk to.
    #[test]
    fn renders_the_efficiency_columns() {
        use ratatui::{backend::TestBackend, Terminal};

        let mut view = HistoryView::new();
        view.update_jobs(vec![HistoryJob {
            id: "12345".to_string(),
            name: "train".to_string(),
            user: "alice".to_string(),
            state: JobState::Completed,
            elapsed: "00:10:00".to_string(),
            elapsed_secs: 600,
            cpus: 4,
            nodes: 1,
            time_limit: "01:00:00".to_string(),
            // half of the reserved 4 x 10 CPU-minutes
            cpu_time_secs: 2400,
            total_cpu_secs: 1200.0,
            // a quarter of the 8G asked for
            req_mem_bytes: Some(8 * 1024 * 1024 * 1024),
            max_rss_bytes: Some(2 * 1024 * 1024 * 1024),
            ..Default::default()
        }]);

        let mut terminal = Terminal::new(TestBackend::new(120, 14)).expect("test backend");
        terminal
            .draw(|frame| view.render(frame, frame.area()))
            .expect("draw");

        let rendered: String = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect();

        for expected in [
            "CPU%",
            "Mem%",
            "12345",
            "train",
            "COMPLETED",
            "50%", // cpu efficiency
            "25%", // memory efficiency
            "17%", // walltime efficiency
            "8.0G",
            "2.0G",
            "alice", // detail line under the table
            "1 Jobs (last 7d)",
        ] {
            assert!(
                rendered.contains(expected),
                "{expected:?} missing from:\n{rendered}"
            );
        }
    }

    /// The widest window is the slowest query, so pressing past it must
    /// not run it again.
    #[test]
    fn stays_put_past_the_widest_window() {
        let mut view = HistoryView::new();
        view.window = HistoryWindow::Quarter;

        let action = view.handle_key(KeyEvent::from(KeyCode::Right));
        assert_eq!(action, HistoryAction::None);
        assert_eq!(view.window, HistoryWindow::Quarter);
    }

    #[test]
    fn widening_the_window_asks_for_a_refresh() {
        let mut view = HistoryView::new();
        view.window = HistoryWindow::Day;

        let action = view.handle_key(KeyEvent::from(KeyCode::Right));
        assert_eq!(action, HistoryAction::Refresh);
        assert_eq!(view.window, HistoryWindow::ThreeDays);

        // sorting does not need new data
        assert_eq!(
            view.handle_key(KeyEvent::from(KeyCode::Char('s'))),
            HistoryAction::None
        );
    }
}
