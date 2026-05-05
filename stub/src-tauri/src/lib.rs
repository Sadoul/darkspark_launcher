use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::PathBuf;
use tauri::{AppHandle, Emitter};

const GITHUB_REPO: &str = "Sadoul/rpwlauncher";
const INSTALLER_ASSET_SUFFIX: &str = "_x64-setup.exe";

/// Possible binary names Tauri NSIS may install under.
/// Tauri uses the Cargo package binary name ("rpw-launcher") as the .exe name inside the bundle,
/// but we also check the productName variant just in case.
const EXE_NAMES: &[&str] = &["rpw-launcher.exe", "RPWorld Launcher.exe", "RPWorld-Launcher.exe"];

/// Returns the install location from the Windows registry if available.
#[cfg(windows)]
fn install_location_from_registry() -> Option<PathBuf> {
    // Tauri v2 NSIS currentUser mode registers under:
    //   HKCU\Software\Microsoft\Windows\CurrentVersion\Uninstall\{identifier}_is1
    // identifier = "com.rpworld.launcher"
    let keys = [
        r"Software\Microsoft\Windows\CurrentVersion\Uninstall\com.rpworld.launcher_is1",
        r"Software\Microsoft\Windows\CurrentVersion\Uninstall\RPWorld Launcher_is1",
    ];

    use winreg::enums::*;
    use winreg::RegKey;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    for key_path in &keys {
        if let Ok(key) = hkcu.open_subkey(key_path) {
            if let Ok(loc) = key.get_value::<String, _>("InstallLocation") {
                let path = PathBuf::from(&loc);
                if path.exists() {
                    return Some(path);
                }
            }
        }
    }
    None
}

#[cfg(not(windows))]
fn install_location_from_registry() -> Option<PathBuf> {
    None
}

/// Find the installed launcher exe.
fn get_launcher_exe() -> Option<PathBuf> {
    // 1. Check registry for install location, then look for known exe names
    if let Some(install_dir) = install_location_from_registry() {
        for name in EXE_NAMES {
            let candidate = install_dir.join(name);
            if candidate.exists() {
                return Some(candidate);
            }
        }
        // Fallback: scan the dir for any .exe that isn't an uninstaller
        if let Ok(entries) = std::fs::read_dir(&install_dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.extension().map(|e| e == "exe").unwrap_or(false) {
                    let fname = p.file_name().unwrap_or_default().to_string_lossy().to_lowercase();
                    if !fname.contains("uninstall") {
                        return Some(p);
                    }
                }
            }
        }
    }

    // 2. Standard Tauri NSIS currentUser install path candidates
    let mut candidates: Vec<PathBuf> = Vec::new();

    let local = dirs::data_local_dir().unwrap_or_else(|| PathBuf::from("C:\\Users\\Default\\AppData\\Local"));

    for name in EXE_NAMES {
        candidates.push(local.join("Programs").join("RPWorld Launcher").join(name));
        candidates.push(local.join("Programs").join("com.rpworld.launcher").join(name));
        candidates.push(local.join("RPWorld Launcher").join(name));
    }

    // 3. Same directory as the stub itself
    if let Ok(stub_dir) = std::env::current_exe().map(|p| p.parent().map(|d| d.to_path_buf()).unwrap_or_default()) {
        for name in EXE_NAMES {
            candidates.push(stub_dir.join(name));
        }
    }

    for path in candidates {
        if path.exists() {
            return Some(path);
        }
    }

    None
}

#[tauri::command]
fn check_installed() -> bool {
    get_launcher_exe().is_some()
}

#[tauri::command]
async fn launch_launcher(app: AppHandle) -> Result<(), String> {
    let exe = get_launcher_exe().ok_or_else(|| {
        // Give a helpful diagnostic message including what we searched
        let local = dirs::data_local_dir()
            .map(|d| d.display().to_string())
            .unwrap_or_else(|| "%LOCALAPPDATA%".to_string());
        format!(
            "RPWorld Launcher не ��айден. Ожидаемый путь: {}\\Programs\\RPWorld Launcher\\rpw-launcher.exe",
            local
        )
    })?;

    std::process::Command::new(&exe)
        .spawn()
        .map_err(|e| format!("Не у��алось запустить: {e}"))?;

    app.exit(0);
    Ok(())
}

#[derive(Clone, Serialize)]
struct ProgressPayload {
    percent: u32,
    status: String,
    downloaded: Option<String>,
    speed: Option<String>,
}

#[derive(Deserialize)]
struct GithubRelease {
    assets: Vec<GithubAsset>,
}

#[derive(Deserialize)]
struct GithubAsset {
    name: String,
    browser_download_url: String,
    size: u64,
}

fn emit_progress(app: &AppHandle, percent: u32, status: &str, downloaded: Option<String>, speed: Option<String>) {
    let _ = app.emit("install-progress", ProgressPayload {
        percent,
        status: status.to_string(),
        downloaded,
        speed,
    });
}

#[tauri::command]
async fn download_and_install(app: AppHandle) -> Result<(), String> {
    emit_progress(&app, 10, "Получение информации о версии...", None, None);

    let client = reqwest::Client::builder()
        .user_agent("RPWorld-Stub/1.0")
        .build()
        .map_err(|e| e.to_string())?;

    let url = format!("https://api.github.com/repos/{GITHUB_REPO}/releases/latest");
    let release: GithubRelease = client
        .get(&url)
        .send()
        .await
        .map_err(|_| "Не удалось подключиться к GitHub. Проверьте интернет-соединение.")?
        .json()
        .await
        .map_err(|e| format!("Ошибка разбора ответа: {e}"))?;

    let asset = release
        .assets
        .iter()
        .find(|a| a.name.ends_with(INSTALLER_ASSET_SUFFIX))
        .ok_or("Установщик не найден в последнем релизе")?;

    emit_progress(&app, 15, "Начало загрузки...", None, None);

    let download_url = &asset.browser_download_url;
    let total_size = asset.size;
    let asset_name = &asset.name;

    let temp_dir = std::env::temp_dir();
    let dest_path = temp_dir.join(asset_name);

    let response = client
        .get(download_url)
        .send()
        .await
        .map_err(|e| format!("Ошибка загрузки: {e}"))?;

    let mut file = std::fs::File::create(&dest_path)
        .map_err(|e| format!("Не удалось создать файл: {e}"))?;

    let mut downloaded: u64 = 0;
    let mut stream = response.bytes_stream();
    let start_time = std::time::Instant::now();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| format!("Ошибка загрузки: {e}"))?;
        file.write_all(&chunk)
            .map_err(|e| format!("Ошибка записи: {e}"))?;
        downloaded += chunk.len() as u64;

        let percent = if total_size > 0 {
            15 + ((downloaded as f64 / total_size as f64) * 70.0) as u32
        } else {
            50
        };

        let elapsed = start_time.elapsed().as_secs_f64();
        let speed = if elapsed > 0.5 {
            let mbs = (downloaded as f64 / elapsed) / (1024.0 * 1024.0);
            Some(format!("{:.1} МБ/с", mbs))
        } else {
            None
        };

        let dl_mb = downloaded as f64 / (1024.0 * 1024.0);
        let total_mb = total_size as f64 / (1024.0 * 1024.0);
        let dl_str = if total_size > 0 {
            Some(format!("{:.1} / {:.1} МБ", dl_mb, total_mb))
        } else {
            Some(format!("{:.1} МБ", dl_mb))
        };

        emit_progress(
            &app,
            percent.min(85),
            "Загрузка RPWorld Launcher...",
            dl_str,
            speed,
        );
    }

    drop(file);

    emit_progress(&app, 87, "Запуск установщика...", None, None);

    // Run NSIS installer silently
    let status = std::process::Command::new(&dest_path)
        .args(["/S"])
        .status()
        .map_err(|e| format!("Не удалось запустить установщик: {e}"))?;

    // Clean up
    let _ = std::fs::remove_file(&dest_path);

    if !status.success() {
        return Err(format!(
            "Установщик завершился с ошибкой: {}",
            status.code().unwrap_or(-1)
        ));
    }

    // Small delay to let NSIS finish writing files to disk
    tokio::time::sleep(std::time::Duration::from_millis(1500)).await;

    emit_progress(&app, 97, "Установка завершена!", None, None);
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            check_installed,
            launch_launcher,
            download_and_install
        ])
        .run(tauri::generate_context!())
        .expect("error running stub");
}
