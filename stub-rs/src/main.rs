//! DanganVerse Launcher Stub — single exe bootstrap
//! Checks if launcher is installed, installs via NSIS if not, then launches it.
//! On subsequent runs: just launches the already-installed launcher.

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
#[allow(dead_code)]
const STUB_ASSET_NAME: &str = "DanganVerse-Launcher.exe";
const LAUNCHER_EXE: &str = "danganverse-launcher.exe";
const LAUNCHER_PRODUCT_NAME: &str = "DanganVerse Launcher";
const REG_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Uninstall\DanganVerse Launcher";
const STUB_VERSION: &str = env!("CARGO_PKG_VERSION");

const CREATE_NO_WINDOW: u32 = 0x08000000;
#[allow(dead_code)]
const DETACHED_PROCESS: u32 = 0x00000008;

fn log(msg: &str) {
    let path = std::env::temp_dir().join("danganverse_stub.log");
    let line = format!("[{}] {}\r\n", chrono::Local::now().format("%H:%M:%S"), msg);
    let _ = std::fs::OpenOptions::new().create(true).append(true).open(&path)
        .and_then(|mut f| std::io::Write::write_all(&mut f, line.as_bytes()));
}

fn main() {
    log(&format!("DanganVerse-Stub v{} starting", STUB_VERSION));

    // Fast path: launcher already installed — launch immediately, no network needed.
    if let Some(path) = find_launcher() {
        log(&format!("Launcher found (fast path): {:?}", path));
        close_running_launcher();
        std::thread::sleep(std::time::Duration::from_millis(300));
        let _ = Command::new(&path).spawn();
        exit(0);
    }

    // Launcher not installed — fetch release and install it.
    let client = match reqwest::blocking::Client::builder()
        .user_agent("DanganVerse-Stub/1.0")
        .timeout(std::time::Duration::from_secs(8))
        .build()
    {
        Ok(c) => c,
        Err(_) => { exit(1); }
    };

    let api_url = format!("https://api.github.com/repos/{}/releases/latest", GITHUB_REPO);
    let release = match fetch_release(&client, &api_url) {
        Some(r) => r,
        None => {
            show_error("Не удалось подключиться к GitHub. Проверьте интернет.");
            exit(1);
        }
    };

    log(&format!("Release: {}", release.tag_name));

    // 4. Launcher NOT installed — find the NSIS setup exe in release assets and run it silently
    log("Launcher not installed — looking for NSIS installer in release assets");
    let installer_asset = release.assets.iter().find(|a| {
        let n = a.name.to_lowercase();
        (n.ends_with("_x64-setup.exe") || n.ends_with("-setup.exe")) && !n.contains("debug")
    });

    if let Some(asset) = installer_asset {
        log(&format!("Downloading NSIS installer: {}", asset.name));
        if let Some(tmp) = download_file(&client, &asset.browser_download_url, "DanganVerse-Setup.exe") {
            log("Running NSIS installer silently...");
            let status = Command::new(&tmp)
                .arg("/S")
                .creation_flags(CREATE_NO_WINDOW)
                .status();
            let _ = std::fs::remove_file(&tmp);

            log(&format!("NSIS installer exited: {:?}", status));

            // Give the installer a moment to finish writing files
            std::thread::sleep(std::time::Duration::from_secs(3));

            if let Some(path) = find_launcher() {
                log(&format!("Launching after install: {:?}", path));
                let _ = Command::new(&path).spawn();
            } else {
                log("Launcher not found after NSIS install");
                show_error("Установка завершена, но лаунчер не найден. Попробуйте перезапустить.");
            }
        } else {
            log("Failed to download NSIS installer");
            show_error("Не удалось скачать установщик. Проверьте интернет-соединение.");
        }
    } else {
        log("No NSIS installer found in release assets");
        show_error("Установщик не найден в последнем релизе на GitHub.");
    }
}

fn fetch_release(client: &reqwest::blocking::Client, url: &str) -> Option<GitHubRelease> {
    let resp = client.get(url).send().ok()?;
    let status = resp.status().as_u16();
    if status == 403 || status == 429 || !resp.status().is_success() {
        return None;
    }
    resp.json().ok()
}

fn download_file(client: &reqwest::blocking::Client, url: &str, tmp_name: &str) -> Option<PathBuf> {
    let resp = client.get(url).send().ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let bytes = resp.bytes().ok()?;
    let tmp = std::env::temp_dir().join(tmp_name);
    std::fs::write(&tmp, &bytes).ok()?;
    Some(tmp)
}

fn find_launcher() -> Option<PathBuf> {
    use winreg::enums::*;
    use winreg::RegKey;

    // 1. Check registry (set by Tauri NSIS currentUser installer)
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    if let Ok(key) = hkcu.open_subkey(REG_KEY) {
        if let Ok(raw) = key.get_value::<String, _>("InstallLocation") {
            let dir = raw.trim_matches('"').trim_end_matches('\\');
            let exe = PathBuf::from(dir).join(LAUNCHER_EXE);
            log(&format!("Registry InstallLocation check: {:?}", exe));
            if exe.exists() {
                return Some(exe);
            }
        }
    }

    // 2. Fallback: standard Tauri NSIS currentUser install path
    //    %LOCALAPPDATA%\Programs\<productName>\<binary>.exe
    if let Ok(local) = std::env::var("LOCALAPPDATA") {
        let path = PathBuf::from(&local)
            .join("Programs")
            .join(LAUNCHER_PRODUCT_NAME)
            .join(LAUNCHER_EXE);
        log(&format!("Fallback path check: {:?}", path));
        if path.exists() {
            return Some(path);
        }
    }

    None
}

#[allow(dead_code)]
fn launch_if_installed() {
    if let Some(path) = find_launcher() {
        close_running_launcher();
        std::thread::sleep(std::time::Duration::from_millis(500));
        let _ = Command::new(&path).spawn();
    }
}

fn close_running_launcher() {
    let _ = Command::new("taskkill")
        .args(["/IM", LAUNCHER_EXE, "/F", "/T"])
        .creation_flags(CREATE_NO_WINDOW)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .stdin(std::process::Stdio::null())
        .status();
}

fn show_error(msg: &str) {
    let _ = Command::new("cmd")
        .args(["/c", &format!(
            "mshta \"javascript:var sh=new ActiveXObject('WScript.Shell');sh.Popup('{}',0,'DanganVerse Launcher',16);close()\"",
            msg
        )])
        .creation_flags(CREATE_NO_WINDOW)
        .spawn();
}