use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;

static LOGGING_ENABLED: Mutex<bool> = Mutex::new(true);

fn log_path() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".darkspark")
        .join("launcher.log")
}

pub fn log(msg: &str) {
    let enabled = LOGGING_ENABLED.lock().map(|g| *g).unwrap_or(false);
    if !enabled {
        return;
    }

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