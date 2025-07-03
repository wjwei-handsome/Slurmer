pub mod command;
pub mod event;
pub mod file_watcher;

/// Returns the current username from the environment
pub fn get_username() -> String {
    std::env::var("USER").unwrap_or_else(|_| "unknown".to_string())
}

/// Truncate a string to a maximum length, appending an ellipsis if truncated
pub fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}

/// Format memory size to a human-readable string
pub fn format_memory(memory_mb: u64) -> String {
    if memory_mb < 1024 {
        format!("{}M", memory_mb)
    } else {
        format!("{:.1}G", memory_mb as f64 / 1024.0)
    }
}

/// Format time duration in a human-readable format
pub fn format_duration(seconds: u64) -> String {
    let days = seconds / (24 * 3600);
    let hours = (seconds % (24 * 3600)) / 3600;
    let minutes = (seconds % 3600) / 60;
    let secs = seconds % 60;

    if days > 0 {
        format!("{}d {:02}:{:02}:{:02}", days, hours, minutes, secs)
    } else {
        format!("{:02}:{:02}:{:02}", hours, minutes, secs)
    }
}
