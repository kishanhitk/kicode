use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

const CHECK_INTERVAL_SECS: u64 = 24 * 60 * 60; // 24 hours

/// Get the path to the state file that stores the last update check timestamp
fn state_file_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("kicode")
        .join(".last_update_check")
}

/// Check if we should perform an update check (more than 24 hours since last check)
pub fn should_check() -> bool {
    let path = state_file_path();

    let last_check = match std::fs::read_to_string(&path) {
        Ok(content) => content.trim().parse::<u64>().unwrap_or(0),
        Err(_) => return true, // No state file - should check
    };

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    now.saturating_sub(last_check) >= CHECK_INTERVAL_SECS
}

/// Record that an update check was performed
pub fn record_check() {
    let path = state_file_path();

    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    // Silent failure - don't interrupt user experience
    let _ = std::fs::write(&path, now.to_string());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_file_path() {
        let path = state_file_path();
        assert!(path.ends_with(".last_update_check"));
        assert!(path.to_string_lossy().contains("kicode"));
    }
}
