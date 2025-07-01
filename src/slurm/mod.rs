pub mod scancel;
pub mod scontrol;
pub mod squeue;

use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobState {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
    Timeout,
    NodeFail,
    Preempted,
    Boot,
    Other,
}

impl fmt::Display for JobState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let state_str = match self {
            JobState::Pending => "PENDING",
            JobState::Running => "RUNNING",
            JobState::Completed => "COMPLETED",
            JobState::Failed => "FAILED",
            JobState::Cancelled => "CANCELLED",
            JobState::Timeout => "TIMEOUT",
            JobState::NodeFail => "NODE_FAIL",
            JobState::Preempted => "PREEMPTED",
            JobState::Boot => "BOOT_FAIL",
            JobState::Other => "OTHER",
        };
        write!(f, "{}", state_str)
    }
}

impl FromStr for JobState {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "PENDING" | "PD" => Ok(JobState::Pending),
            "RUNNING" | "R" => Ok(JobState::Running),
            "COMPLETED" | "CD" | "COMPLETING" | "CG" => Ok(JobState::Completed),
            "FAILED" | "F" => Ok(JobState::Failed),
            "CANCELLED" | "CA" => Ok(JobState::Cancelled),
            "TIMEOUT" | "TO" => Ok(JobState::Timeout),
            "NODE_FAIL" | "NF" => Ok(JobState::NodeFail),
            "PREEMPTED" | "PR" => Ok(JobState::Preempted),
            "BOOT_FAIL" | "BF" => Ok(JobState::Boot),
            _ => Ok(JobState::Other),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Job {
    pub id: String,
    pub name: String,
    pub user: String,
    pub state: JobState,
    pub time: String,
    pub nodes: u32,
    pub cpus: u32,
    pub memory: String,
    pub partition: String,
    pub qos: String,
    pub account: Option<String>,
    pub priority: Option<u32>,
    pub work_dir: Option<String>,
    pub submit_time: Option<String>,
    pub start_time: Option<String>,
    pub end_time: Option<String>,
}

impl Default for Job {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            user: String::new(),
            state: JobState::Other,
            time: String::new(),
            nodes: 0,
            cpus: 0,
            memory: String::new(),
            partition: String::new(),
            qos: String::new(),
            account: None,
            priority: None,
            work_dir: None,
            submit_time: None,
            start_time: None,
            end_time: None,
        }
    }
}
