use async_process::{Command, Output};
use color_eyre::Result;
use std::collections::HashMap;
use std::str::FromStr;

use super::Job;
use super::JobState;

#[derive(Debug, Clone)]
pub struct SqueueOptions {
    pub user: Option<String>,
    pub states: Vec<JobState>,
    pub partitions: Vec<String>,
    pub qos: Vec<String>,
    pub name_filter: Option<String>,
    pub format: String,
    pub sort_by: Option<String>,
    pub sort_desc: bool,
}

impl Default for SqueueOptions {
    fn default() -> Self {
        // Default username from environment
        let username = std::env::var("USER").unwrap_or_else(|_| "".to_string());

        Self {
            user: Some(username),
            states: Vec::new(),
            partitions: Vec::new(),
            qos: Vec::new(),
            name_filter: None,
            format: "%i|%j|%u|%T|%M|%N|%C|%m|%P|%q".to_string(), // JobID|Name|User|State|Time|Nodes|CPUs|Memory|Partition|QOS
            sort_by: None,
            sort_desc: false,
        }
    }
}

impl SqueueOptions {
    pub fn to_args(&self) -> Vec<String> {
        let mut args = Vec::new();

        // User filter
        if let Some(user) = &self.user {
            args.push("--user".to_string());
            args.push(user.clone());
        }

        // State filter
        if !self.states.is_empty() {
            let states = self
                .states
                .iter()
                .map(|s| s.to_string())
                .collect::<Vec<_>>()
                .join(",");
            args.push("--states".to_string());
            args.push(states);
        }

        // Partition filter
        if !self.partitions.is_empty() {
            let partitions = self.partitions.join(",");
            args.push("--partition".to_string());
            args.push(partitions);
        }

        // QOS filter
        if !self.qos.is_empty() {
            let qos = self.qos.join(",");
            args.push("--qos".to_string());
            args.push(qos);
        }

        // Name filter
        if let Some(name) = &self.name_filter {
            args.push("--name".to_string());
            args.push(name.clone());
        }

        // Format specification
        args.push("--format".to_string());
        args.push(self.format.clone());

        // Sort options
        if let Some(sort_by) = &self.sort_by {
            let sort_flag = if self.sort_desc {
                format!("-{}", sort_by)
            } else {
                sort_by.clone()
            };
            args.push("--sort".to_string());
            args.push(sort_flag);
        }

        // No header flag to make parsing easier
        args.push("--noheader".to_string());

        args
    }
}

pub async fn run_squeue(options: &SqueueOptions) -> Result<Vec<Job>> {
    let args = options.to_args();

    let output = Command::new("squeue").args(&args).output().await?;

    parse_squeue_output(&output)
}

fn parse_squeue_output(output: &Output) -> Result<Vec<Job>> {
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.lines().collect();

    let mut jobs = Vec::new();

    for line in lines {
        if line.trim().is_empty() {
            continue;
        }

        let parts: Vec<&str> = line.split('|').collect();

        // Ensure we have the expected number of parts
        if parts.len() < 10 {
            continue;
        }

        // Parse job information
        let job = Job {
            id: parts[0].trim().to_string(),
            name: parts[1].trim().to_string(),
            user: parts[2].trim().to_string(),
            state: JobState::from_str(parts[3].trim()).unwrap_or(JobState::Other),
            time: parts[4].trim().to_string(),
            nodes: parts[5].trim().parse::<u32>().unwrap_or(0),
            cpus: parts[6].trim().parse::<u32>().unwrap_or(0),
            memory: parts[7].trim().to_string(),
            partition: parts[8].trim().to_string(),
            qos: parts[9].trim().to_string(),
        };

        jobs.push(job);
    }

    Ok(jobs)
}

pub fn available_partitions() -> Result<Vec<String>> {
    // In a real implementation, this would query Slurm for available partitions
    // For now, we'll return a placeholder
    Ok(vec![
        "compute".to_string(),
        "gpu".to_string(),
        "debug".to_string(),
    ])
}

pub fn available_qos() -> Result<Vec<String>> {
    // In a real implementation, this would query Slurm for available QOS options
    // For now, we'll return a placeholder
    Ok(vec![
        "normal".to_string(),
        "high".to_string(),
        "urgent".to_string(),
    ])
}
