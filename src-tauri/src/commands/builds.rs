use base64::{engine::general_purpose, Engine as _};
use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};
use std::fs;
use std::path::{Path, PathBuf};
use std::io::Write;
use std::sync::Mutex;


const BUILD_BRANCH: &str = "main";
const USER_AGENT: &str = "DanganVerseLauncher-BuildAdmin";

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BuildManifest {
    pub name: String,
    pub minecraft_version: String,
    pub loader: String,
    #[serde(default)]
    pub loader_version: String,
    #[serde(default)]
    pub mods: Vec<BuildFileEntry>,
    #[serde(default)]
    pub server_ip: Option<String>,
    #[serde(default)]
    pub discord_url: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BuildFileEntry {
    pub name: String,
    pub path: String,
    pub url: String,
    pub sha1: String,
    pub size: u64,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

#[derive(Debug, Deserialize)]
struct GitHubContentResponse {
    sha: String,
    #[serde(default)]
    content: String,
    #[serde(default)]
    download_url: Option<String>,
}

fn default_enabled() -> bool { true }

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UploadProgress {
    pub done: usize,
    pub total: usize,
    pub current: String,
    pub errors: Vec<String>,
    pub finished: bool,
}

static UPLOAD_PROGRESS: Mutex<Option<UploadProgress>> = Mutex::new(None);

const UPLOAD_ALLOWED_DIRS: &[&str] = &["mods", "config", "resourcepacks", "shaderpacks", "schematics"];
const UPLOAD_ALLOWED_ROOT: &[&str] = &["options.txt"];

fn walk_dir(dir: &Path, files: &mut Vec<PathBuf>) {
    if let Ok(entries) = fs::read_dir(dir) {
        let mut sorted: Vec<_> = entries.flatten().collect();
        sorted.sort_by_key(|e| e.file_name());
        for entry in sorted {
            let p = entry.path();
            if p.is_dir() { walk_dir(&p, files); }
            else if p.is_file() { files.push(p); }
        }
    }
}

fn collect_upload_files(base: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for rf in UPLOAD_ALLOWED_ROOT {
        let p = base.join(rf);
        if p.is_file() { files.push(p); }
    }
    for dir_name in UPLOAD_ALLOWED_DIRS {
        let dir = base.join(dir_name);
        if dir.is_dir() { walk_dir(&dir, &mut files); }
    }
    files
}

fn git_blob_sha1(content: &[u8]) -> String {
    let header = format!("blob {}\0", content.len());
    let mut h = Sha1::new();
    h.update(header.as_bytes());
    h.update(content);
    format!("{:x}", h.finalize())
}

fn launcher_data_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".danganverse")
}

fn download_dir_file() -> PathBuf {
    launcher_data_dir().join("download_dir.txt")
}

fn default_download_dir() -> PathBuf {
    dirs::download_dir()
        .unwrap_or_else(|| launcher_data_dir())
        .join("DanganVerse Downloads")
}

fn read_download_dir() -> PathBuf {
    let file = download_dir_file();
    fs::read_to_string(file)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(default_download_dir)
}

fn safe_file_name(name: &str) -> String {
    name.chars()
        .map(|ch| match ch {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '_',
            _ => ch,
        })
        .collect()
}


fn repo_for_build(build: &str) -> Result<&'static str, String> {
    match build.to_lowercase().as_str() {
        "danganverse" => Ok("Sadoul/darkspark_modpack"),
        _ => Err(format!("Неизвестная сборка: {build}")),
    }
}

fn manifest_api(repo: &str) -> String {
    format!("https://api.github.com/repos/{repo}/contents/manifest.json")
}

fn file_api(repo: &str, path: &str) -> String {
    format!("https://api.github.com/repos/{repo}/contents/{path}")
}

fn raw_url(repo: &str, path: &str) -> String {
    format!("https://raw.githubusercontent.com/{repo}/main/{path}")
}

fn github_client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .map_err(|e| e.to_string())
}

fn sha1_file(path: &Path) -> Result<String, String> {
    let bytes = fs::read(path).map_err(|e| format!("Не удалось прочитать файл {}: {e}", path.display()))?;
    let mut hasher = Sha1::new();
    hasher.update(bytes);
    Ok(format!("{:x}", hasher.finalize()))
}

async fn get_github_file(client: &reqwest::Client, token: &str, api_url: &str) -> Result<Option<GitHubContentResponse>, String> {
    let response = client
        .get(api_url)
        .bearer_auth(token)
        .query(&[("ref", BUILD_BRANCH)])
        .send()
        .await
        .map_err(|e| format!("GitHub request failed: {e}"))?;

    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(None);
    }
    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("GitHub вернул {status}: {body}"));
    }
    response.json::<GitHubContentResponse>().await.map(Some).map_err(|e| e.to_string())
}

async fn put_github_file(
    client: &reqwest::Client,
    token: &str,
    api_url: &str,
    message: &str,
    content: &[u8],
    old_sha: Option<String>,
) -> Result<(), String> {
    let mut payload = serde_json::json!({
        "message": message,
        "content": general_purpose::STANDARD.encode(content),
        "branch": BUILD_BRANCH,
    });
    if let Some(sha) = old_sha {
        payload["sha"] = serde_json::Value::String(sha);
    }

    let response = client
        .put(api_url)
        .bearer_auth(token)
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("GitHub upload failed: {e}"))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("GitHub отклонил commit: {status}. {body}"));
    }
    Ok(())
}

fn default_manifest(build: &str) -> BuildManifest {
    BuildManifest {
        name: build.to_string(),
        minecraft_version: "1.20.1".to_string(),
        loader: "fabric".to_string(),
        loader_version: String::new(),
        mods: vec![],
        server_ip: None,
        discord_url: None,
    }
}

#[tauri::command]
pub fn get_upload_progress() -> Option<UploadProgress> {
    UPLOAD_PROGRESS.lock().ok().and_then(|p| p.clone())
}

#[tauri::command]
pub async fn upload_modpack_build(
    build: String,
    github_token: String,
    folder_path: String,
) -> Result<Vec<BuildFileEntry>, String> {
    let repo = repo_for_build(&build)?;
    let token = github_token.trim().to_string();
    let base = PathBuf::from(&folder_path);

    if !base.is_dir() {
        return Err(format!("Папка не найдена: {folder_path}"));
    }

    let files = collect_upload_files(&base);
    let total = files.len();
    if total == 0 {
        return Err("В папке нет файлов для загрузки (mods/, config/, resourcepacks/, shaderpacks/, schematics/, options.txt)".to_string());
    }

    if let Ok(mut p) = UPLOAD_PROGRESS.lock() {
        *p = Some(UploadProgress { done: 0, total, current: "Начинаю загрузку...".to_string(), errors: vec![], finished: false });
    }

    let client = github_client()?;
    let mut entries: Vec<BuildFileEntry> = Vec::new();

    for (i, file_path) in files.iter().enumerate() {
        let rel = file_path.strip_prefix(&base)
            .map_err(|e| format!("Ошибка пути: {e}"))?;
        let rel_str = rel.to_string_lossy().replace('\\', "/");

        if let Ok(mut p) = UPLOAD_PROGRESS.lock() {
            if let Some(ref mut prog) = *p {
                prog.done = i;
                prog.current = rel_str.clone();
            }
        }

        let bytes = match fs::read(file_path) {
            Ok(b) => b,
            Err(e) => {
                if let Ok(mut p) = UPLOAD_PROGRESS.lock() {
                    if let Some(ref mut prog) = *p { prog.errors.push(format!("{rel_str}: {e}")); }
                }
                continue;
            }
        };

        let size = bytes.len() as u64;
        let mut h = Sha1::new();
        h.update(&bytes);
        let sha1 = format!("{:x}", h.finalize());

        let api = file_api(repo, &rel_str);
        let existing = get_github_file(&client, &token, &api).await.ok().flatten();

        // Skip upload if file content identical (compare git blob SHA)
        let git_sha = git_blob_sha1(&bytes);
        if let Some(ref ex) = existing {
            if ex.sha == git_sha {
                let name = file_path.file_name().unwrap_or_default().to_string_lossy().to_string();
                entries.push(BuildFileEntry { name, path: rel_str.clone(), url: raw_url(repo, &rel_str), sha1, size, enabled: true });
                continue;
            }
        }

        match put_github_file(
            &client,
            &token,
            &api,
            &format!("build: upload {rel_str}"),
            &bytes,
            existing.map(|f| f.sha),
        ).await {
            Ok(()) => {
                let name = file_path.file_name().unwrap_or_default().to_string_lossy().to_string();
                entries.push(BuildFileEntry { name, path: rel_str.clone(), url: raw_url(repo, &rel_str), sha1, size, enabled: true });
            }
            Err(e) => {
                if let Ok(mut p) = UPLOAD_PROGRESS.lock() {
                    if let Some(ref mut prog) = *p { prog.errors.push(format!("{rel_str}: {e}")); }
                }
            }
        }
    }

    if let Ok(mut p) = UPLOAD_PROGRESS.lock() {
        if let Some(ref mut prog) = *p {
            prog.done = total;
            prog.current = format!("Готово! Загружено: {} из {total} файлов", entries.len());
            prog.finished = true;
        }
    }

    Ok(entries)
}

#[tauri::command]
pub async fn get_build_manifest(build: String, github_token: String) -> Result<BuildManifest, String> {
    let repo = repo_for_build(&build)?;
    let token = github_token.trim();
    let client = github_client()?;
    let Some(file) = get_github_file(&client, token, &manifest_api(repo)).await? else {
        return Ok(default_manifest(&build));
    };

    let mut bytes: Vec<u8> = if !file.content.trim().is_empty() {
        let content = file.content.replace(['\r', '\n', ' '], "");
        general_purpose::STANDARD.decode(content).map_err(|e| e.to_string())?
    } else if let Some(url) = file.download_url.clone() {
        let resp = client
            .get(&url)
            .bearer_auth(token)
            .send()
            .await
            .map_err(|e| format!("Manifest download failed: {e}"))?;
        if !resp.status().is_success() {
            return Err(format!("Manifest download HTTP {}", resp.status()));
        }
        resp.bytes().await.map_err(|e| e.to_string())?.to_vec()
    } else {
        return Err("Manifest без content и download_url".to_string());
    };

    if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        bytes.drain(0..3);
    }

    serde_json::from_slice::<BuildManifest>(&bytes)
        .map_err(|e| format!("Manifest parse failed: {e}"))
}

#[tauri::command]
pub async fn commit_build_manifest(build: String, github_token: String, manifest: BuildManifest) -> Result<String, String> {
    let repo = repo_for_build(&build)?;
    let token = github_token.trim();
    let client = github_client()?;
    let current = get_github_file(&client, token, &manifest_api(repo)).await?;
    let bytes = serde_json::to_vec_pretty(&manifest).map_err(|e| e.to_string())?;
    put_github_file(
        &client,
        token,
        &manifest_api(repo),
        &format!("chore: update {build} build manifest from launcher admin panel"),
        &bytes,
        current.map(|f| f.sha),
    ).await?;
    Ok(format!("Manifest сборки {build} обновлён"))
}

#[tauri::command]
pub async fn upload_build_mod(build: String, github_token: String, file_path: String, target_name: Option<String>) -> Result<BuildFileEntry, String> {
    let repo = repo_for_build(&build)?;
    let token = github_token.trim();
    let path = PathBuf::from(&file_path);
    let file_name = target_name
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| path.file_name().unwrap_or_default().to_string_lossy().to_string());
    if !file_name.ends_with(".jar") {
        return Err("Можно загружать только .jar моды".to_string());
    }
    let bytes = fs::read(&path).map_err(|e| format!("Не удалось прочитать мод: {e}"))?;
    let size = bytes.len() as u64;
    let sha1 = sha1_file(&path)?;
    let remote_path = format!("mods/{file_name}");

    let client = github_client()?;
    let api = file_api(repo, &remote_path);
    let current = get_github_file(&client, token, &api).await?;
    put_github_file(
        &client,
        token,
        &api,
        &format!("chore: upload mod {file_name} from launcher admin panel"),
        &bytes,
        current.map(|f| f.sha),
    ).await?;

    Ok(BuildFileEntry {
        name: file_name.clone(),
        path: remote_path.clone(),
        url: raw_url(repo, &remote_path),
        sha1,
        size,
        enabled: true,
    })
}

#[tauri::command]
pub fn get_build_download_dir() -> Result<String, String> {
    Ok(read_download_dir().to_string_lossy().to_string())
}

#[tauri::command]
pub fn set_build_download_dir(path: String) -> Result<(), String> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err("Путь сохранения пустой".to_string());
    }
    let dir = PathBuf::from(trimmed);
    fs::create_dir_all(&dir).map_err(|e| format!("Не удалось создать папку сохранения: {e}"))?;
    let config_dir = launcher_data_dir();
    fs::create_dir_all(&config_dir).map_err(|e| format!("Не удалось создать папку настроек: {e}"))?;
    fs::write(download_dir_file(), dir.to_string_lossy().as_bytes())
        .map_err(|e| format!("Не удалось сохранить путь: {e}"))
}

#[tauri::command]
pub async fn download_build_mod_file(mod_entry: BuildFileEntry) -> Result<String, String> {
    let target_dir = read_download_dir();
    fs::create_dir_all(&target_dir).map_err(|e| format!("Не удалось создать папку сохранения: {e}"))?;
    let target_path = target_dir.join(safe_file_name(&mod_entry.name));

    let client = github_client()?;
    let response = client
        .get(&mod_entry.url)
        .send()
        .await
        .map_err(|e| format!("Не удалось скачать мод: {e}"))?;
    if !response.status().is_success() {
        return Err(format!("Скачивание мода вернуло HTTP {}", response.status()));
    }
    let bytes = response.bytes().await.map_err(|e| e.to_string())?;
    let mut file = fs::File::create(&target_path)
        .map_err(|e| format!("Не удалось создать файл {}: {e}", target_path.display()))?;
    file.write_all(&bytes).map_err(|e| format!("Не удалось записать мод: {e}"))?;
    Ok(target_path.to_string_lossy().to_string())
}

#[tauri::command]
pub async fn download_build_bundle(build: String, manifest: BuildManifest) -> Result<String, String> {
    let target_dir = read_download_dir().join(safe_file_name(&build));
    let mods_dir = target_dir.join("mods");
    fs::create_dir_all(&mods_dir).map_err(|e| format!("Не удалось создать папку сборки: {e}"))?;
    let manifest_path = target_dir.join("manifest.json");
    let manifest_bytes = serde_json::to_vec_pretty(&manifest).map_err(|e| e.to_string())?;
    fs::write(&manifest_path, manifest_bytes).map_err(|e| format!("Не удалось записать manifest: {e}"))?;

    let client = github_client()?;
    let mut downloaded = 0usize;
    for mod_entry in manifest.mods.iter().filter(|mod_entry| mod_entry.enabled) {
        let response = client
            .get(&mod_entry.url)
            .send()
            .await
            .map_err(|e| format!("Не удалось скачать {}: {e}", mod_entry.name))?;
        if !response.status().is_success() {
            return Err(format!("{}: HTTP {}", mod_entry.name, response.status()));
        }
        let bytes = response.bytes().await.map_err(|e| e.to_string())?;
        fs::write(mods_dir.join(safe_file_name(&mod_entry.name)), bytes)
            .map_err(|e| format!("Не удалось записать {}: {e}", mod_entry.name))?;
        downloaded += 1;
    }

    Ok(format!("Сборка сохранена в {} (модов: {downloaded})", target_dir.display()))
}