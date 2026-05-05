use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[derive(Debug, Serialize, Deserialize)]
pub struct JavaInfo {
    pub path: String,
    pub version: String,
    pub found: bool,
}

#[tauri::command]
pub async fn find_java() -> Result<JavaInfo, String> {

    let bundled = get_bundled_java_path();
    if bundled.exists() {
        if let Some(version) = get_java_version(&bundled) {
            return Ok(JavaInfo {
                path: bundled.to_string_lossy().to_string(),
                version,
                found: true,
            });
        }
    }


    if let Ok(java_home) = std::env::var("JAVA_HOME") {
        let java_path = PathBuf::from(&java_home).join("bin").join("java.exe");
        if java_path.exists() {
            if let Some(version) = get_java_version(&java_path) {
                return Ok(JavaInfo {
                    path: java_path.to_string_lossy().to_string(),
                    version,
                    found: true,
                });
            }
        }
    }


    if let Some(version) = get_java_version(&PathBuf::from("java")) {
        return Ok(JavaInfo {
            path: "java".to_string(),
            version,
            found: true,
        });
    }


    let common_paths = vec![
        "C:\\Program Files\\Java",
        "C:\\Program Files (x86)\\Java",
        "C:\\Program Files\\Eclipse Adoptium",
        "C:\\Program Files\\Microsoft\\jdk-17",
    ];

    for base in common_paths {
        let base_path = PathBuf::from(base);
        if base_path.exists() {
            if let Ok(entries) = fs::read_dir(&base_path) {
                for entry in entries.flatten() {
                    let java_path = entry.path().join("bin").join("java.exe");
                    if java_path.exists() {
                        if let Some(version) = get_java_version(&java_path) {
                            return Ok(JavaInfo {
                                path: java_path.to_string_lossy().to_string(),
                                version,
                                found: true,
                            });
                        }
                    }
                }
            }
        }
    }

    Ok(JavaInfo {
        path: String::new(),
        version: String::new(),
        found: false,
    })
}

fn get_bundled_java_path() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".darkspark")
        .join("java")
        .join("bin")
        .join("java.exe")
}

fn get_java_version(java_path: &PathBuf) -> Option<String> {
    let output = Command::new(java_path.to_string_lossy().to_string())
        .arg("-version")
        .output()
        .ok()?;

    let stderr = String::from_utf8_lossy(&output.stderr);
    let first_line = stderr.lines().next()?;


    if let Some(start) = first_line.find('"') {
        if let Some(end) = first_line[start + 1..].find('"') {
            return Some(first_line[start + 1..start + 1 + end].to_string());
        }
    }

    None
}

#[tauri::command]
pub async fn download_java() -> Result<JavaInfo, String> {
    let client = reqwest::Client::builder()
        .user_agent("RPWLauncher/1.0")
        .build()
        .map_err(|e| e.to_string())?;


    let api_url = "https://api.adoptium.net/v3/assets/latest/17/hotspot?architecture=x64&image_type=jre&os=windows&vendor=eclipse";

    let response = client
        .get(api_url)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let releases: Vec<serde_json::Value> = response.json().await.map_err(|e| e.to_string())?;

    let download_url = releases
        .first()
        .and_then(|r| r["binary"]["package"]["link"].as_str())
        .ok_or("Could not find Java download URL")?
        .to_string();


    let response = client
        .get(&download_url)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let bytes = response.bytes().await.map_err(|e| e.to_string())?;


    let java_base_dir = dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".darkspark")
        .join("java");

    if java_base_dir.exists() {
        fs::remove_dir_all(&java_base_dir).ok();
    }
    fs::create_dir_all(&java_base_dir).map_err(|e| e.to_string())?;

    let cursor = std::io::Cursor::new(&bytes);
    let mut archive = zip::ZipArchive::new(cursor).map_err(|e| e.to_string())?;


    let root_dir = archive
        .by_index(0)
        .map_err(|e| e.to_string())?
        .name()
        .split('/')
        .next()
        .unwrap_or("")
        .to_string();

    for i in 0..archive.len() {
        let mut file = archive.by_index(i).map_err(|e| e.to_string())?;
        let name = file.name().to_string();


        let relative = name
            .strip_prefix(&format!("{}/", root_dir))
            .unwrap_or(&name);
        if relative.is_empty() {
            continue;
        }

        let out_path = java_base_dir.join(relative);

        if name.ends_with('/') {
            fs::create_dir_all(&out_path).ok();
        } else {
            if let Some(parent) = out_path.parent() {
                fs::create_dir_all(parent).ok();
            }
            let mut buf = Vec::new();
            std::io::Read::read_to_end(&mut file, &mut buf).map_err(|e| e.to_string())?;
            fs::write(&out_path, &buf).map_err(|e| e.to_string())?;
        }
    }

    let java_path = java_base_dir.join("bin").join("java.exe");
    let version = get_java_version(&java_path).unwrap_or_else(|| "17".to_string());

    Ok(JavaInfo {
        path: java_path.to_string_lossy().to_string(),
        version,
        found: true,
    })
}