use async_process::{Command, Output};
use color_eyre::Result;
use std::collections::HashMap;

/// Execute a Slurm command asynchronously and return the output
pub async fn execute_command(cmd: &str, args: Vec<String>) -> Result<Output> {
    let output = Command::new(cmd).args(args).output().await?;

    Ok(output)
}

/// Execute the squeue command to get job information
pub async fn _execute_squeue(args: Vec<String>) -> Result<String> {
    let output = execute_command("squeue", args).await?;
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    Ok(stdout)
}

/// Execute the scontrol command to get detailed job information
pub async fn _execute_scontrol(job_id: &str) -> Result<String> {
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

/// Execute a command to modify a job (scontrol update)
pub async fn _modify_job(job_id: &str, parameters: HashMap<String, String>) -> Result<()> {
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
