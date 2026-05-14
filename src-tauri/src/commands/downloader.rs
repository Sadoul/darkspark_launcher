use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Read;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

static DOWNLOAD_PROGRESS: Mutex<Option<DownloadProgress>> = Mutex::new(None);
static CANCEL_FLAG: AtomicBool = AtomicBool::new(false);


#[tauri::command]
pub fn cancel_download() {
    CANCEL_FLAG.store(true, Ordering::SeqCst);
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DownloadProgress {
    pub downloaded: u64,
    pub total: u64,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModpackInfo {
    pub name: String,
    pub version: String,
    pub minecraft_version: String,
    pub download_url: String,
}

fn get_modpacks_dir() -> PathBuf {
    let dir = dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".danganverse")
        .join("modpacks");
    fs::create_dir_all(&dir).ok();
    dir
}

fn get_modpack_version_file(modpack_name: &str) -> PathBuf {
    get_modpacks_dir().join(format!("{}.version.json", modpack_name))
}

#[tauri::command]
pub async fn check_modpack_update(
    modpack_name: String,
    github_repo: String,
) -> Result<Option<ModpackInfo>, String> {
    let client = reqwest::Client::builder()
        .user_agent("DarkSparkLauncher/1.0")
        .build()
        .map_err(|e| e.to_string())?;


    let api_url = format!(
        "https://api.github.com/repos/{}/releases/latest",
        github_repo
    );

    let response = client
        .get(&api_url)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !response.status().is_success() {
        return Ok(None);
    }

    let release: serde_json::Value = response.json().await.map_err(|e| e.to_string())?;
    let tag = release["tag_name"]
        .as_str()
        .unwrap_or("unknown")
        .to_string();


    let version_file = get_modpack_version_file(&modpack_name);
    if version_file.exists() {
        let saved: ModpackInfo = serde_json::from_str(
            &fs::read_to_string(&version_file).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;

        if saved.version == tag {
            return Ok(None);
        }
    }


    let download_url = release["assets"]
        .as_array()
        .and_then(|assets| {
            assets.iter().find_map(|a| {
                let name = a["name"].as_str().unwrap_or("");
                if name.contains(&modpack_name) && name.ends_with(".zip") {
                    a["browser_download_url"].as_str().map(|s| s.to_string())
                } else {
                    None
                }
            })
        })
        .unwrap_or_default();

    if download_url.is_empty() {
        return Ok(None);
    }


    let mc_version = release["body"]
        .as_str()
        .and_then(|body| {
            body.lines().find_map(|line| {
                if line.starts_with("mc_version:") {
                    Some(line.trim_start_matches("mc_version:").trim().to_string())
                } else {
                    None
                }
            })
        })
        .unwrap_or_else(|| "1.20.1".to_string());

    Ok(Some(ModpackInfo {
        name: modpack_name,
        version: tag,
        minecraft_version: mc_version,
        download_url,
    }))
}

#[tauri::command]
pub async fn download_modpack(
    modpack_name: String,
    download_url: String,
    version: String,
    minecraft_version: String,
) -> Result<String, String> {
    CANCEL_FLAG.store(false, Ordering::SeqCst);

    let client = reqwest::Client::builder()
        .user_agent("DarkSparkLauncher/1.0")
        .timeout(std::time::Duration::from_secs(300))
        .build()
        .map_err(|e| e.to_string())?;

    set_download_progress(0, 0, "Подключение к серверу...");

    let mut response = client
        .get(&download_url)
        .send()
        .await
        .map_err(|e| format!("Ошибка соединения: {e}"))?;

    if !response.status().is_success() {
        return Err(format!("HTTP {}: не удалось скачать сборку", response.status()));
    }

    let total = response.content_length().unwrap_or(0);
    let mut downloaded: u64 = 0;
    let mut bytes: Vec<u8> = Vec::with_capacity(total as usize);

    set_download_progress(0, total, "Загрузка сборки...");

    loop {
        if CANCEL_FLAG.load(Ordering::SeqCst) {
            set_download_progress(0, 0, "Загрузка отменена");
            return Err("Загрузка отменена пользователем".to_string());
        }
        match response.chunk().await.map_err(|e| format!("Ошибка загрузки: {e}"))? {
            None => break,
            Some(chunk) => {
                downloaded += chunk.len() as u64;
                bytes.extend_from_slice(&chunk);
                set_download_progress(downloaded, total, "Загрузка сборки...");
            }
        }
    }

    set_download_progress(total, total, "Распаковка сборки...");


    let modpack_dir = get_modpacks_dir().join(&modpack_name);
    if modpack_dir.exists() {
        fs::remove_dir_all(&modpack_dir).ok();
    }
    fs::create_dir_all(&modpack_dir).map_err(|e| e.to_string())?;

    let cursor = std::io::Cursor::new(&bytes);
    let mut archive = zip::ZipArchive::new(cursor).map_err(|e| e.to_string())?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i).map_err(|e| e.to_string())?;
        let name = file.name().to_string();

        if name.ends_with('/') {
            fs::create_dir_all(modpack_dir.join(&name)).ok();
        } else {
            let out_path = modpack_dir.join(&name);
            if let Some(parent) = out_path.parent() {
                fs::create_dir_all(parent).ok();
            }
            let mut buf = Vec::new();
            file.read_to_end(&mut buf).map_err(|e| e.to_string())?;
            fs::write(&out_path, &buf).map_err(|e| e.to_string())?;
        }
    }


    let info = ModpackInfo {
        name: modpack_name.clone(),
        version,
        minecraft_version,
        download_url,
    };
    let json = serde_json::to_string_pretty(&info).map_err(|e| e.to_string())?;
    fs::write(get_modpack_version_file(&modpack_name), json).map_err(|e| e.to_string())?;

    set_download_progress(total, total, "Сборка установлена!");

    Ok(modpack_dir.to_string_lossy().to_string())
}

fn set_download_progress(downloaded: u64, total: u64, message: &str) {
    if let Ok(mut p) = DOWNLOAD_PROGRESS.lock() {
        *p = Some(DownloadProgress {
            downloaded,
            total,
            message: message.to_string(),
        });
    }
}

#[tauri::command]
pub async fn get_download_progress() -> Result<Option<DownloadProgress>, String> {
    Ok(DOWNLOAD_PROGRESS
        .lock()
        .map_err(|e| e.to_string())?
        .clone())
}