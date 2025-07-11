use async_process::{Command, Output, Stdio};
use color_eyre::Result;
use std::collections::HashMap;

/// Execute a Slurm command asynchronously and return the output
pub async fn execute_command(cmd: &str, args: Vec<String>) -> Result<Output> {
    let output = Command::new(cmd).args(args).output().await?;

    Ok(output)
}

/// Execute the squeue command to get job information
pub async fn execute_squeue(args: Vec<String>) -> Result<String> {
    let output = execute_command("squeue", args).await?;
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    Ok(stdout)
}

/// Execute the scontrol command to get detailed job information
pub async fn execute_scontrol(job_id: &str) -> Result<String> {
    let args = vec!["show".to_string(), "job".to_string(), job_id.to_string()];
    let output = execute_command("scontrol", args).await?;
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    Ok(stdout)
}

/// Execute the scancel command to cancel jobs
pub async fn execute_scancel(job_ids: Vec<String>) -> Result<()> {
    if job_ids.is_empty() {
        return Ok(());
    }

    // if jobs >= 200, split into chunks for avoiding command line length issues
    let chunk_size = 200;
    let chunks: Vec<Vec<String>> = job_ids
        .chunks(chunk_size)
        .map(|chunk| chunk.to_vec())
        .collect();
    for chunk in chunks {
        let _ = execute_command("scancel", chunk).await?;
    }

    Ok(())
}

/// Execute a command to get the tail of a job's output file
pub async fn tail_job_output(file_path: &str, lines: usize) -> Result<String> {
    let args = vec!["-n".to_string(), lines.to_string(), file_path.to_string()];
    let output = execute_command("tail", args).await?;
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    Ok(stdout)
}

/// Parse the output of squeue into a structured format
pub fn parse_squeue_output(output: &str) -> Vec<HashMap<String, String>> {
    let mut result = Vec::new();

    for line in output.lines() {
        if line.trim().is_empty() {
            continue;
        }

        let parts: Vec<&str> = line.split('|').collect();
        if parts.len() < 2 {
            continue;
        }

        let mut job = HashMap::new();

        // Assuming format from our squeue call: JobID|Name|User|State|Time|Nodes|CPUs|Memory|Partition|QOS
        if parts.len() >= 10 {
            job.insert("id".to_string(), parts[0].trim().to_string());
            job.insert("name".to_string(), parts[1].trim().to_string());
            job.insert("user".to_string(), parts[2].trim().to_string());
            job.insert("state".to_string(), parts[3].trim().to_string());
            job.insert("time".to_string(), parts[4].trim().to_string());
            job.insert("nodes".to_string(), parts[5].trim().to_string());
            job.insert("cpus".to_string(), parts[6].trim().to_string());
            job.insert("memory".to_string(), parts[7].trim().to_string());
            job.insert("partition".to_string(), parts[8].trim().to_string());
            job.insert("qos".to_string(), parts[9].trim().to_string());

            result.push(job);
        }
    }

    result
}

/// Execute a command to modify a job (scontrol update)
pub async fn modify_job(job_id: &str, parameters: HashMap<String, String>) -> Result<()> {
    let mut args = vec!["update".to_string(), format!("JobId={}", job_id)];

    for (key, value) in parameters {
        args.push(format!("{}={}", key, value));
    }

    let _ = execute_command("scontrol", args).await?;
    Ok(())
}

/// Get available partitions
pub async fn get_partitions() -> Result<Vec<String>> {
    let output = execute_command(
        "sinfo",
        vec!["-h".to_string(), "-o".to_string(), "%R".to_string()],
    )
    .await?;
    let stdout = String::from_utf8_lossy(&output.stdout);

    let partitions: Vec<String> = stdout
        .lines()
        .map(|line| line.trim().to_string())
        .filter(|line| !line.is_empty())
        .collect();

    Ok(partitions)
}

/// Get available QOS options
pub async fn get_qos() -> Result<Vec<String>> {
    let output = execute_command(
        "sacctmgr",
        vec![
            "-n".to_string(),
            "show".to_string(),
            "qos".to_string(),
            "format=name".to_string(),
        ],
    )
    .await?;

    let stdout = String::from_utf8_lossy(&output.stdout);

    let qos_list: Vec<String> = stdout
        .lines()
        .map(|line| line.trim().to_string())
        .filter(|line| !line.is_empty())
        .collect();

    if qos_list.is_empty() {
        // Fallback to some common values if the command fails
        Ok(vec!["normal".to_string(), "huge".to_string()])
    } else {
        Ok(qos_list)
    }
}
