//! DarkSpark Launcher Stub
//! Tiny Windows executable (~400 KB) that:
//!   1. Checks GitHub for the latest release version
//!   2. If launcher not installed -> downloads NSIS installer silently
//!   3. Launches the (freshly) installed launcher

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::os::windows::process::CommandExt;
use std::path::PathBuf;
use std::process::{Command, exit};

#[derive(serde::Deserialize)]
struct GitHubRelease {
    tag_name: String,
    assets: Vec<GitHubAsset>,
}

#[derive(serde::Deserialize)]
struct GitHubAsset {
    name: String,
    browser_download_url: String,
}

const GITHUB_REPO: &str = "Sadoul/darkspark_launcher";
const EXE_NAME:    &str = "darkspark-launcher.exe";
const INSTALL_DIR: &str = "DarkSpark Launcher";
const REG_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Uninstall\DarkSpark Launcher";
const STUB_VERSION: &str = "2.70.7";

fn log(msg: &str) {
    let path = std::env::temp_dir().join("darkspark_stub.log");
    let line = format!("[{}] {}\r\n", chrono::Local::now().format("%H:%M:%S"), msg);
    let _ = std::fs::OpenOptions::new().create(true).append(true).open(&path)
        .and_then(|mut f| std::io::Write::write_all(&mut f, line.as_bytes()));
}

fn main() {
    log(&format!("DarkSpark-Stub v{} started", STUB_VERSION));

    let client = match reqwest::blocking::Client::builder()
        .user_agent("DarkSpark-Stub/2.70.7")
        .timeout(std::time::Duration::from_secs(30))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            log(&format!("HTTP client error: {}", e));
            launch_if_installed();
            exit(0);
        }
    };

    // 1. Fetch latest release from GitHub
    let api_url = format!("https://api.github.com/repos/{}/releases/latest", GITHUB_REPO);
    log(&format!("Fetching: {}", api_url));

    let release_response = match client.get(&api_url).send() {
        Ok(r) => r,
        Err(e) => {
            log(&format!("Failed to fetch release: {}", e));
            show_error("Не удалось проверить обновление на GitHub.\nПроверьте интернет-соединение.");
            launch_if_installed();
            exit(0);
        }
    };

    let status = release_response.status();
    log(&format!("GitHub API response: {}", status));

    if !status.is_success() {
        show_error(&format!("GitHub вернул ошибку: {}\nЛаунчер будет запущен.", status));
        launch_if_installed();
        exit(0);
    }

    let release: GitHubRelease = match release_response.json() {
        Ok(r) => r,
        Err(e) => {
            log(&format!("Failed to parse release JSON: {}", e));
            launch_if_installed();
            exit(0);
        }
    };

    log(&format!("Latest release tag: {}", release.tag_name));

    // 2. Check if launcher is installed
    let launcher_path = find_launcher();

    if let Some(path) = &launcher_path {
        log(&format!("Launcher found at: {:?}", path));
        launch_installed_launcher(path);
        exit(0);
    }

    log("Launcher not installed, will download installer");

    // 3. Find NSIS installer in assets
    let asset = release.assets.iter().find(|a| {
        let n = a.name.to_lowercase();
        n.contains("setup") && n.ends_with(".exe") && !n.contains("DarkSpark-Launcher")
    });

    let (asset_name, download_url) = match asset {
        Some(a) => {
            log(&format!("Found installer: {}", a.name));
            (a.name.clone(), a.browser_download_url.clone())
        }
        None => {
            log("No NSIS installer found in release assets");
            show_error("Не удалось найти установщик в релизе.\nСкачайте лаунчер вручную с GitHub.");
            exit(0);
        }
    };

    // 4. Download installer
    log(&format!("Downloading: {}", download_url));
    show_info("DarkSpark Launcher будет скачан и установлен.\nНажмите OK для продолжения.");

    let download_response = match client.get(&download_url).send() {
        Ok(r) => r,
        Err(e) => {
            log(&format!("Download failed: {}", e));
            show_error(&format!("Ошибка скачивания: {}\nПроверьте интернет-соединение.", e));
            exit(0);
        }
    };

    if !download_response.status().is_success() {
        log(&format!("Download HTTP error: {}", download_response.status()));
        show_error(&format!("Ошибка скачивания: HTTP {}", download_response.status()));
        exit(0);
    }

    log("Download OK, saving installer");

    let bytes = match download_response.bytes() {
        Ok(b) => b,
        Err(e) => {
            log(&format!("Failed to read bytes: {}", e));
            show_error(&format!("Ошибка чтения файла: {}", e));
            exit(0);
        }
    };

    log(&format!("Downloaded {} bytes", bytes.len()));

    let temp_dir = std::env::temp_dir();
    let installer_path = temp_dir.join(&asset_name);

    if let Err(e) = std::fs::write(&installer_path, &bytes) {
        log(&format!("Failed to save installer: {}", e));
        show_error(&format!("Не удалось сохранить установщик: {}", e));
        exit(0);
    }

    log(&format!("Installer saved to: {:?}", installer_path));

    // 5. Run NSIS installer silently
    log("Running NSIS installer...");
    let status = Command::new(&installer_path)
        .args(["/S"])
        .creation_flags(0x08000000)
        .spawn()
        .and_then(|mut c: std::process::Child| c.wait());

    let _ = std::fs::remove_file(&installer_path);

    let wait_ms = if status.is_ok() { 3000 } else { 1000 };
    std::thread::sleep(std::time::Duration::from_millis(wait_ms));

    if let Some(path) = find_launcher() {
        log(&format!("Launching: {:?}", path));
        launch_installed_launcher(&path);
    } else {
        log("Launcher not found after install!");
        show_error("Лаунчер не был установлен.\nПопробуйте снова или скачайте вручную.");
    }
}

fn launch_if_installed() {
    if let Some(path) = find_launcher() {
        launch_installed_launcher(&path);
    }
}

fn launch_installed_launcher(path: &PathBuf) {
    close_running_launcher();
    std::thread::sleep(std::time::Duration::from_millis(900));
    log(&format!("Launching: {:?}", path));
    let _ = Command::new(path).spawn();
}

fn close_running_launcher() {
    let _ = Command::new("taskkill")
        .args(["/IM", EXE_NAME, "/F", "/T"])
        .creation_flags(0x08000000)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .stdin(std::process::Stdio::null())
        .status();
}

fn find_launcher() -> Option<PathBuf> {
    use winreg::enums::*;
    use winreg::RegKey;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);

    if let Ok(key) = hkcu.open_subkey(REG_KEY) {
        if let Ok(raw) = key.get_value::<String, _>("InstallLocation") {
            let dir = raw.trim_matches('"');
            let exe = PathBuf::from(dir).join(EXE_NAME);
            if exe.exists() {
                return Some(exe);
            }
        }
    }

    let candidates = [
        dirs::data_local_dir().map(|d| d.join(INSTALL_DIR).join(EXE_NAME)),
        dirs::data_local_dir().map(|d| d.join("Programs").join(INSTALL_DIR).join(EXE_NAME)),
    ];
    for candidate in candidates.into_iter().flatten() {
        if candidate.exists() {
            return Some(candidate);
        }
    }
    None
}

fn show_error(msg: &str) {
    log(&format!("ERROR: {}", msg));
    #[cfg(windows)]
    unsafe {
        use std::ffi::OsStr;
        use std::os::windows::ffi::OsStrExt;
        let msg_w: Vec<u16> = OsStr::new(msg).encode_wide().chain(Some(0)).collect();
        let cap_w: Vec<u16> = OsStr::new("DarkSpark Launcher").encode_wide().chain(Some(0)).collect();
        windows_sys::Win32::UI::WindowsAndMessaging::MessageBoxW(
            0, msg_w.as_ptr(), cap_w.as_ptr(),
            windows_sys::Win32::UI::WindowsAndMessaging::MB_OK
                | windows_sys::Win32::UI::WindowsAndMessaging::MB_ICONERROR,
        );
    }
}

fn show_info(msg: &str) {
    #[cfg(windows)]
    unsafe {
        use std::ffi::OsStr;
        use std::os::windows::ffi::OsStrExt;
        let msg_w: Vec<u16> = OsStr::new(msg).encode_wide().chain(Some(0)).collect();
        let cap_w: Vec<u16> = OsStr::new("DarkSpark Launcher").encode_wide().chain(Some(0)).collect();
        windows_sys::Win32::UI::WindowsAndMessaging::MessageBoxW(
            0, msg_w.as_ptr(), cap_w.as_ptr(),
            windows_sys::Win32::UI::WindowsAndMessaging::MB_OK
                | windows_sys::Win32::UI::WindowsAndMessaging::MB_ICONINFORMATION,
        );
    }
}