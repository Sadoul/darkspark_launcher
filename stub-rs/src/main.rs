//! DarkSpark Launcher Stub — single exe bootstrap
//! Checks GitHub for updates, self-updates, and launches the main launcher.

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
const STUB_ASSET_NAME: &str = "DarkSpark-Stub.exe";
const LAUNCHER_EXE: &str = "darkspark-launcher.exe";
const LAUNCHER_DIR: &str = "DarkSpark Launcher";
const REG_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Uninstall\DarkSpark Launcher";
const STUB_VERSION: &str = env!("CARGO_PKG_VERSION");

fn log(msg: &str) {
    let path = std::env::temp_dir().join("darkspark_stub.log");
    let line = format!("[{}] {}\r\n", chrono::Local::now().format("%H:%M:%S"), msg);
    let _ = std::fs::OpenOptions::new().create(true).append(true).open(&path)
        .and_then(|mut f| std::io::Write::write_all(&mut f, line.as_bytes()));
}

fn main() {
    log(&format!("DarkSpark-Stub v{} starting", STUB_VERSION));

    let client = match reqwest::blocking::Client::builder()
        .user_agent("DarkSpark-Stub/1.0")
        .timeout(std::time::Duration::from_secs(20))
        .build()
    {
        Ok(c) => c,
        Err(_) => {
            launch_if_installed();
            exit(0);
        }
    };

    // 1. Fetch latest release
    let api_url = format!("https://api.github.com/repos/{}/releases/latest", GITHUB_REPO);

    let resp = match client.get(&api_url).send() {
        Ok(r) => r,
        Err(_) => {
            launch_if_installed();
            exit(0);
        }
    };

    let status = resp.status().as_u16();
    if status == 403 || status == 429 {
        // Rate limited — just launch whatever is installed
        launch_if_installed();
        exit(0);
    }
    if !resp.status().is_success() {
        launch_if_installed();
        exit(0);
    }

    let release: GitHubRelease = match resp.json() {
        Ok(r) => r,
        Err(_) => {
            launch_if_installed();
            exit(0);
        }
    };

    let latest_tag = release.tag_name.trim_start_matches('v').to_string();
    let current_ver = STUB_VERSION.trim_start_matches('v');

    log(&format!("Local={}, Remote={}", current_ver, latest_tag));

    // 2. Check self-update: if newer stub available, download and replace self
    if compare_versions(&latest_tag, current_ver) > 0 {
        log("Newer stub available — self-updating");
        if let Some(asset) = release.assets.iter().find(|a| a.name == STUB_ASSET_NAME) {
            if let Some(tmp) = download_file(&client, &asset.browser_download_url, STUB_ASSET_NAME) {
                if update_self(&tmp) {
                    log("Self-update OK — relaunching");
                    let new_exe = std::env::current_exe().unwrap();
                    let _ = Command::new(&new_exe).spawn();
                    exit(0);
                }
            }
        }
    }

    // 3. Check if launcher is installed
    if let Some(path) = find_launcher() {
        close_running_launcher();
        std::thread::sleep(std::time::Duration::from_millis(900));
        log(&format!("Launching: {:?}", path));
        let _ = Command::new(&path).spawn();
        exit(0);
    }

    // 4. Launcher not installed — find Tauri exe in release assets
    log("Launcher not installed, looking for Tauri exe");

    let launcher_asset = release.assets.iter().find(|a| {
        let n = a.name.to_lowercase();
        n.contains("darkspark-launcher") && n.ends_with(".exe") && n != STUB_ASSET_NAME.to_lowercase()
    });

    if let Some(asset) = launcher_asset {
        log(&format!("Downloading launcher: {}", asset.name));
        if let Some(tmp) = download_file(&client, &asset.browser_download_url, &asset.name) {
            let target = get_launcher_install_dir().join(&asset.name);
            if std::fs::copy(&tmp, &target).is_ok() {
                std::thread::sleep(std::time::Duration::from_millis(500));
                log(&format!("Launching: {:?}", target));
                let _ = Command::new(&target).spawn();
            }
            let _ = std::fs::remove_file(&tmp);
        }
    } else {
        // No direct exe found — fall back to downloading NSIS installer if exists
        let nsis = release.assets.iter().find(|a| {
            let n = a.name.to_lowercase();
            n.contains("setup") && n.ends_with(".exe")
        });
        if let Some(asset) = nsis {
            log(&format!("Running NSIS installer: {}", asset.name));
            if let Some(tmp) = download_file(&client, &asset.browser_download_url, &asset.name) {
                let _ = Command::new(&tmp)
                    .args(["/S"])
                    .creation_flags(0x08000000)
                    .spawn();
                std::thread::sleep(std::time::Duration::from_millis(4000));
                if let Some(path) = find_launcher() {
                    close_running_launcher();
                    std::thread::sleep(std::time::Duration::from_millis(900));
                    let _ = Command::new(&path).spawn();
                }
                let _ = std::fs::remove_file(&tmp);
            }
        }
    }
}

fn compare_versions(latest: &str, current: &str) -> i32 {
    let parse = |v: &str| {
        v.split('.').map(|s| s.parse::<u64>().unwrap_or(0)).collect::<Vec<u64>>()
    };
    let a = parse(latest);
    let b = parse(current);
    let len = a.len().max(b.len());
    for i in 0..len {
        let av = a.get(i).unwrap_or(&0);
        let bv = b.get(i).unwrap_or(&0);
        if av != bv {
            return if av > bv { 1 } else { -1 };
        }
    }
    0
}

fn download_file(client: &reqwest::blocking::Client, url: &str, name: &str) -> Option<PathBuf> {
    let resp = match client.get(url).send() {
        Ok(r) => r,
        Err(_) => return None,
    };
    if !resp.status().is_success() {
        return None;
    }
    let bytes = match resp.bytes() {
        Ok(b) => b,
        Err(_) => return None,
    };
    let tmp = std::env::temp_dir().join(name);
    if std::fs::write(&tmp, &bytes).is_ok() {
        Some(tmp)
    } else {
        None
    }
}

fn update_self(tmp: &PathBuf) -> bool {
    let self_exe = match std::env::current_exe() {
        Ok(p) => p,
        Err(_) => return false,
    };
    let backup = std::env::temp_dir().join("darkspark_stub_old.exe");

    // Remove old backup first
    let _ = std::fs::remove_file(&backup);

    // Rename current exe to backup
    std::fs::rename(&self_exe, &backup).ok();

    // Copy new exe over current
    if std::fs::copy(tmp, &self_exe).is_err() {
        // Restore backup
        let _ = std::fs::rename(&backup, &self_exe);
        return false;
    }

    // Clean up backup
    let _ = std::fs::remove_file(&backup);
    true
}

fn get_launcher_install_dir() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(LAUNCHER_DIR)
}

fn launch_if_installed() {
    if let Some(path) = find_launcher() {
        close_running_launcher();
        std::thread::sleep(std::time::Duration::from_millis(900));
        let _ = Command::new(&path).spawn();
    }
}

fn close_running_launcher() {
    let _ = Command::new("taskkill")
        .args(["/IM", LAUNCHER_EXE, "/F", "/T"])
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
            let exe = PathBuf::from(dir).join(LAUNCHER_EXE);
            if exe.exists() {
                return Some(exe);
            }
        }
    }

    let candidates = [
        dirs::data_local_dir().map(|d| d.join(&LAUNCHER_DIR).join(LAUNCHER_EXE)),
        dirs::data_local_dir().map(|d| d.join("Programs").join(&LAUNCHER_DIR).join(LAUNCHER_EXE)),
    ];
    for candidate in candidates.into_iter().flatten() {
        if candidate.exists() {
            return Some(candidate);
        }
    }
    None
}