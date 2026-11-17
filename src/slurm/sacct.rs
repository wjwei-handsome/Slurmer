use async_process::Command;
use color_eyre::eyre::{eyre, Error};
use color_eyre::Result;
use std::str::FromStr;

use super::JobState;

/// Fields requested from `sacct`, in the order they are parsed.
///
/// `JobName` is last on purpose: it is the only field a user can put a `|`
/// into, and keeping it at the end means a stray separator cannot shift the
/// columns we care about.
const SACCT_FIELDS: &[&str] = &[
    "JobID",
    "User",
    "Partition",
    "QOS",
    "State",
    "ExitCode",
    "Elapsed",
    "ElapsedRaw",
    "Start",
    "End",
    "AllocCPUS",
    "CPUTimeRAW",
    "TotalCPU",
    "ReqMem",
    "MaxRSS",
    "NNodes",
    "NodeList",
    "Timelimit",
    "JobName",
];

/// How far back the history view looks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HistoryWindow {
    Day,
    ThreeDays,
    #[default]
    Week,
    Month,
    Quarter,
}

impl HistoryWindow {
    pub fn all() -> Vec<HistoryWindow> {
        vec![
            HistoryWindow::Day,
            HistoryWindow::ThreeDays,
            HistoryWindow::Week,
            HistoryWindow::Month,
            HistoryWindow::Quarter,
        ]
    }

    /// Short label for the UI
    pub fn label(&self) -> &'static str {
        match self {
            HistoryWindow::Day => "24h",
            HistoryWindow::ThreeDays => "3d",
            HistoryWindow::Week => "7d",
            HistoryWindow::Month => "30d",
            HistoryWindow::Quarter => "90d",
        }
    }

    /// Value passed to `sacct --starttime`
    pub fn start_time(&self) -> &'static str {
        match self {
            HistoryWindow::Day => "now-1days",
            HistoryWindow::ThreeDays => "now-3days",
            HistoryWindow::Week => "now-7days",
            HistoryWindow::Month => "now-30days",
            HistoryWindow::Quarter => "now-90days",
        }
    }

    /// Next longer window, saturating at the longest
    pub fn wider(&self) -> Self {
        let all = Self::all();
        let i = all.iter().position(|w| w == self).unwrap_or(0);
        all[(i + 1).min(all.len() - 1)]
    }

    /// Next shorter window, saturating at the shortest
    pub fn narrower(&self) -> Self {
        let all = Self::all();
        let i = all.iter().position(|w| w == self).unwrap_or(0);
        all[i.saturating_sub(1)]
    }
}

/// Filters passed to `sacct`
#[derive(Debug, Clone, Default)]
pub struct SacctOptions {
    /// Limit to a single user; `None` or empty means all users
    pub user: Option<String>,
    pub states: Vec<JobState>,
    pub partitions: Vec<String>,
    pub window: HistoryWindow,
}

impl SacctOptions {
    pub fn to_args(&self) -> Vec<String> {
        let mut args = vec![
            "--starttime".to_string(),
            self.window.start_time().to_string(),
            "--parsable2".to_string(),
            "--noheader".to_string(),
            "--format".to_string(),
            SACCT_FIELDS.join(","),
        ];

        match self.user.as_deref() {
            Some(user) if !user.is_empty() => {
                args.push("--user".to_string());
                args.push(user.to_string());
            }
            // sacct defaults to the calling user, so ask for everyone explicitly
            _ => args.push("--allusers".to_string()),
        }

        if !self.states.is_empty() {
            let states = self
                .states
                .iter()
                .map(|s| s.to_string())
                .collect::<Vec<_>>()
                .join(",");
            args.push("--state".to_string());
            args.push(states);
        }

        if !self.partitions.is_empty() {
            args.push("--partition".to_string());
            args.push(self.partitions.join(","));
        }

        args
    }
}

/// One job from the accounting database, with its resource usage rolled up
/// from the job's steps.
#[derive(Debug, Clone, Default)]
pub struct HistoryJob {
    pub id: String,
    pub name: String,
    pub user: String,
    pub partition: String,
    pub qos: String,
    pub state: JobState,
    pub exit_code: String,
    pub elapsed: String,
    pub elapsed_secs: u64,
    pub start_time: String,
    pub end_time: String,
    pub cpus: u32,
    pub nodes: u32,
    pub node_list: String,
    pub time_limit: String,
    /// CPU seconds reserved for the job (`AllocCPUS` x elapsed)
    pub cpu_time_secs: u64,
    /// CPU seconds actually burned (`TotalCPU`)
    pub total_cpu_secs: f64,
    /// Total memory the job asked for, in bytes
    pub req_mem_bytes: Option<u64>,
    /// Peak resident set size across all steps, in bytes
    pub max_rss_bytes: Option<u64>,
}

impl HistoryJob {
    /// Usage is only final once a job has stopped: `sacct` fills in
    /// `TotalCPU` and `MaxRSS` as steps end, so a running job would look idle.
    fn has_finished(&self) -> bool {
        !matches!(self.state, JobState::Pending | JobState::Running)
    }

    /// Share of the reserved CPU time the job actually used, as a percentage.
    ///
    /// This is the number `seff` reports: `TotalCPU / (AllocCPUS * Elapsed)`.
    pub fn cpu_efficiency(&self) -> Option<f64> {
        if !self.has_finished() || self.cpu_time_secs == 0 {
            return None;
        }
        Some(self.total_cpu_secs / self.cpu_time_secs as f64 * 100.0)
    }

    /// Share of the allocated memory the job actually touched, as a percentage.
    ///
    /// `MaxRSS` is the peak for a single task, so this compares it against the
    /// memory allocated per node. For jobs that pack several tasks onto one
    /// node it is therefore a lower bound on what the node really used.
    pub fn memory_efficiency(&self) -> Option<f64> {
        if !self.has_finished() {
            return None;
        }
        let req = self.req_mem_bytes?;
        let rss = self.max_rss_bytes?;
        let per_node = req / self.nodes.max(1) as u64;
        if per_node == 0 {
            return None;
        }
        Some(rss as f64 / per_node as f64 * 100.0)
    }

    /// Share of the requested walltime the job used, as a percentage.
    pub fn time_efficiency(&self) -> Option<f64> {
        if !self.has_finished() {
            return None;
        }
        let limit = parse_duration(&self.time_limit)?;
        if limit <= 0.0 {
            return None;
        }
        Some(self.elapsed_secs as f64 / limit * 100.0)
    }

    pub fn req_mem_display(&self) -> String {
        self.req_mem_bytes
            .map(format_size)
            .unwrap_or_else(|| "-".to_string())
    }

    pub fn max_rss_display(&self) -> String {
        self.max_rss_bytes
            .map(format_size)
            .unwrap_or_else(|| "-".to_string())
    }
}

/// Run `sacct` and return one entry per job allocation.
pub async fn run_sacct(options: &SacctOptions) -> Result<Vec<HistoryJob>> {
    let args = options.to_args();

    let output = Command::new("sacct")
        .args(&args)
        .output()
        .await
        .map_err(Error::new)?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(eyre!(if stderr.is_empty() {
            "sacct exited with a non-zero status".to_string()
        } else {
            stderr
        }));
    }

    Ok(parse_sacct_output(&String::from_utf8_lossy(&output.stdout)))
}

/// Parse `sacct --parsable2 --noheader` output.
///
/// `sacct` prints one line per allocation followed by one line per step.
/// Only the step lines carry `MaxRSS`, so they are folded into the
/// allocation they belong to and otherwise dropped.
pub fn parse_sacct_output(stdout: &str) -> Vec<HistoryJob> {
    let mut jobs: Vec<HistoryJob> = Vec::new();

    for line in stdout.lines() {
        if line.trim().is_empty() {
            continue;
        }

        let parts: Vec<&str> = line.splitn(SACCT_FIELDS.len(), '|').collect();
        if parts.len() < SACCT_FIELDS.len() {
            continue;
        }

        let id = parts[0].trim();

        // Step line, e.g. `12345.batch`: roll its peak memory into the job
        if let Some((parent, _)) = id.split_once('.') {
            if let Some(job) = jobs.last_mut() {
                if job.id == parent {
                    if let Some(rss) = parse_size(parts[14], UNIT_KIB) {
                        job.max_rss_bytes =
                            Some(job.max_rss_bytes.map_or(rss, |current| current.max(rss)));
                    }
                }
            }
            continue;
        }

        let cpus = parts[10].trim().parse::<u32>().unwrap_or(0);
        let nodes = parts[15].trim().parse::<u32>().unwrap_or(1);

        jobs.push(HistoryJob {
            id: id.to_string(),
            user: parts[1].trim().to_string(),
            partition: parts[2].trim().to_string(),
            qos: parts[3].trim().to_string(),
            // sacct writes reasons into the state, e.g. `CANCELLED by 1000`
            state: parts[4]
                .split_whitespace()
                .next()
                .and_then(|s| JobState::from_str(s).ok())
                .unwrap_or(JobState::Other),
            exit_code: parts[5].trim().to_string(),
            elapsed: parts[6].trim().to_string(),
            elapsed_secs: parts[7].trim().parse::<u64>().unwrap_or(0),
            start_time: parts[8].trim().to_string(),
            end_time: parts[9].trim().to_string(),
            cpus,
            cpu_time_secs: parts[11].trim().parse::<u64>().unwrap_or(0),
            total_cpu_secs: parse_duration(parts[12]).unwrap_or(0.0),
            req_mem_bytes: parse_req_mem(parts[13], cpus, nodes),
            max_rss_bytes: parse_size(parts[14], UNIT_KIB),
            nodes,
            node_list: parts[16].trim().to_string(),
            time_limit: parts[17].trim().to_string(),
            name: parts[18].trim().to_string(),
        });
    }

    jobs
}

const UNIT_KIB: u64 = 1024;
const UNIT_MIB: u64 = 1024 * 1024;

/// Parse a Slurm duration (`[DD-][HH:]MM:SS[.mmm]`) into seconds.
pub fn parse_duration(s: &str) -> Option<f64> {
    let s = s.trim();
    if s.is_empty() || s.eq_ignore_ascii_case("UNLIMITED") || s.contains("Partition") {
        return None;
    }

    // A leading `DD-` splits off whole days and leaves `HH:MM:SS`
    let (days, rest) = match s.split_once('-') {
        Some((days, rest)) => (days.trim().parse::<f64>().ok()?, rest),
        None => (0.0, s),
    };

    let fields: Vec<f64> = rest
        .split(':')
        .map(|f| f.trim().parse::<f64>().ok())
        .collect::<Option<Vec<f64>>>()?;

    let seconds = match fields.as_slice() {
        [hours, minutes, seconds] => hours * 3600.0 + minutes * 60.0 + seconds,
        [minutes, seconds] => minutes * 60.0 + seconds,
        [seconds] => *seconds,
        _ => return None,
    };

    Some(days * 86400.0 + seconds)
}

/// Parse a Slurm size (`1234K`, `5.6G`, `2048`) into bytes, taking an
/// unsuffixed value to be in `default_unit`.
pub fn parse_size(s: &str, default_unit: u64) -> Option<u64> {
    let s = s.trim();
    let last = s.chars().last()?;

    let (digits, unit) = match last.to_ascii_uppercase() {
        'K' => (&s[..s.len() - last.len_utf8()], UNIT_KIB),
        'M' => (&s[..s.len() - last.len_utf8()], UNIT_MIB),
        'G' => (&s[..s.len() - last.len_utf8()], UNIT_MIB * 1024),
        'T' => (&s[..s.len() - last.len_utf8()], UNIT_MIB * 1024 * 1024),
        'P' => (
            &s[..s.len() - last.len_utf8()],
            UNIT_MIB * 1024 * 1024 * 1024,
        ),
        _ if last.is_ascii_digit() => (s, default_unit),
        _ => return None,
    };

    let value = digits.trim().parse::<f64>().ok()?;
    if value < 0.0 {
        return None;
    }

    Some((value * unit as f64) as u64)
}

/// Parse `sacct`'s `ReqMem` into the total bytes a job asked for.
///
/// Current Slurm releases report a plain total (`16G`). Older ones append
/// `n` for a per-node request or `c` for a per-CPU one, which have to be
/// multiplied out.
pub fn parse_req_mem(s: &str, cpus: u32, nodes: u32) -> Option<u64> {
    let s = s.trim();
    let last = s.chars().last()?;

    let (size, factor) = match last {
        'n' | 'N' => (&s[..s.len() - last.len_utf8()], nodes.max(1) as u64),
        'c' | 'C' => (&s[..s.len() - last.len_utf8()], cpus.max(1) as u64),
        _ => (s, 1),
    };

    // `ReqMem` is reported in megabytes when it carries no unit suffix
    Some(parse_size(size, UNIT_MIB)? * factor)
}

/// Render a byte count the way Slurm tools do, e.g. `3.4G`
pub fn format_size(bytes: u64) -> String {
    const UNITS: [(u64, &str); 4] = [
        (1024 * 1024 * 1024 * 1024, "T"),
        (1024 * 1024 * 1024, "G"),
        (1024 * 1024, "M"),
        (1024, "K"),
    ];

    for (unit, suffix) in UNITS {
        if bytes >= unit {
            return format!("{:.1}{}", bytes as f64 / unit as f64, suffix);
        }
    }

    format!("{}B", bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_durations() {
        assert_eq!(parse_duration("00:00:30"), Some(30.0));
        assert_eq!(parse_duration("01:02:03"), Some(3723.0));
        assert_eq!(parse_duration("2-03:00:00"), Some(183600.0));
        assert_eq!(parse_duration("05:30"), Some(330.0));
        assert_eq!(parse_duration("00:00.004"), Some(0.004));
        assert_eq!(parse_duration("UNLIMITED"), None);
        assert_eq!(parse_duration(""), None);
    }

    #[test]
    fn parses_sizes() {
        assert_eq!(parse_size("1024K", UNIT_KIB), Some(1024 * 1024));
        assert_eq!(parse_size("2.5G", UNIT_KIB), Some(2684354560));
        // no suffix falls back to the caller's unit
        assert_eq!(parse_size("2048", UNIT_KIB), Some(2048 * 1024));
        assert_eq!(parse_size("2048", UNIT_MIB), Some(2048 * 1024 * 1024));
        assert_eq!(parse_size("", UNIT_KIB), None);
    }

    #[test]
    fn parses_req_mem_in_both_slurm_dialects() {
        // modern: a plain total in megabytes
        assert_eq!(parse_req_mem("4G", 8, 2), Some(4 * 1024 * 1024 * 1024));
        // legacy per-node
        assert_eq!(parse_req_mem("4Gn", 8, 2), Some(8 * 1024 * 1024 * 1024));
        // legacy per-cpu
        assert_eq!(parse_req_mem("1Gc", 8, 2), Some(8 * 1024 * 1024 * 1024));
        assert_eq!(parse_req_mem("", 1, 1), None);
    }

    /// `MaxRSS` only shows up on step lines, so it has to be folded back into
    /// the allocation that owns them.
    #[test]
    fn rolls_step_memory_into_the_allocation() {
        let output = "\
12345|alice|gpu|normal|COMPLETED|0:0|00:10:00|600|2024-01-01T10:00:00|2024-01-01T10:10:00|4|2400|00:40:00|8G||1|node01|01:00:00|train
12345.batch|alice|gpu|normal|COMPLETED|0:0|00:10:00|600|2024-01-01T10:00:00|2024-01-01T10:10:00|4|2400|00:40:00||2G|1|node01||batch
12345.extern|alice|gpu|normal|COMPLETED|0:0|00:10:00|600|2024-01-01T10:00:00|2024-01-01T10:10:00|4|2400|00:00:00||1024K|1|node01||extern
";

        let jobs = parse_sacct_output(output);
        assert_eq!(jobs.len(), 1);

        let job = &jobs[0];
        assert_eq!(job.id, "12345");
        assert_eq!(job.name, "train");
        assert_eq!(job.cpus, 4);
        // the batch step's 2G beats the extern step's 1M
        assert_eq!(job.max_rss_bytes, Some(2 * 1024 * 1024 * 1024));
        // 40 CPU-minutes used out of 4 CPUs x 10 minutes reserved
        assert_eq!(job.cpu_efficiency(), Some(100.0));
        assert_eq!(job.memory_efficiency(), Some(25.0));
        // 10 minutes of a one hour limit
        let time_eff = job.time_efficiency().expect("a time limit was given");
        assert!((time_eff - 100.0 / 6.0).abs() < 1e-9, "got {}", time_eff);
    }

    #[test]
    fn keeps_a_pipe_in_a_job_name() {
        let output = "99|bob|cpu|normal|FAILED|1:0|00:00:05|5|2024-01-01T10:00:00|2024-01-01T10:00:05|1|5|00:00:01|1G||1|node02|00:10:00|a|b\n";

        let jobs = parse_sacct_output(output);
        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].name, "a|b");
        assert_eq!(jobs[0].state, JobState::Failed);
    }

    #[test]
    fn reads_a_cancellation_reason_as_cancelled() {
        let output = "7|bob|cpu|normal|CANCELLED by 1000|0:0|00:00:05|5|Unknown|2024-01-01T10:00:05|1|5|00:00:00|1G||1|node02|00:10:00|sleep\n";

        let jobs = parse_sacct_output(output);
        assert_eq!(jobs[0].state, JobState::Cancelled);
    }

    #[test]
    fn skips_jobs_that_never_ran() {
        // a pending job has no elapsed time, so there is nothing to divide by
        let output = "8|bob|cpu|normal|PENDING|0:0|00:00:00|0|Unknown|Unknown|1|0|00:00:00|1G||1||00:10:00|queued\n";

        let jobs = parse_sacct_output(output);
        assert_eq!(jobs[0].cpu_efficiency(), None);
        assert_eq!(jobs[0].memory_efficiency(), None);
    }

    /// A running job's CPU and memory figures are still filling in, so they
    /// must read as unknown rather than as an idle allocation.
    #[test]
    fn leaves_running_jobs_unscored() {
        let output = "9|bob|cpu|normal|RUNNING|0:0|00:30:00|1800|2024-01-01T10:00:00|Unknown|8|14400|00:00:00|16G||1|node03|02:00:00|still_going\n";

        let jobs = parse_sacct_output(output);
        assert_eq!(jobs[0].state, JobState::Running);
        assert_eq!(jobs[0].cpu_efficiency(), None);
        assert_eq!(jobs[0].memory_efficiency(), None);
        assert_eq!(jobs[0].time_efficiency(), None);
    }

    #[test]
    fn builds_sacct_args() {
        let options = SacctOptions {
            user: Some("alice".to_string()),
            states: vec![JobState::Completed],
            partitions: vec!["gpu".to_string()],
            window: HistoryWindow::Week,
        };
        let args = options.to_args();

        assert!(args.windows(2).any(|w| w == ["--starttime", "now-7days"]));
        assert!(args.windows(2).any(|w| w == ["--user", "alice"]));
        assert!(args.windows(2).any(|w| w == ["--state", "COMPLETED"]));
        assert!(args.windows(2).any(|w| w == ["--partition", "gpu"]));
        assert!(!args.iter().any(|a| a == "--allusers"));
    }

    #[test]
    fn asks_for_all_users_without_a_user_filter() {
        let options = SacctOptions::default();
        assert!(options.to_args().iter().any(|a| a == "--allusers"));
    }
}
