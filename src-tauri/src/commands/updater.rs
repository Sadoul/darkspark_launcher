use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
#[cfg(windows)]
use std::os::windows::process::CommandExt;
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;
#[cfg(windows)]
const CREATE_BREAKAWAY_FROM_JOB: u32 = 0x01000000;
use tauri::Emitter;

use super::logger::log as launcher_log;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UpdateInfo {
    pub current_version: String,
    pub latest_version: String,
    pub update_available: bool,
    pub download_url: String,
    pub installer_url: String,
    pub release_notes: String,
    pub file_size: u64,
}

#[derive(Debug, Serialize, Clone)]
pub struct UpdateProgress {
    pub stage: String,
    pub downloaded: u64,
    pub total: u64,
    pub speed_kb: u64,
    pub message: String,
}

const GITHUB_REPO: &str = "Sadoul/darkspark_launcher";
const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

fn data_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".danganverse")
}


fn marker_path() -> PathBuf {
    data_dir().join("update_marker")
}


#[tauri::command]
pub fn check_just_updated() -> bool {
    let path = marker_path();
    if !path.exists() {
        return false;
    }

    let content = fs::read_to_string(&path).unwrap_or_default();
    let _ = fs::remove_file(&path);


    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    if let Some(ts_str) = content.split(':').nth(1) {
        if let Ok(ts) = ts_str.parse::<u64>() {
            let age_secs = now.saturating_sub(ts);
            launcher_log(&format!("[updater] Found update marker, age: {}s", age_secs));
            if age_secs < 300 {
                launcher_log("[updater] Marker is fresh — skipping update check this run");
                return true;
            }
            launcher_log("[updater] Marker is stale (>5min) — ignoring, will check for updates");
            return false;
        }
    }


    launcher_log("[updater] Found old-format marker without timestamp — ignoring");
    false
}

fn update_log_path() -> PathBuf {
    std::env::temp_dir().join("darkspark_update.log")
}

fn update_log(message: &str) {
    let line = format!("[{}] {}\r\n", chrono::Local::now().format("%Y-%m-%d %H:%M:%S"), message);
    let _ = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(update_log_path())
        .and_then(|mut file| file.write_all(line.as_bytes()));
    launcher_log(message);
}

fn write_update_marker() {
    let dir = data_dir();
    let _ = fs::create_dir_all(&dir);
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let content = format!("{}:{}", CURRENT_VERSION, ts);
    let _ = fs::write(marker_path(), content);
}

fn compare_versions(a: &str, b: &str) -> std::cmp::Ordering {
    let parse = |v: &str| {
        v.trim_start_matches('v')
            .split('.')
            .filter_map(|s| s.parse::<u32>().ok())
            .collect::<Vec<_>>()
    };
    parse(a).cmp(&parse(b))
}


#[tauri::command]
pub async fn check_launcher_update() -> Result<UpdateInfo, String> {
    launcher_log(&format!("[updater] Checking for updates. Current version: {}", CURRENT_VERSION));

    let client = reqwest::Client::builder()
        .user_agent("DanganVerseLauncher/1.0")
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| e.to_string())?;

    let api_url = format!(
        "https://api.github.com/repos/{}/releases/latest",
        GITHUB_REPO
    );

    launcher_log(&format!("[updater] Fetching: {}", api_url));

    let response = client
        .get(&api_url)
        .send()
        .await
        .map_err(|e| {
            let msg = format!("[updater] Network error: {}", e);
            launcher_log(&msg);
            msg
        })?;

    let status = response.status();
    launcher_log(&format!("[updater] GitHub API response: {}", status));

    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        let msg = format!(
            "[updater] GitHub API returned {}. Private repo token is missing/expired/forbidden. Body: {}",
            status,
            body
        );
        launcher_log(&msg);
        return Err(msg);
    }

    let release: serde_json::Value = response.json().await.map_err(|e| {
        let msg = format!("[updater] JSON parse error: {}", e);
        launcher_log(&msg);
        msg
    })?;

    let tag = release["tag_name"]
        .as_str()
        .unwrap_or(CURRENT_VERSION)
        .to_string();
    let latest_clean = tag.trim_start_matches('v').to_string();
    launcher_log(&format!("[updater] Latest release tag: {} (clean: {})", tag, latest_clean));

    let assets = release["assets"].as_array().cloned().unwrap_or_default();
    launcher_log(&format!("[updater] Release has {} assets", assets.len()));

    let mut installer_url = String::new();
    let mut file_size: u64 = 0;

    for asset in &assets {
        let name = asset["name"].as_str().unwrap_or("");
        let url = asset["browser_download_url"]
            .as_str()
            .unwrap_or("")
            .to_string();
        let size = asset["size"].as_u64().unwrap_or(0);
        launcher_log(&format!("[updater] Asset: {} ({} bytes)", name, size));

        if (name.ends_with("_x64-setup.exe") || name.ends_with("-setup.exe"))
            && !name.contains("debug")
        {
            installer_url = url;
            file_size = size;
            launcher_log(&format!("[updater] Selected installer: {}", name));
        }
    }

    let raw_notes = release["body"].as_str().unwrap_or("").trim();
    let release_notes = if raw_notes.is_empty() || raw_notes.contains("Full Changelog") || raw_notes.contains("github.com/Sadoul/darkspark_launcher/compare/") {
        format!("Обновление лаунчера до версии v{}", latest_clean)
    } else {
        raw_notes.to_string()
    };
    let version_cmp = compare_versions(&latest_clean, CURRENT_VERSION);
    let update_available = !installer_url.is_empty()
        && version_cmp == std::cmp::Ordering::Greater;

    launcher_log(&format!(
        "[updater] Version comparison: {} vs {} => {:?} | installer_found={} | update_available={}",
        latest_clean, CURRENT_VERSION, version_cmp, !installer_url.is_empty(), update_available
    ));

    Ok(UpdateInfo {
        current_version: CURRENT_VERSION.to_string(),
        latest_version: latest_clean,
        update_available,
        download_url: installer_url.clone(),
        installer_url,
        release_notes,
        file_size,
    })
}

#[tauri::command]
pub async fn update_launcher(app: tauri::AppHandle) -> Result<String, String> {
    let info = check_launcher_update().await?;

    if !info.update_available {
        return Ok("no_update".to_string());
    }

    let app_ref = app.clone();
    let emit = move |stage: &str, downloaded: u64, total: u64, speed: u64, msg: &str| {
        let _ = app_ref.emit(
            "update-progress",
            UpdateProgress {
                stage: stage.to_string(),
                downloaded,
                total,
                speed_kb: speed,
                message: msg.to_string(),
            },
        );
    };

    update_log(&format!("[updater] Starting in-app update {} -> {}", info.current_version, info.latest_version));
    emit("downloading", 0, info.file_size, 0, "Начало скачивания...");

    let client = reqwest::Client::builder()
        .user_agent("DanganVerseLauncher/1.0")
        .build()
        .map_err(|e| e.to_string())?;

    let response = client
        .get(&info.installer_url)
        .send()
        .await
        .map_err(|e| format!("Ошибка скачивания: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Ошибка скачивания обновления: HTTP {}", response.status()));
    }

    let total = response.content_length().unwrap_or(info.file_size);
    let temp_dir = std::env::temp_dir();
    let download_path = temp_dir.join(format!("darkspark-setup-{}.exe", info.latest_version));
    update_log(&format!("[updater] Download target: {}", download_path.display()));

    use futures_util::StreamExt;
    let mut stream = response.bytes_stream();
    let mut downloaded: u64 = 0;
    let mut file = fs::File::create(&download_path).map_err(|e| e.to_string())?;
    let start_time = std::time::Instant::now();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| e.to_string())?;
        file.write_all(&chunk).map_err(|e| e.to_string())?;
        downloaded += chunk.len() as u64;

        let elapsed = start_time.elapsed().as_secs_f64();
        let speed_kb = if elapsed > 0.1 {
            (downloaded as f64 / elapsed / 1024.0) as u64
        } else {
            0
        };

        let mb_done = downloaded as f64 / 1_048_576.0;
        let mb_total = total as f64 / 1_048_576.0;
        emit(
            "downloading",
            downloaded,
            total,
            speed_kb,
            &format!("Скачивание... {:.1}/{:.1} МБ", mb_done, mb_total),
        );
    }
    drop(file);

    update_log(&format!("[updater] Download finished: {} bytes", downloaded));
    emit("applying", total, total, 0, "Установка обновления...");


    tokio::time::sleep(std::time::Duration::from_millis(800)).await;


    write_update_marker();

    apply_nsis_update(app, &download_path)?;

    Ok("update_started".to_string())
}

fn apply_nsis_update(app: tauri::AppHandle, installer: &PathBuf) -> Result<(), String> {
    update_log(&format!("[updater] Starting NSIS installer directly: {}", installer.display()));

    let mut installer_command = Command::new(installer);
    installer_command
        .arg("/S")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    #[cfg(windows)]
    installer_command.creation_flags(CREATE_NO_WINDOW | CREATE_BREAKAWAY_FROM_JOB);

    let installer_child = installer_command
        .spawn()
        .map_err(|e| format!("Не удалось запустить установщик обновления: {e}"))?;

    let installer_pid = installer_child.id();
    update_log(&format!("[updater] NSIS installer started PID={}, scheduling relaunch after it finishes", installer_pid));

    // Wait for NSIS to finish, then relaunch — same pattern as game watcher.
    #[cfg(windows)]
    if let Ok(exe) = std::env::current_exe() {
        let script = format!(
            "Wait-Process -Id {} -ErrorAction SilentlyContinue; Start-Sleep -Seconds 1; Start-Process -FilePath '{}'",
            installer_pid,
            exe.to_string_lossy().replace('\'', "''")
        );
        let _ = Command::new("powershell.exe")
            .args(["-NoProfile", "-NonInteractive", "-WindowStyle", "Hidden", "-Command", &script])
            .creation_flags(CREATE_NO_WINDOW | CREATE_BREAKAWAY_FROM_JOB)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();
        update_log(&format!("[updater] Relaunch watcher started for PID={}: {}", installer_pid, exe.display()));
    }

    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
        app.exit(0);
    });

    Ok(())
}