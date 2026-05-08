//! DarkSpark Launcher Stub
//! Tiny Windows executable that checks install and launches or downloads the launcher.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::PathBuf;
use std::process::{Command, exit};

#[derive(serde::Deserialize)]
struct GitHubRelease {
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

fn main() {
    let client = match reqwest::blocking::Client::builder()
        .user_agent("DarkSpark-Stub/3.0")
        .timeout(std::time::Duration::from_secs(30))
        .build()
    {
        Ok(c) => c,
        Err(_) => {
            if let Some(path) = find_launcher() {
                launch_installed_launcher(&path);
            }
            exit(0);
        }
    };

    let api_url = format!("https://api.github.com/repos/{}/releases/latest", GITHUB_REPO);

    let release_response = match client.get(&api_url).send() {
        Ok(r) => r,
        Err(e) => {
            show_error(&format!("Не удалось проверить обновление: {}", e));
            launch_if_installed();
            exit(0);
        }
    };

    if !release_response.status().is_success() {
        launch_if_installed();
        exit(0);
    }

    let release: GitHubRelease = match release_response.json() {
        Ok(r) => r,
        Err(_) => {
            launch_if_installed();
            exit(0);
        }
    };

    let installed_version = get_installed_version();
    let launcher_path = find_launcher();

    if let Some(path) = launcher_path {
        launch_installed_launcher(&path);
        exit(0);
    }

    if !installed_version.is_none() {
        exit(0);
    }

    let asset = release.assets.iter().find(|a| {
        let n = a.name.to_lowercase();
        (n.contains("setup") || n.contains("x64")) && n.ends_with(".exe")
            && !n.contains("darkspark-launcher")
    });

    let asset = match asset {
        Some(a) => a,
        None => {
            if let Some(path) = launcher_path {
                launch_installed_launcher(&path);
            }
            exit(0);
        }
    };

    if installed_version.is_none() {
        #[cfg(windows)]
        unsafe {
            use std::ffi::OsStr;
            use std::os::windows::ffi::OsStrExt;
            let msg: Vec<u16> = OsStr::new(
                "DarkSpark Launcher не установлен.\n\
                 Сейчас будет скачан и установлен автоматически.\n\n\
                 Нажмите OK чтобы продолжить.",
            ).encode_wide().chain(Some(0)).collect();
            let caption: Vec<u16> = OsStr::new("DarkSpark Launcher")
                .encode_wide().chain(Some(0)).collect();
            windows_sys::Win32::UI::WindowsAndMessaging::MessageBoxW(
                0, msg.as_ptr(), caption.as_ptr(),
                windows_sys::Win32::UI::WindowsAndMessaging::MB_OK
                    | windows_sys::Win32::UI::WindowsAndMessaging::MB_ICONINFORMATION,
            );
        }
    }

    let temp_dir = std::env::temp_dir();
    let installer_path = temp_dir.join(&asset.name);

    let download_response = match client.get(&asset.browser_download_url).send() {
        Ok(r) => r,
        Err(_) => {
            launch_if_installed();
            exit(0);
        }
    };

    if !download_response.status().is_success() {
        launch_if_installed();
        exit(0);
    }

    let bytes = match download_response.bytes() {
        Ok(b) => b,
        Err(_) => {
            launch_if_installed();
            exit(0);
        }
    };

    if std::fs::write(&installer_path, &bytes).is_err() {
        if let Some(path) = launcher_path {
            launch_installed_launcher(&path);
        }
        exit(0);
    }

    let status = Command::new(&installer_path)
        .args(["/S"])
        .spawn()
        .and_then(|mut c| c.wait());

    let _ = std::fs::remove_file(&installer_path);

    let wait_ms = if status.is_ok() { 3000 } else { 1000 };
    std::thread::sleep(std::time::Duration::from_millis(wait_ms));

    if let Some(path) = find_launcher() {
        launch_installed_launcher(&path);
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
    let _ = Command::new(path).spawn();
}

fn close_running_launcher() {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        let _ = Command::new("taskkill")
            .args(["/IM", EXE_NAME, "/F", "/T"])
            .creation_flags(CREATE_NO_WINDOW)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .stdin(std::process::Stdio::null())
            .status();
    }
}

fn get_installed_version() -> Option<String> {
    #[cfg(windows)]
    {
        use winreg::enums::*;
        use winreg::RegKey;
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        if let Ok(key) = hkcu.open_subkey(REG_KEY) {
            if let Ok(ver) = key.get_value::<String, _>("DisplayVersion") {
                let v = ver.trim().trim_start_matches('v').to_string();
                if !v.is_empty() {
                    return Some(v);
                }
            }
        }
    }
    None
}

fn find_launcher() -> Option<PathBuf> {
    #[cfg(windows)]
    {
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
    #[cfg(windows)]
    unsafe {
        use std::ffi::OsStr;
        use std::os::windows::ffi::OsStrExt;
        let msg_w: Vec<u16> = OsStr::new(msg).encode_wide().chain(Some(0)).collect();
        let cap_w: Vec<u16> = OsStr::new("DarkSpark Launcher")
            .encode_wide().chain(Some(0)).collect();
        windows_sys::Win32::UI::WindowsAndMessaging::MessageBoxW(
            0, msg_w.as_ptr(), cap_w.as_ptr(),
            windows_sys::Win32::UI::WindowsAndMessaging::MB_OK
                | windows_sys::Win32::UI::WindowsAndMessaging::MB_ICONERROR,
        );
    }
    #[cfg(not(windows))]
    eprintln!("{}", msg);
}
