use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::Once;

static LOGGING_ENABLED: Mutex<bool> = Mutex::new(true);
static CLEANUP_ONCE: Once = Once::new();

fn log_path() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".danganverse")
        .join("launcher.log")
}

fn last_cleanup_marker() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".danganverse")
        .join("log_cleanup.txt")
}

/// Truncates the launcher log file once per UTC calendar day.
/// Marker file stores the last cleanup date as YYYY-MM-DD; on first
/// log() call of a new day we wipe the log and update the marker.
fn maybe_daily_cleanup() {
    CLEANUP_ONCE.call_once(|| {
        let today = chrono::Local::now().format("%Y-%m-%d").to_string();
        let marker = last_cleanup_marker();
        let last = fs::read_to_string(&marker)
            .ok()
            .map(|s| s.trim().to_string())
            .unwrap_or_default();

        if last != today {
            let log = log_path();
            if log.exists() {
                let _ = fs::write(&log, format!("[{}] === Daily log rotation: previous log cleared ===\n", chrono::Local::now().format("%Y-%m-%d %H:%M:%S")));
            }
            if let Some(parent) = marker.parent() {
                let _ = fs::create_dir_all(parent);
            }
            let _ = fs::write(&marker, &today);
        }
    });
}

pub fn log(msg: &str) {
    let enabled = LOGGING_ENABLED.lock().map(|g| *g).unwrap_or(false);
    if !enabled {
        return;
    }

    maybe_daily_cleanup();

    let path = log_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(&path) {
        let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
        let _ = writeln!(f, "[{timestamp}] {msg}");
    }
}

#[tauri::command]
pub fn set_logging_enabled(enabled: bool) {
    if let Ok(mut g) = LOGGING_ENABLED.lock() {
        *g = enabled;
    }
    log(&format!("Logging {}", if enabled { "enabled" } else { "disabled" }));
}

#[tauri::command]
pub fn get_log() -> String {
    fs::read_to_string(log_path()).unwrap_or_default()
}

#[tauri::command]
pub fn clear_log() {
    let _ = fs::write(log_path(), "");
}

#[tauri::command]
pub fn get_log_path() -> String {
    log_path().to_string_lossy().to_string()
}