#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::PathBuf;
use std::process::Command;

const GITHUB_REPO: &str = "Sadoul/rpwlauncher";
const APP_NAME: &str = "RPWorld Launcher";

fn get_install_path() -> PathBuf {
    // Tauri NSIS currentUser installs to %LOCALAPPDATA%\Programs\<ProductName>\<ProductName>.exe
    let local_app_data = std::env::var("LOCALAPPDATA").unwrap_or_default();
    PathBuf::from(&local_app_data)
        .join("Programs")
        .join(APP_NAME)
        .join(format!("{}.exe", APP_NAME))
}

fn launch_app(path: &PathBuf) -> bool {
    match Command::new(path).spawn() {
        Ok(_) => true,
        Err(_) => false,
    }
}

fn show_error(msg: &str) {
    // Use Windows MessageBox via cmd
    let _ = Command::new("cmd")
        .args(["/c", &format!(
            "mshta \"javascript:var sh=new ActiveXObject('WScript.Shell');sh.Popup('{}',0,'RPWorld Launcher',16);close()\"",
            msg
        )])
        .spawn();
}

fn download_and_install() -> bool {
    let temp_dir = std::env::temp_dir();
    let installer_path = temp_dir.join("RPWorld-Launcher-Setup.exe");

    // Step 1: Get latest release info from GitHub API using curl.exe (built into Win10+)
    let api_url = format!(
        "https://api.github.com/repos/{}/releases/latest",
        GITHUB_REPO
    );

    let json_path = temp_dir.join("rpw_release.json");

    let curl_result = Command::new("curl.exe")
        .args([
            "-L",
            "-s",
            "-A", "RPWStub/1.0",
            "-o", json_path.to_str().unwrap_or(""),
            &api_url,
        ])
        .status();

    if curl_result.map(|s| !s.success()).unwrap_or(true) {
        show_error("Не удалось подключиться к серверу. Проверьте интернет соединение.");
        return false;
    }

    // Step 2: Parse JSON to find the NSIS installer URL
    let json_content = match std::fs::read_to_string(&json_path) {
        Ok(c) => c,
        Err(_) => {
            show_error("Ошибка при получении данных о релизе.");
            return false;
        }
    };

    // Simple manual JSON parsing — find browser_download_url for *_x64-setup.exe
    let download_url = extract_download_url(&json_content);
    let _ = std::fs::remove_file(&json_path);

    let download_url = match download_url {
        Some(url) => url,
        None => {
            show_error("Не найден установщик в последнем релизе на GitHub.");
            return false;
        }
    };

    // Step 3: Download the installer
    let curl_dl = Command::new("curl.exe")
        .args([
            "-L",
            "-s",
            "-A", "RPWStub/1.0",
            "--progress-bar",
            "-o", installer_path.to_str().unwrap_or(""),
            &download_url,
        ])
        .status();

    if curl_dl.map(|s| !s.success()).unwrap_or(true) {
        show_error("Ошибка при скачивании установщика.");
        return false;
    }

    if !installer_path.exists() {
        show_error("Файл установщика не найден после скачивания.");
        return false;
    }

    // Step 4: Run the installer silently
    let install_result = Command::new(&installer_path)
        .arg("/S") // NSIS silent install
        .status();

    // Clean up
    let _ = std::fs::remove_file(&installer_path);

    match install_result {
        Ok(s) if s.success() => true,
        _ => {
            // Try without /S (user might need to see installer)
            let _ = Command::new(&installer_path).spawn();
            true
        }
    }
}

fn extract_download_url(json: &str) -> Option<String> {
    // Find all browser_download_url entries and pick the NSIS setup exe
    let mut pos = 0;
    while let Some(idx) = json[pos..].find("browser_download_url") {
        let start = pos + idx;
        // Find the URL value after ": "
        if let Some(colon_idx) = json[start..].find(": \"") {
            let url_start = start + colon_idx + 3;
            if let Some(end_quote) = json[url_start..].find('"') {
                let url = &json[url_start..url_start + end_quote];
                // Prefer NSIS setup exe
                if (url.ends_with("_x64-setup.exe") || url.ends_with("-setup.exe"))
                    && !url.contains("debug")
                {
                    return Some(url.to_string());
                }
                // Fallback to any exe
                if url.ends_with(".exe") && !url.contains("debug") {
                    return Some(url.to_string());
                }
            }
        }
        pos = start + 1;
    }
    None
}

fn main() {
    let install_path = get_install_path();

    if install_path.exists() {
        // Launcher is installed — just run it
        if !launch_app(&install_path) {
            show_error(&format!(
                "Не удалось запустить лаунчер: {}",
                install_path.display()
            ));
        }
    } else {
        // Not installed — download and install
        if download_and_install() {
            // After installation, try to launch
            // Give installer a moment to finish
            std::thread::sleep(std::time::Duration::from_secs(2));
            if install_path.exists() {
                launch_app(&install_path);
            }
        }
    }
}
