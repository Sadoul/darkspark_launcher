use base64::{engine::general_purpose, Engine as _};
use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};
use std::fs;
use std::path::{Path, PathBuf};
use std::io::{Read, Write};
use std::sync::Mutex;
use zip::write::SimpleFileOptions;

use super::logger::log as launcher_log;

fn token_preview(token: &str) -> String {
    let len = token.len();
    if len == 0 {
        return "<empty>".to_string();
    }
    let prefix: String = token.chars().take(4).collect();
    let suffix: String = token.chars().rev().take(4).collect::<String>().chars().rev().collect();
    format!("{prefix}…{suffix} (len={len})")
}

const BUILD_BRANCH: &str = "main";
const USER_AGENT: &str = "DanganVerseLauncher-BuildAdmin";

// GitHub Contents API hard limit is 100 MB. We upload files larger than this
// threshold as release assets (Releases support up to 2 GB per file).
const LARGE_FILE_THRESHOLD: u64 = 95 * 1024 * 1024;
const STORAGE_RELEASE_TAG: &str = "storage";

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

#[derive(Debug, Deserialize, Serialize)]
pub struct GitTreeEntry {
    pub path: String,
    pub mode: String,
    pub r#type: String,
    pub sha: String,
    pub size: Option<u64>,
    pub url: String,
}

#[derive(Debug, Deserialize)]
struct GitTreeResponse {
    sha: String,
    tree: Vec<GitTreeEntry>,
    truncated: bool,
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

fn github_upload_client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .user_agent(USER_AGENT)
        // Large release assets (>100 MB) need plenty of headroom on slow links.
        .timeout(std::time::Duration::from_secs(60 * 60))
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
    launcher_log(&format!("[builds] GET {api_url} (token={})", token_preview(token)));
    let response = client
        .get(api_url)
        .bearer_auth(token)
        .query(&[("ref", BUILD_BRANCH)])
        .send()
        .await
        .map_err(|e| {
            let msg = format!("GitHub request failed: {e}");
            launcher_log(&format!("[builds] GET {api_url} network error: {e}"));
            msg
        })?;

    let status = response.status();
    launcher_log(&format!("[builds] GET {api_url} -> {status}"));

    if status == reqwest::StatusCode::NOT_FOUND {
        return Ok(None);
    }
    if status == reqwest::StatusCode::UNAUTHORIZED {
        let body = response.text().await.unwrap_or_default();
        launcher_log(&format!("[builds] 401 body: {body}"));
        return Err("Токен GitHub недействителен или истёк. Проверьте: токен должен иметь разрешение «Contents: Read and write» для репозитория Sadoul/darkspark_modpack. Fine-grained PAT: нужно выбрать этот репозиторий в настройках токена.".to_string());
    }
    if status == reqwest::StatusCode::FORBIDDEN {
        let body = response.text().await.unwrap_or_default();
        launcher_log(&format!("[builds] 403 body: {body}"));
        let hint = if body.contains("rate limit") {
            "GitHub вернул 403 (rate limit). Проверьте, что токен реально передаётся (не пустая строка) и что у токена есть доступ к Sadoul/darkspark_modpack. Если используете VPN/Cloudflare Warp — попробуйте отключить."
        } else if body.contains("Resource not accessible") {
            "GitHub вернул 403 «Resource not accessible by personal access token». Fine-grained PAT не выбрал репозиторий Sadoul/darkspark_modpack или нет разрешения Contents: Read and write."
        } else {
            "GitHub вернул 403 Forbidden. У токена нет write-доступа к репозиторию Sadoul/darkspark_modpack либо аккаунт не является collaborator."
        };
        return Err(format!("{hint}\n\nПолный ответ: {body}"));
    }
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        launcher_log(&format!("[builds] {status} body: {body}"));
        return Err(format!("GitHub вернул {status}: {body}"));
    }
    response.json::<GitHubContentResponse>().await.map(Some).map_err(|e| {
        launcher_log(&format!("[builds] GET {api_url} json parse error: {e}"));
        e.to_string()
    })
}

async fn put_github_file_with_client(
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

    launcher_log(&format!("[builds] PUT {api_url} ({} bytes, token={})", content.len(), token_preview(token)));

    let response = client
        .put(api_url)
        .bearer_auth(token)
        .json(&payload)
        .send()
        .await
        .map_err(|e| {
            launcher_log(&format!("[builds] PUT {api_url} network error: {e}"));
            format!("GitHub upload failed: {e}")
        })?;

    let status = response.status();
    launcher_log(&format!("[builds] PUT {api_url} -> {status}"));

    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        launcher_log(&format!("[builds] PUT {api_url} error body: {body}"));
        let hint = if status == reqwest::StatusCode::UNAUTHORIZED {
            "Токен недействителен или истёк."
        } else if status == reqwest::StatusCode::FORBIDDEN {
            if body.contains("Resource not accessible") {
                "У токена нет прав на запись в этот репозиторий. Нужно: Contents: Read and write + выбран репозиторий Sadoul/darkspark_modpack."
            } else if body.contains("rate limit") {
                "GitHub rate limit. Попробуйте через несколько минут или отключите VPN/Warp."
            } else {
                "GitHub отказал в доступе."
            }
        } else if status == reqwest::StatusCode::CONFLICT {
            "Конфликт sha (файл уже изменили). Перезагрузите дерево и попробуйте снова."
        } else if status == reqwest::StatusCode::UNPROCESSABLE_ENTITY {
            "GitHub отклонил тело запроса (422). Возможно, файл слишком большой (>100 МБ) или неверный sha."
        } else {
            "Неизвестная ошибка GitHub."
        };
        return Err(format!("GitHub отклонил commit: {status}. {hint}\n\nПолный ответ: {body}"));
    }
    Ok(())
}

// ---- Release-asset upload (for files > 100 MB) -----------------------------

#[derive(Debug, Deserialize)]
struct GitHubRelease {
    id: u64,
    #[serde(default)]
    assets: Vec<GitHubReleaseAsset>,
}

#[derive(Debug, Deserialize, Clone)]
struct GitHubReleaseAsset {
    id: u64,
    name: String,
    browser_download_url: String,
    #[serde(default)]
    size: u64,
}

/// Sanitize a path so it can become a release-asset file name. GitHub's
/// `uploads.github.com` accepts a flat name; `/` becomes `__`.
fn asset_name_for_path(rel_path: &str) -> String {
    rel_path.replace('/', "__")
}

async fn get_or_create_storage_release(
    client: &reqwest::Client,
    token: &str,
    repo: &str,
) -> Result<GitHubRelease, String> {
    let api = format!(
        "https://api.github.com/repos/{repo}/releases/tags/{STORAGE_RELEASE_TAG}"
    );
    launcher_log(&format!("[builds] storage release lookup: {api}"));

    let response = client
        .get(&api)
        .bearer_auth(token)
        .send()
        .await
        .map_err(|e| format!("Storage release lookup failed: {e}"))?;

    let status = response.status();
    if status.is_success() {
        return response
            .json::<GitHubRelease>()
            .await
            .map_err(|e| format!("Не удалось разобрать storage release: {e}"));
    }

    if status != reqwest::StatusCode::NOT_FOUND {
        let body = response.text().await.unwrap_or_default();
        return Err(format!(
            "Storage release lookup HTTP {status}. Тело: {body}"
        ));
    }

    // Create the release if it does not exist yet.
    launcher_log(&format!("[builds] creating storage release on {repo}"));
    let create_api = format!("https://api.github.com/repos/{repo}/releases");
    let payload = serde_json::json!({
        "tag_name": STORAGE_RELEASE_TAG,
        "target_commitish": BUILD_BRANCH,
        "name": "Modpack large file storage",
        "body": "Авто-релиз для хранения файлов >100 МБ. Управляется лаунчером.",
        "draft": false,
        "prerelease": true,
    });

    let create = client
        .post(&create_api)
        .bearer_auth(token)
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("Создание storage release failed: {e}"))?;

    let create_status = create.status();
    if !create_status.is_success() {
        let body = create.text().await.unwrap_or_default();
        return Err(format!(
            "Не удалось создать storage release: {create_status}. {body}"
        ));
    }
    create
        .json::<GitHubRelease>()
        .await
        .map_err(|e| format!("Storage release create parse failed: {e}"))
}

async fn delete_release_asset(
    client: &reqwest::Client,
    token: &str,
    repo: &str,
    asset_id: u64,
) -> Result<(), String> {
    let url = format!(
        "https://api.github.com/repos/{repo}/releases/assets/{asset_id}"
    );
    launcher_log(&format!("[builds] DELETE asset {url}"));
    let resp = client
        .delete(&url)
        .bearer_auth(token)
        .send()
        .await
        .map_err(|e| format!("Asset delete failed: {e}"))?;
    if !resp.status().is_success() && resp.status() != reqwest::StatusCode::NOT_FOUND {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("Asset delete {status}: {body}"));
    }
    Ok(())
}

/// Upload a file as a release asset and return its public download URL.
/// Replaces any existing asset with the same name.
async fn upload_as_release_asset(
    client: &reqwest::Client,
    token: &str,
    repo: &str,
    rel_path: &str,
    bytes: Vec<u8>,
) -> Result<String, String> {
    let release = get_or_create_storage_release(client, token, repo).await?;
    let asset_name = asset_name_for_path(rel_path);

    // If an asset with the same name AND same size already exists, skip
    // re-upload — this saves bandwidth on rebuilds where the file did not
    // actually change.
    if let Some(existing) = release.assets.iter().find(|a| a.name == asset_name) {
        if existing.size == bytes.len() as u64 {
            launcher_log(&format!(
                "[builds] release asset {} unchanged (size={}), skipping",
                asset_name, existing.size
            ));
            return Ok(existing.browser_download_url.clone());
        }
        launcher_log(&format!(
            "[builds] replacing existing asset {} (id={}, old size={}, new size={})",
            existing.name, existing.id, existing.size, bytes.len()
        ));
        delete_release_asset(client, token, repo, existing.id).await?;
    }

    let upload_url = format!(
        "https://uploads.github.com/repos/{repo}/releases/{}/assets?name={}",
        release.id,
        urlencoding::encode(&asset_name)
    );
    launcher_log(&format!(
        "[builds] uploading release asset {} ({} bytes) to {}",
        asset_name,
        bytes.len(),
        upload_url
    ));

    let resp = client
        .post(&upload_url)
        .bearer_auth(token)
        .header(reqwest::header::CONTENT_TYPE, "application/octet-stream")
        .body(bytes)
        .send()
        .await
        .map_err(|e| format!("Asset upload failed: {e}"))?;

    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        launcher_log(&format!("[builds] asset upload {status} body: {body}"));
        let hint = if status == reqwest::StatusCode::FORBIDDEN {
            "Нет прав на загрузку release asset. Токену нужны Contents + (для fine-grained) разрешение на репозиторий."
        } else if status == reqwest::StatusCode::UNPROCESSABLE_ENTITY {
            "GitHub отклонил ассет (422). Возможно дубликат имени или файл >2 ГБ."
        } else {
            "Неизвестная ошибка загрузки release asset."
        };
        return Err(format!(
            "GitHub отклонил release asset: {status}. {hint}\n\nПолный ответ: {body}"
        ));
    }

    let asset: GitHubReleaseAsset = resp
        .json()
        .await
        .map_err(|e| format!("Не удалось разобрать ответ asset: {e}"))?;
    launcher_log(&format!(
        "[builds] asset uploaded: id={}, url={}",
        asset.id, asset.browser_download_url
    ));
    Ok(asset.browser_download_url)
}

/// Routes upload to either Contents API (small files) or Releases asset
/// (files >= LARGE_FILE_THRESHOLD). Returns the public download URL.
async fn upload_file_smart(
    client: &reqwest::Client,
    token: &str,
    repo: &str,
    rel_path: &str,
    bytes: Vec<u8>,
) -> Result<String, String> {
    let size = bytes.len() as u64;
    if size >= LARGE_FILE_THRESHOLD {
        launcher_log(&format!(
            "[builds] {} is {} bytes (>= {}), routing to release-asset upload",
            rel_path, size, LARGE_FILE_THRESHOLD
        ));
        return upload_as_release_asset(client, token, repo, rel_path, bytes).await;
    }

    let api = file_api(repo, rel_path);
    let existing = get_github_file(client, token, &api).await.ok().flatten();
    let git_sha = git_blob_sha1(&bytes);
    if let Some(ref ex) = existing {
        if ex.sha == git_sha {
            return Ok(raw_url(repo, rel_path));
        }
    }
    put_github_file_with_client(
        client,
        token,
        &api,
        &format!("build: upload {rel_path}"),
        &bytes,
        existing.map(|f| f.sha),
    )
    .await?;
    Ok(raw_url(repo, rel_path))
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

    launcher_log(&format!(
        "[builds] upload_modpack_build start: build={build}, repo={repo}, folder={folder_path}, token={}",
        token_preview(&token)
    ));

    if token.is_empty() {
        return Err("GitHub токен не задан. Сохраните токен в админ-панели перед загрузкой.".to_string());
    }

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

    let client = github_upload_client()?;
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

        match upload_file_smart(&client, &token, repo, &rel_str, bytes).await {
            Ok(url) => {
                let name = file_path.file_name().unwrap_or_default().to_string_lossy().to_string();
                entries.push(BuildFileEntry { name, path: rel_str.clone(), url, sha1, size, enabled: true });
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
    put_github_file_with_client(
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
pub async fn get_build_git_tree(build: String, github_token: String) -> Result<Vec<GitTreeEntry>, String> {
    let repo = repo_for_build(&build)?;
    let token = github_token.trim();
    let client = github_client()?;
    
    let api_url = format!("https://api.github.com/repos/{repo}/git/trees/{BUILD_BRANCH}?recursive=1");
    let response = client
        .get(&api_url)
        .bearer_auth(token)
        .send()
        .await
        .map_err(|e| format!("GitHub request failed: {e}"))?;
        
    if !response.status().is_success() {
        return Err(format!("GitHub вернул HTTP {}", response.status()));
    }
    
    let json = response.json::<GitTreeResponse>().await.map_err(|e| format!("Не удалось разобрать дерево: {e}"))?;
    Ok(json.tree)
}

fn zip_directory(dir: &Path) -> Result<Vec<u8>, String> {
    let mut buffer = Vec::new();
    {
        let mut zip = zip::ZipWriter::new(std::io::Cursor::new(&mut buffer));
        let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        
        let mut stack = vec![dir.to_path_buf()];
        while let Some(current_dir) = stack.pop() {
            let entries = fs::read_dir(&current_dir).map_err(|e| format!("Не удалось прочитать папку: {e}"))?;
            for entry in entries {
                let entry = entry.map_err(|e| e.to_string())?;
                let path = entry.path();
                let name = path.strip_prefix(dir).unwrap().to_string_lossy().replace("\\", "/");
                
                if path.is_dir() {
                    zip.add_directory(&name, options).map_err(|e| e.to_string())?;
                    stack.push(path);
                } else {
                    zip.start_file(&name, options).map_err(|e| e.to_string())?;
                    let mut f = fs::File::open(&path).map_err(|e| e.to_string())?;
                    let mut b = Vec::new();
                    f.read_to_end(&mut b).map_err(|e| e.to_string())?;
                    zip.write_all(&b).map_err(|e| e.to_string())?;
                }
            }
        }
        zip.finish().map_err(|e| format!("Ошибка создания архива: {e}"))?;
    }
    Ok(buffer)
}

#[tauri::command]
pub async fn upload_build_mod(build: String, github_token: String, file_path: String, target_name: Option<String>) -> Result<BuildFileEntry, String> {
    let repo = repo_for_build(&build)?;
    let token = github_token.trim();
    let path = PathBuf::from(&file_path);

    launcher_log(&format!(
        "[builds] upload_build_mod start: build={build}, repo={repo}, file={file_path}, token={}",
        token_preview(token)
    ));

    if token.is_empty() {
        return Err("GitHub токен не задан. Сохраните токен в админ-панели перед загрузкой.".to_string());
    }
    
    let is_dir = path.is_dir();
    let mut file_name = target_name
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| path.file_name().unwrap_or_default().to_string_lossy().to_string());
        
    let bytes = if is_dir {
        if !file_name.ends_with(".zip") {
            file_name = format!("{}.zip", file_name);
        }
        zip_directory(&path)?
    } else {
        fs::read(&path).map_err(|e| format!("Не удалось прочитать файл: {e}"))?
    };
    
    let mut folder = "mods";
    if file_name.ends_with(".zip") {
        if is_dir {
            if path.join("pack.mcmeta").exists() {
                folder = "resourcepacks";
            } else if path.join("shaders").exists() || path.join("shaders").is_dir() {
                folder = "shaderpacks";
            }
        } else {
            if let Ok(mut archive) = zip::ZipArchive::new(std::io::Cursor::new(&bytes)) {
                let mut has_mcmeta = false;
                let mut has_shaders = false;
                for i in 0..archive.len() {
                    if let Ok(file) = archive.by_index(i) {
                        let name = file.name();
                        if name == "pack.mcmeta" { has_mcmeta = true; }
                        if name.starts_with("shaders/") { has_shaders = true; }
                    }
                }
                if has_mcmeta { folder = "resourcepacks"; }
                else if has_shaders { folder = "shaderpacks"; }
            }
        }
    }
    
    let size = bytes.len() as u64;
    let mut h = Sha1::new();
    h.update(&bytes);
    let sha1 = format!("{:x}", h.finalize());
    let remote_path = format!("{folder}/{file_name}");

    let client = github_upload_client()?;
    let url = upload_file_smart(&client, token, repo, &remote_path, bytes).await?;

    Ok(BuildFileEntry {
        name: file_name.clone(),
        path: remote_path.clone(),
        url,
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

#[tauri::command]
pub async fn delete_build_file(
    build: String,
    github_token: String,
    file_path: String,
    sha: String,
) -> Result<(), String> {
    let repo = repo_for_build(&build)?;
    let token = github_token.trim();
    let client = github_upload_client()?;

    launcher_log(&format!(
        "[builds] delete_build_file: build={build}, repo={repo}, path={file_path}, token={}",
        token_preview(token)
    ));

    if token.is_empty() {
        return Err("GitHub токен не задан.".to_string());
    }

    // First try to delete a matching release asset (used for files >100 MB).
    // We don't fail if there isn't one — fall through to Contents API.
    if let Ok(release) = get_or_create_storage_release(&client, token, repo).await {
        let asset_name = asset_name_for_path(&file_path);
        if let Some(asset) = release.assets.iter().find(|a| a.name == asset_name) {
            launcher_log(&format!(
                "[builds] delete: matching release asset found id={}, deleting",
                asset.id
            ));
            delete_release_asset(&client, token, repo, asset.id).await?;
            // If sha is empty, we're done (file was release-only).
            if sha.trim().is_empty() {
                return Ok(());
            }
        }
    }

    let api = file_api(repo, &file_path);
    let payload = serde_json::json!({
        "message": format!("chore: delete {} via launcher admin panel", file_path),
        "sha": sha,
        "branch": BUILD_BRANCH,
    });

    let response = client
        .delete(&api)
        .bearer_auth(token)
        .json(&payload)
        .send()
        .await
        .map_err(|e| {
            launcher_log(&format!("[builds] DELETE {api} network error: {e}"));
            format!("GitHub request failed: {e}")
        })?;

    let status = response.status();
    launcher_log(&format!("[builds] DELETE {api} -> {status}"));

    if status == reqwest::StatusCode::UNAUTHORIZED {
        return Err("Токен GitHub недействителен. Нужно разрешение Contents: Read and write".to_string());
    }
    // 404 is acceptable: the file is gone (release-only or already deleted)
    if status == reqwest::StatusCode::NOT_FOUND {
        return Ok(());
    }
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        launcher_log(&format!("[builds] DELETE {api} body: {body}"));
        return Err(format!("GitHub отклонил удаление {}: {status}. {body}", file_path));
    }
    Ok(())
}

#[tauri::command]
pub async fn upload_build_from_zip(
    build: String,
    github_token: String,
    zip_path: String,
) -> Result<Vec<BuildFileEntry>, String> {
    let repo = repo_for_build(&build)?;
    let token = github_token.trim().to_string();
    let path = PathBuf::from(&zip_path);

    launcher_log(&format!(
        "[builds] upload_build_from_zip start: build={build}, repo={repo}, zip={zip_path}, token={}",
        token_preview(&token)
    ));

    if token.is_empty() {
        return Err("GitHub токен не задан. Сохраните токен в админ-панели перед загрузкой.".to_string());
    }

    if !path.is_file() {
        return Err(format!("Файл не найден: {zip_path}"));
    }

    // The ZIP is EXTRACTED and every inner file is committed to GitHub at its
    // own path, preserving the archive's folder structure (emotes/,
    // shaderpacks/, resourcepacks/, config/, mods/, options.txt, …). Folders
    // themselves are kept as-is. Large files (>95 MB) transparently go to the
    // storage release. The init-only protection on the launcher side means
    // existing players keep their options.txt / packs / emotes, while fresh
    // installs receive everything.
    let zip_bytes = fs::read(&path).map_err(|e| format!("Не удалось прочитать ZIP: {e}"))?;
    let cursor = std::io::Cursor::new(&zip_bytes);
    let mut archive = zip::ZipArchive::new(cursor).map_err(|e| format!("Не удалось открыть ZIP: {e}"))?;

    let mut zip_entries: Vec<(String, Vec<u8>)> = Vec::new();
    for i in 0..archive.len() {
        let mut file = archive.by_index(i).map_err(|e| format!("ZIP ошибка [{i}]: {e}"))?;
        if file.is_dir() { continue; }
        let name = file.name().replace('\\', "/").to_string();
        let mut content = Vec::new();
        file.read_to_end(&mut content).map_err(|e| format!("Ошибка чтения {}: {e}", file.name()))?;
        zip_entries.push((name, content));
    }

    if zip_entries.is_empty() {
        return Err("ZIP файл пуст или содержит только папки".to_string());
    }

    // If the archive wraps everything in a single top-level folder
    // (e.g. you zipped the `danganverse` folder itself → danganverse/mods/…),
    // strip that wrapper so paths become mods/…, config/… etc. If the archive
    // already has multiple top-level entries (mods/, emotes/, options.txt),
    // nothing is stripped — folders are left untouched.
    let root_prefix: String = {
        let first = &zip_entries[0].0;
        if let Some(pos) = first.find('/') {
            let candidate = format!("{}/", &first[..pos]);
            if zip_entries.iter().all(|(p, _)| p.starts_with(&candidate)) { candidate } else { String::new() }
        } else { String::new() }
    };

    let files_to_upload: Vec<(String, Vec<u8>)> = zip_entries
        .into_iter()
        .map(|(p, c)| {
            let stripped = if !root_prefix.is_empty() && p.starts_with(&root_prefix) {
                p[root_prefix.len()..].to_string()
            } else {
                p
            };
            (stripped, c)
        })
        .filter(|(p, _)| !p.is_empty() && !p.starts_with('.') && !p.contains("__MACOSX"))
        .collect();

    let total = files_to_upload.len();
    if total == 0 {
        return Err("В ZIP нет подходящих файлов для загрузки".to_string());
    }

    if let Ok(mut pl) = UPLOAD_PROGRESS.lock() {
        *pl = Some(UploadProgress {
            done: 0,
            total,
            current: "Распаковка ZIP и загрузка файлов...".to_string(),
            errors: vec![],
            finished: false,
        });
    }

    let client = github_upload_client()?;
    let mut entries: Vec<BuildFileEntry> = Vec::new();

    for (i, (rel_path, content)) in files_to_upload.into_iter().enumerate() {
        if let Ok(mut pl) = UPLOAD_PROGRESS.lock() {
            if let Some(ref mut prog) = *pl { prog.done = i; prog.current = rel_path.clone(); }
        }
        let size = content.len() as u64;
        let mut h = Sha1::new();
        h.update(&content);
        let sha1 = format!("{:x}", h.finalize());

        match upload_file_smart(&client, &token, repo, &rel_path, content).await {
            Ok(url) => {
                let name = rel_path.rsplit('/').next().unwrap_or(&rel_path).to_string();
                entries.push(BuildFileEntry { name, path: rel_path.clone(), url, sha1, size, enabled: true });
            }
            Err(e) => {
                if let Ok(mut pl) = UPLOAD_PROGRESS.lock() {
                    if let Some(ref mut prog) = *pl { prog.errors.push(format!("{rel_path}: {e}")); }
                }
            }
        }
    }

    if let Ok(mut pl) = UPLOAD_PROGRESS.lock() {
        if let Some(ref mut prog) = *pl {
            prog.done = total;
            prog.current = format!("Готово! Загружено файлов: {} из {total}", entries.len());
            prog.finished = true;
        }
    }

    launcher_log(&format!("[builds] ZIP extracted: {} files uploaded to {repo}", entries.len()));
    Ok(entries)
}