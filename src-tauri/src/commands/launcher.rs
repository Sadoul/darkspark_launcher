use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};

use super::logger::log;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;
const INSTALLER_LOG_TAIL_LINES: usize = 80;
const GAME_EARLY_EXIT_CHECK_SECONDS: u64 = 12;
const GAME_LOG_TAIL_LINES: usize = 120;

static LAUNCH_PROGRESS: Mutex<Option<LaunchProgress>> = Mutex::new(None);
static GAME_RUNNING: AtomicBool = AtomicBool::new(false);
static LAUNCH_CANCELLED: AtomicBool = AtomicBool::new(false);

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LaunchProgress {
    pub stage: String,
    pub progress: f64,
    pub total: f64,
    pub message: String,
}

#[derive(Debug, Deserialize)]
struct VersionManifest {
    versions: Vec<VersionEntry>,
}

#[derive(Debug, Deserialize)]
struct VersionEntry {
    id: String,
    url: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct VersionInfo {
    id: String,
    #[serde(rename = "mainClass")]
    main_class: String,
    libraries: Vec<Library>,
    downloads: Option<Downloads>,
    #[serde(rename = "assetIndex")]
    asset_index: Option<AssetIndex>,
    assets: Option<String>,
    #[serde(rename = "minecraftArguments")]
    minecraft_arguments: Option<String>,
    arguments: Option<Arguments>,
    #[serde(rename = "inheritsFrom")]
    inherits_from: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Arguments {
    game: Option<Vec<serde_json::Value>>,
    jvm: Option<Vec<serde_json::Value>>,
}

#[derive(Debug, Deserialize)]
struct Downloads {
    client: Option<DownloadEntry>,
}

#[derive(Debug, Deserialize)]
struct DownloadEntry {
    url: String,
    #[allow(dead_code)]
    sha1: Option<String>,
    #[allow(dead_code)]
    size: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct AssetIndex {
    id: String,
    url: String,
}

#[derive(Debug, Deserialize)]
struct Library {
    name: String,
    downloads: Option<LibraryDownloads>,
    rules: Option<Vec<Rule>>,
    #[allow(dead_code)]
    url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct LibraryDownloads {
    artifact: Option<LibraryArtifact>,
}

#[derive(Debug, Deserialize)]
struct LibraryArtifact {
    path: String,
    url: String,
    #[allow(dead_code)]
    sha1: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Rule {
    action: String,
    os: Option<OsRule>,
}

#[derive(Debug, Deserialize)]
struct OsRule {
    name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AssetIndexFile {
    objects: HashMap<String, AssetObject>,
}

#[derive(Debug, Deserialize)]
struct AssetObject {
    hash: String,
    #[allow(dead_code)]
    size: u64,
}

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

fn default_enabled() -> bool { true }

fn get_minecraft_dir() -> PathBuf {
    let dir = dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".danganverse")
        .join("minecraft");
    fs::create_dir_all(&dir).ok();
    dir
}

fn set_progress(stage: &str, progress: f64, total: f64, message: &str) {
    log(&format!("[{}] {}", stage, message));
    if let Ok(mut p) = LAUNCH_PROGRESS.lock() {
        *p = Some(LaunchProgress {
            stage: stage.to_string(),
            progress,
            total,
            message: message.to_string(),
        });
    }
}

fn file_sha1(path: &PathBuf) -> Result<String, String> {
    let bytes = fs::read(path).map_err(|e| format!("sha1 read failed for {}: {}", path.display(), e))?;
    let mut hasher = Sha1::new();
    hasher.update(bytes);
    Ok(format!("{:x}", hasher.finalize()))
}

async fn download_file(client: &reqwest::Client, url: &str, path: &PathBuf) -> Result<(), String> {
    if LAUNCH_CANCELLED.load(Ordering::SeqCst) {
        return Err("Запуск Minecraft отменён".to_string());
    }
    if path.exists() {
        log(&format!("[download] Already exists, skipping: {}", path.display()));
        return Ok(());
    }

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("mkdir failed for {}: {}", parent.display(), e))?;
    }

    log(&format!("[download] Fetching: {}", url));
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("[download] Request failed for {}: {}", url, e))?;

    if !response.status().is_success() {
        return Err(format!("[download] HTTP {} for URL: {}", response.status(), url));
    }

    if LAUNCH_CANCELLED.load(Ordering::SeqCst) {
        return Err("Запуск Minecraft отменён".to_string());
    }

    let bytes = response
        .bytes()
        .await
        .map_err(|e| format!("[download] Read body failed for {}: {}", url, e))?;

    log(&format!("[download] Saving {} bytes to {}", bytes.len(), path.display()));
    fs::write(path, &bytes).map_err(|e| format!("[download] Write failed for {}: {}", path.display(), e))?;

    Ok(())
}

fn build_repo_for_modpack(name: &str) -> Option<&'static str> {
    match name.to_lowercase().as_str() {
        "danganverse" => Some("Sadoul/danganverse_modpack"),
        _ => None,
    }
}

async fn fetch_build_manifest(client: &reqwest::Client, repo: &str) -> Result<BuildManifest, String> {
    let url = format!("https://raw.githubusercontent.com/{}/main/manifest.json?t={}", repo, chrono::Utc::now().timestamp_millis());
    log(&format!("[build] Fetch manifest: {}", url));
    let response = client.get(&url).send().await.map_err(|e| format!("[build] Manifest request failed: {}", e))?;
    if !response.status().is_success() {
        return Err(format!("[build] Manifest HTTP {} for {}", response.status(), repo));
    }
    response.json::<BuildManifest>().await.map_err(|e| format!("[build] Manifest parse failed: {}", e))
}

async fn sync_build_files(client: &reqwest::Client, modpack_name: &str, mc_dir: &PathBuf) -> Result<Option<BuildManifest>, String> {
    let Some(repo) = build_repo_for_modpack(modpack_name) else { return Ok(None); };
    let manifest = fetch_build_manifest(client, repo).await?;
    let mods_dir = mc_dir.join("mods");
    fs::create_dir_all(&mods_dir).map_err(|e| format!("[build] Cannot create mods dir: {}", e))?;

    set_progress("build", 0.0, manifest.mods.len() as f64, "Синхронизация модов сборки...");
    let enabled: std::collections::HashMap<String, &BuildFileEntry> = manifest.mods.iter()
        .filter(|m| m.enabled)
        .map(|m| (m.name.clone(), m))
        .collect();

    for entry in manifest.mods.iter().filter(|m| m.enabled) {
        check_launch_cancelled()?;
        let path = mc_dir.join(&entry.path);
        let needs_download = if path.exists() {
            file_sha1(&path).map(|sha| sha != entry.sha1).unwrap_or(true)
        } else {
            true
        };
        if needs_download {
            set_progress("build", 0.0, manifest.mods.len() as f64, &format!("Скачивание мода {}", entry.name));
            download_file(client, &entry.url, &path).await?;
        }
    }

    for file in fs::read_dir(&mods_dir).map_err(|e| format!("[build] Cannot read mods dir: {}", e))? {
        let file = file.map_err(|e| e.to_string())?;
        if file.file_type().map_err(|e| e.to_string())?.is_file() {
            let name = file.file_name().to_string_lossy().to_string();
            if name.ends_with(".jar") && !enabled.contains_key(&name) {
                log(&format!("[build] Removing stale mod: {}", name));
                let _ = fs::remove_file(file.path());
            }
        }
    }
    set_progress("build", manifest.mods.len() as f64, manifest.mods.len() as f64, "Моды сборки синхронизированы");
    Ok(Some(manifest))
}

fn is_library_allowed(lib: &Library) -> bool {
    if let Some(rules) = &lib.rules {
        let mut allowed = false;
        for rule in rules {
            let os_match = match &rule.os {
                Some(os) => os.name.as_deref() == Some("windows"),
                None => true,
            };
            if os_match {
                allowed = rule.action == "allow";
            }
        }
        return allowed;
    }
    true
}


fn parse_modded_version(version: &str) -> Option<(String, String)> {
    if version.starts_with("forge-") {
        let mc = version.trim_start_matches("forge-").to_string();
        return Some((mc, "forge".to_string()));
    }
    if version.contains("-forge") {
        if let Some(mc) = version.split("-forge").next() {
            return Some((mc.to_string(), "forge".to_string()));
        }
    }
    if version.starts_with("fabric-") {
        let mc = version.trim_start_matches("fabric-").to_string();
        return Some((mc, "fabric".to_string()));
    }
    if version.starts_with("neoforge-") {
        let rest = version.trim_start_matches("neoforge-").to_string();
        return Some((rest, "neoforge".to_string()));
    }
    None
}


fn find_installed_forge(mc_dir: &PathBuf, mc_version: &str, loader: &str) -> Option<String> {
    let versions_dir = mc_dir.join("versions");
    if !versions_dir.exists() {
        return None;
    }
    if let Ok(entries) = fs::read_dir(&versions_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            let matches = match loader {
                "neoforge" => name.contains("neoforge"),
                _ => name.contains("forge") && !name.contains("neoforge"),
            };
            if matches && name.contains(mc_version) {
                let json = entry.path().join(format!("{}.json", name));
                if json.exists() {
                    log(&format!("[{}] Found existing install: {}", loader, name));
                    return Some(name);
                }
            }
        }
    }
    None
}


async fn ensure_vanilla(
    client: &reqwest::Client,
    mc_dir: &PathBuf,
    mc_version: &str,
) -> Result<(), String> {
    check_launch_cancelled()?;
    let version_dir = mc_dir.join("versions").join(mc_version);
    let version_json_path = version_dir.join(format!("{}.json", mc_version));

    if version_json_path.exists() {
        log(&format!("[vanilla] {} JSON already present", mc_version));
    } else {
        set_progress("vanilla", 0.0, 2.0, &format!("Загрузка манифеста Minecraft {}...", mc_version));
        let manifest_url = "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";
        log(&format!("[vanilla] Fetching version manifest: {}", manifest_url));

        let manifest: VersionManifest = client
            .get(manifest_url)
            .send()
            .await
            .map_err(|e| format!("[vanilla] Manifest request failed: {}", e))?
            .json()
            .await
            .map_err(|e| format!("[vanilla] Manifest parse failed: {}", e))?;

        let entry = manifest
            .versions
            .iter()
            .find(|v| v.id == mc_version)
            .ok_or_else(|| format!("[vanilla] Version {} not found in manifest", mc_version))?;

        set_progress("vanilla", 1.0, 2.0, &format!("Скачивание Minecraft {} JSON...", mc_version));
        download_file(client, &entry.url, &version_json_path).await?;
        log(&format!("[vanilla] Downloaded version JSON for {}", mc_version));
    }

    let version_info: VersionInfo = serde_json::from_str(
        &fs::read_to_string(&version_json_path)
            .map_err(|e| format!("[vanilla] Cannot read version JSON: {}", e))?,
    )
    .map_err(|e| format!("[vanilla] Cannot parse version JSON: {}", e))?;

    let client_jar = version_dir.join(format!("{}.jar", mc_version));
    if client_jar.exists() {
        log(&format!("[vanilla] client.jar already present for {}", mc_version));
    } else if let Some(dl) = &version_info.downloads {
        if let Some(c) = &dl.client {
            set_progress("vanilla", 2.0, 2.0, &format!("Скачивание client.jar для {}...", mc_version));
            log(&format!("[vanilla] Downloading client.jar from: {}", c.url));
            download_file(client, &c.url, &client_jar).await?;
            log(&format!("[vanilla] client.jar downloaded for {}", mc_version));
        }
    } else {
        log(&format!("[vanilla] WARNING: No client download entry in JSON for {}", mc_version));
    }

    log(&format!("[vanilla] {} ready", mc_version));
    Ok(())
}


async fn get_latest_forge_version(
    client: &reqwest::Client,
    mc_version: &str,
) -> Result<String, String> {
    let url = "https://maven.minecraftforge.net/net/minecraftforge/forge/maven-metadata.xml";
    log(&format!("[forge] Fetching maven metadata: {}", url));

    let xml = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("[forge] Maven metadata request failed: {}", e))?
        .text()
        .await
        .map_err(|e| format!("[forge] Maven metadata read failed: {}", e))?;

    let prefix = format!("{}-", mc_version);
    let mut matching: Vec<String> = Vec::new();

    for line in xml.lines() {
        let t = line.trim();
        if t.starts_with("<version>") && t.ends_with("</version>") {
            let v = t
                .trim_start_matches("<version>")
                .trim_end_matches("</version>")
                .to_string();
            if v.starts_with(&prefix) {
                matching.push(v);
            }
        }
    }

    log(&format!("[forge] Found {} matching versions for MC {}", matching.len(), mc_version));
    let chosen = matching
        .into_iter()
        .last()
        .ok_or_else(|| format!("[forge] No Forge version found for MC {}", mc_version))?;

    log(&format!("[forge] Selected version: {}", chosen));
    Ok(chosen)
}

fn log_process_output(prefix: &str, output: &str) {
    if output.trim().is_empty() {
        log(&format!("[{}] <empty>", prefix));
        return;
    }

    for line in output.lines() {
        log(&format!("[{}] {}", prefix, line));
    }
}

fn tail_lines(output: &str, max_lines: usize) -> String {
    let mut lines = output.lines().rev().take(max_lines).collect::<Vec<_>>();
    lines.reverse();
    lines.join("\n")
}


#[cfg(windows)]
fn javaw_path(java_path: &str) -> String {

    let p = std::path::Path::new(java_path);
    let javaw = if java_path.to_lowercase().ends_with("java.exe") {
        p.with_file_name("javaw.exe")
    } else if java_path.to_lowercase().ends_with("java") {
        p.with_file_name("javaw")
    } else {
        p.to_path_buf()
    };

    if javaw.exists() {
        javaw.to_string_lossy().to_string()
    } else {
        java_path.to_string()
    }
}

#[cfg(not(windows))]
fn javaw_path(java_path: &str) -> String {
    java_path.to_string()
}


async fn run_modded_installer(
    client: &reqwest::Client,
    java_path: &str,
    mc_dir: &PathBuf,
    installer_url: &str,
    loader: &str,
    mc_version: &str,
) -> Result<String, String> {
    check_launch_cancelled()?;


    let profiles_path = mc_dir.join("launcher_profiles.json");
    if !profiles_path.exists() {
        log(&format!("[{}] Creating launcher_profiles.json required by Forge installer", loader));
        let profiles_json = "{\"profiles\":{},\"selectedProfile\":\"\",\"clientToken\":\"rpwlauncher\",\"authenticationDatabase\":{}}";
        fs::write(&profiles_path, profiles_json)
            .map_err(|e| format!("[{}] Cannot create launcher_profiles.json: {}", loader, e))?;
    }
    let installer_path = mc_dir.join(format!("{}-installer.jar", loader));
    if installer_path.exists() {
        let _ = fs::remove_file(&installer_path);
    }

    set_progress(loader, 1.0, 4.0, &format!("Скачивание {} installer...", loader));
    log(&format!("[{}] Installer URL: {}", loader, installer_url));
    download_file(client, installer_url, &installer_path).await?;
    check_launch_cancelled()?;

    let installer_size = fs::metadata(&installer_path)
        .map(|metadata| metadata.len())
        .unwrap_or_default();
    log(&format!(
        "[{}] Installer downloaded to: {} ({} bytes)",
        loader,
        installer_path.display(),
        installer_size
    ));

    check_launch_cancelled()?;
    set_progress(loader, 2.0, 4.0, &format!("Запуск {} installer в фоне...", loader));
    log(&format!(
        "[{}] Running hidden: {} -Djava.awt.headless=true -jar {} --installClient in {}",
        loader,
        java_path,
        installer_path.display(),
        mc_dir.display()
    ));


    let java_exe = javaw_path(java_path);
    log(&format!("[{}] Java executable for installer: {}", loader, java_exe));

    let installer_str = installer_path.to_str().unwrap_or_default().to_string();
    let mc_dir_str = mc_dir.clone();
    let loader_name = loader.to_string();

    let output = tokio::task::spawn_blocking(move || {
        let mut cmd = Command::new(&java_exe);
        cmd.args([
                "-Djava.awt.headless=true",
                "-jar",
                &installer_str,
                "--installClient",
            ])
            .current_dir(&mc_dir_str)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }

        cmd.output()
            .map_err(|e| format!("[{}] Failed to spawn installer: {}", loader_name, e))
    })
    .await
    .map_err(|e| format!("[{}] spawn_blocking failed: {}", loader, e))?
    .map_err(|e| e)?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    log_process_output(&format!("{}-installer stdout", loader), &stdout);
    log_process_output(&format!("{}-installer stderr", loader), &stderr);

    let _ = fs::remove_file(&installer_path);

    if !output.status.success() {
        let code = output.status.code().unwrap_or(-1);
        let stdout_tail = tail_lines(&stdout, INSTALLER_LOG_TAIL_LINES);
        let stderr_tail = tail_lines(&stderr, INSTALLER_LOG_TAIL_LINES);
        let msg = format!(
            "[{}] Installer exited with code {}. Java: {}. Game dir: {}. Installer URL: {}. Last stdout:\n{}\nLast stderr:\n{}",
            loader,
            code,
            java_path,
            mc_dir.display(),
            installer_url,
            stdout_tail,
            stderr_tail
        );
        log(&msg);
        return Err(msg);
    }

    log(&format!("[{}] Installer completed successfully (exit 0)", loader));

    set_progress(loader, 3.0, 4.0, "Поиск установленной версии...");
    let found = find_installed_forge(mc_dir, mc_version, loader)
        .ok_or_else(|| format!("[{}] Installer succeeded but version not found in versions dir", loader))?;

    log(&format!("[{}] Installed version: {}", loader, found));
    Ok(found)
}


async fn install_forge(
    client: &reqwest::Client,
    java_path: &str,
    mc_dir: &PathBuf,
    mc_version: &str,
) -> Result<String, String> {
    check_launch_cancelled()?;
    set_progress("forge", 0.0, 4.0, "Поиск последней версии Forge...");
    let forge_ver = get_latest_forge_version(client, mc_version).await?;
    log(&format!("[forge] Will install: {}", forge_ver));

    let installer_url = format!(
        "https://maven.minecraftforge.net/net/minecraftforge/forge/{}/forge-{}-installer.jar",
        forge_ver, forge_ver
    );

    run_modded_installer(client, java_path, mc_dir, &installer_url, "forge", mc_version).await
}


async fn install_neoforge(
    client: &reqwest::Client,
    java_path: &str,
    mc_dir: &PathBuf,
    version_str: &str,
) -> Result<String, String> {
    check_launch_cancelled()?;
    set_progress("neoforge", 0.0, 4.0, "Поиск последней версии NeoForge...");

    let neo_ver = get_latest_neoforge_version(client, version_str).await?;
    log(&format!("[neoforge] Will install: {}", neo_ver));

    let installer_url = format!(
        "https://maven.neoforged.net/releases/net/neoforged/neoforge/{}/neoforge-{}-installer.jar",
        neo_ver, neo_ver
    );

    let mc_version = version_str_to_mc(version_str);
    run_modded_installer(client, java_path, mc_dir, &installer_url, "neoforge", &mc_version).await
}

fn version_str_to_mc(s: &str) -> String {


    if s.starts_with("1.") {
        return s.to_string();
    }

    let parts: Vec<&str> = s.splitn(3, '.').collect();
    if parts.len() >= 2 {
        return format!("1.{}.{}", parts[0], parts[1]);
    }
    s.to_string()
}

async fn get_latest_neoforge_version(
    client: &reqwest::Client,
    version_str: &str,
) -> Result<String, String> {
    let url = "https://maven.neoforged.net/releases/net/neoforged/neoforge/maven-metadata.xml";
    log(&format!("[neoforge] Fetching maven metadata: {}", url));

    let xml = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("[neoforge] Maven metadata request failed: {}", e))?
        .text()
        .await
        .map_err(|e| format!("[neoforge] Maven metadata read failed: {}", e))?;


    let prefix = if version_str.starts_with("1.") {

        let parts: Vec<&str> = version_str.trim_start_matches("1.").splitn(2, '.').collect();
        if parts.len() == 2 {
            format!("{}.{}.", parts[0], parts[1])
        } else {
            format!("{}.", parts[0])
        }
    } else {
        format!("{}.", version_str)
    };

    log(&format!("[neoforge] Searching versions with prefix: {}", prefix));
    let mut matching: Vec<String> = Vec::new();

    for line in xml.lines() {
        let t = line.trim();
        if t.starts_with("<version>") && t.ends_with("</version>") {
            let v = t
                .trim_start_matches("<version>")
                .trim_end_matches("</version>")
                .to_string();
            if v.starts_with(&prefix) {
                matching.push(v);
            }
        }
    }

    log(&format!("[neoforge] Found {} matching versions", matching.len()));
    matching
        .into_iter()
        .last()
        .ok_or_else(|| format!("[neoforge] No NeoForge version found for {}", version_str))
}


async fn install_fabric(
    client: &reqwest::Client,
    mc_dir: &PathBuf,
    mc_version: &str,
) -> Result<String, String> {
    check_launch_cancelled()?;
    set_progress("fabric", 0.0, 3.0, "Получение информации о Fabric loader...");


    let loaders_url = "https://meta.fabricmc.net/v2/versions/loader";
    log(&format!("[fabric] Fetching loader versions: {}", loaders_url));

    let loaders: Vec<serde_json::Value> = client
        .get(loaders_url)
        .send()
        .await
        .map_err(|e| format!("[fabric] Loader list request failed: {}", e))?
        .json()
        .await
        .map_err(|e| format!("[fabric] Loader list parse failed: {}", e))?;

    let loader_version = loaders
        .iter()
        .find(|l| l["stable"].as_bool().unwrap_or(false))
        .and_then(|l| l["version"].as_str())
        .ok_or_else(|| "[fabric] No stable loader version found".to_string())?
        .to_string();

    log(&format!("[fabric] Latest stable loader: {}", loader_version));

    let version_id = format!("fabric-loader-{}-{}", loader_version, mc_version);
    let version_dir = mc_dir.join("versions").join(&version_id);
    let profile_json_path = version_dir.join(format!("{}.json", version_id));

    if profile_json_path.exists() {
        log(&format!("[fabric] Already installed: {}", version_id));
        return Ok(version_id);
    }

    set_progress("fabric", 1.0, 3.0, &format!("Скачивание Fabric profile для {}...", mc_version));
    let profile_url = format!(
        "https://meta.fabricmc.net/v2/versions/loader/{}/{}/profile/json",
        mc_version, loader_version
    );
    log(&format!("[fabric] Profile URL: {}", profile_url));

    fs::create_dir_all(&version_dir)
        .map_err(|e| format!("[fabric] Cannot create version dir: {}", e))?;

    download_file(client, &profile_url, &profile_json_path).await
        .map_err(|e| format!("[fabric] Profile download failed: {}", e))?;

    set_progress("fabric", 2.0, 3.0, "Установка vanilla для Fabric...");
    ensure_vanilla(client, mc_dir, mc_version).await?;

    log(&format!("[fabric] Fabric installed: {}", version_id));
    Ok(version_id)
}

#[tauri::command]
pub fn is_game_running() -> bool {
    GAME_RUNNING.load(Ordering::SeqCst)
}

#[tauri::command]
pub fn cancel_launch() {
    log("[launch] Cancel requested by user");
    LAUNCH_CANCELLED.store(true, Ordering::SeqCst);

    if let Ok(mut p) = LAUNCH_PROGRESS.lock() {
        *p = Some(LaunchProgress {
            stage: "done".into(),
            progress: 0.0,
            total: 0.0,
            message: "Запуск отменён".into(),
        });
    }
}

fn check_launch_cancelled() -> Result<(), String> {
    if LAUNCH_CANCELLED.load(Ordering::SeqCst) {
        return Err("Запуск Minecraft отменён".to_string());
    }
    Ok(())
}

#[tauri::command]
pub async fn launch_game(
    username: String,
    uuid: String,
    access_token: String,
    version: String,
    java_path: String,
    max_memory: u32,
    game_dir: Option<String>,
    jvm_args: Option<String>,
    gpu_mode: Option<String>,
    allow_multiple_instances: Option<bool>,
    close_launcher_on_game_start: Option<bool>,
    reopen_launcher_after_game_close: Option<bool>,
) -> Result<String, String> {
    LAUNCH_CANCELLED.store(false, Ordering::SeqCst);
    log(&format!("=== Launch requested: version={}, user={}", version, username));

    if !allow_multiple_instances.unwrap_or(false) && is_game_running() {
        return Err("Minecraft уже запущен. Включите «Разрешить твинки» в настройках, если хотите открыть ещё один клиент.".to_string());
    }

    let client = reqwest::Client::builder()
        .user_agent("DanganVerseLauncher/2.10")
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .map_err(|e| e.to_string())?;

    let mc_dir = game_dir
        .map(PathBuf::from)
        .unwrap_or_else(get_minecraft_dir);

    log(&format!("[launch] Game dir: {}", mc_dir.display()));
    log(&format!("[launch] Java: {}", java_path));
    log(&format!("[launch] Memory: {}M", max_memory));

    fs::create_dir_all(&mc_dir)
        .map_err(|e| format!("[launch] Cannot create game dir: {}", e))?;

    let mut launch_version = version.clone();
    let mut server_ip: Option<String> = None;
    if let Some(modpack_name) = mc_dir.file_name().and_then(|n| n.to_str()) {
        if let Some(manifest) = sync_build_files(&client, modpack_name, &mc_dir).await? {
            let loader = manifest.loader.to_lowercase();
            launch_version = if loader == "vanilla" {
                manifest.minecraft_version.clone()
            } else {
                format!("{}-{}", loader, manifest.minecraft_version)
            };
            log(&format!("[build] Launch version overridden by manifest: {}", launch_version));
            server_ip = manifest.server_ip;
            if let Some(ref ip) = server_ip {
                log(&format!("[build] Server IP set: {}", ip));
            }
        }
    }


    let actual_version = if let Some((mc_ver, loader)) = parse_modded_version(&launch_version) {
        log(&format!("[launch] Detected loader: {} for MC {}", loader, mc_ver));

        match loader.as_str() {
            "forge" => {
                if let Some(existing) = find_installed_forge(&mc_dir, &mc_ver, "forge") {
                    existing
                } else {
                    log(&format!("[forge] Not installed, starting installation for MC {}", mc_ver));
                    ensure_vanilla(&client, &mc_dir, &mc_ver).await?;
                    install_forge(&client, &java_path, &mc_dir, &mc_ver).await?
                }
            }
            "neoforge" => {
                if let Some(existing) = find_installed_forge(&mc_dir, &mc_ver, "neoforge") {
                    existing
                } else {
                    log(&format!("[neoforge] Not installed, starting installation for {}", mc_ver));
                    let mc_version = version_str_to_mc(&mc_ver);
                    ensure_vanilla(&client, &mc_dir, &mc_version).await?;
                    install_neoforge(&client, &java_path, &mc_dir, &mc_ver).await?
                }
            }
            "fabric" => {
                install_fabric(&client, &mc_dir, &mc_ver).await?
            }
            _ => {
                log(&format!("[launch] Unknown loader {}, treating as vanilla", loader));
                mc_ver
            }
        }
    } else {
        log(&format!("[launch] Vanilla version: {}", version));
        version.clone()
    };

    check_launch_cancelled()?;
    log(&format!("[launch] Using version ID: {}", actual_version));


    set_progress("version", 0.0, 1.0, "Загрузка информации о версии...");

    let version_dir = mc_dir.join("versions").join(&actual_version);
    let version_json_path = version_dir.join(format!("{}.json", actual_version));

    if !version_json_path.exists() {
        log(&format!("[launch] Version JSON missing, downloading vanilla {}", actual_version));
        let manifest_url = "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";
        let manifest: VersionManifest = client
            .get(manifest_url)
            .send()
            .await
            .map_err(|e| e.to_string())?
            .json()
            .await
            .map_err(|e| e.to_string())?;
        let entry = manifest
            .versions
            .iter()
            .find(|v| v.id == actual_version)
            .ok_or_else(|| format!("Version {} not found in manifest", actual_version))?;
        download_file(&client, &entry.url, &version_json_path).await?;
    }

    log(&format!("[launch] Reading version JSON: {}", version_json_path.display()));
    let version_info: VersionInfo = serde_json::from_str(
        &fs::read_to_string(&version_json_path)
            .map_err(|e| format!("[launch] Cannot read version JSON: {}", e))?,
    )
    .map_err(|e| format!("[launch] Cannot parse version JSON: {}", e))?;

    log(&format!("[launch] mainClass: {}", version_info.main_class));
    if let Some(ref inh) = version_info.inherits_from {
        log(&format!("[launch] inheritsFrom: {}", inh));
    }


    let (base_version_info, base_version_dir) = if let Some(ref base) = version_info.inherits_from {
        let base_dir = mc_dir.join("versions").join(base);
        let base_json = base_dir.join(format!("{}.json", base));

        if !base_json.exists() {
            log(&format!("[launch] Base version {} not found, ensuring vanilla", base));
            ensure_vanilla(&client, &mc_dir, base).await?;
        } else {
            log(&format!("[launch] Base version {} JSON present", base));
        }

        let info: VersionInfo = serde_json::from_str(
            &fs::read_to_string(&base_json)
                .map_err(|e| format!("[launch] Cannot read base JSON: {}", e))?,
        )
        .map_err(|e| format!("[launch] Cannot parse base JSON: {}", e))?;

        (Some(info), Some(base_dir))
    } else {
        (None, None)
    };


    set_progress("client", 0.0, 1.0, "Проверка client.jar...");

    let effective_downloads = version_info
        .downloads
        .as_ref()
        .or_else(|| base_version_info.as_ref().and_then(|b| b.downloads.as_ref()));

    let effective_version_dir = base_version_dir.as_ref().unwrap_or(&version_dir);
    let effective_version_id = version_info.inherits_from.as_deref().unwrap_or(&actual_version);
    let client_jar_path = effective_version_dir.join(format!("{}.jar", effective_version_id));

    log(&format!("[launch] client.jar path: {}", client_jar_path.display()));

    if let Some(dl) = effective_downloads {
        if let Some(c) = &dl.client {
            download_file(&client, &c.url, &client_jar_path).await?;
        }
    } else {
        log("[launch] WARNING: No downloads entry found for client jar");
    }


    let mut all_libs: Vec<&Library> = Vec::new();
    if let Some(ref base) = base_version_info {
        all_libs.extend(base.libraries.iter());
    }
    all_libs.extend(version_info.libraries.iter());

    let total_libs = all_libs.len();
    log(&format!("[launch] Total libraries: {}", total_libs));

    let mut classpath_entries: Vec<String> = Vec::new();
    let mut skipped = 0usize;
    let mut downloaded = 0usize;

    for (i, lib) in all_libs.iter().enumerate() {
        if !is_library_allowed(lib) {
            skipped += 1;
            continue;
        }

        if i % 20 == 0 {
            set_progress(
                "libraries",
                i as f64,
                total_libs as f64,
                &format!("Библиотеки {}/{} (скачано: {})", i + 1, total_libs, downloaded),
            );
        }

        if let Some(dl) = &lib.downloads {
            if let Some(artifact) = &dl.artifact {
                let lib_path = mc_dir.join("libraries").join(&artifact.path);
                if !lib_path.exists() {
                    log(&format!("[libs] Downloading: {} from {}", lib.name, artifact.url));
                    downloaded += 1;
                }
                download_file(&client, &artifact.url, &lib_path).await
                    .map_err(|e| format!("[libs] Failed to download {}: {}", lib.name, e))?;
                classpath_entries.push(lib_path.to_string_lossy().to_string());
            }
        }
    }

    log(&format!("[launch] Libraries: {} total, {} skipped by rules, {} downloaded", total_libs, skipped, downloaded));

    let uses_bootstrap_launcher = version_info.main_class.contains("BootstrapLauncher");
    if uses_bootstrap_launcher && version_info.inherits_from.is_some() {
        log("[launch] Skipping inherited vanilla client jar in classpath for Forge/NeoForge module launch");
    } else {
        classpath_entries.push(client_jar_path.to_string_lossy().to_string());
    }
    log(&format!("[launch] Classpath entries: {}", classpath_entries.len()));


    let effective_asset_index = version_info
        .asset_index
        .as_ref()
        .or_else(|| base_version_info.as_ref().and_then(|b| b.asset_index.as_ref()));

    if let Some(asset_index) = effective_asset_index {
        set_progress("assets", 0.0, 1.0, &format!("Загрузка asset index {}...", asset_index.id));
        log(&format!("[assets] Index ID: {}, URL: {}", asset_index.id, asset_index.url));

        let asset_index_path = mc_dir
            .join("assets")
            .join("indexes")
            .join(format!("{}.json", asset_index.id));
        download_file(&client, &asset_index.url, &asset_index_path).await?;

        let asset_json = fs::read_to_string(&asset_index_path)
            .map_err(|e| format!("[assets] Cannot read index: {}", e))?;
        let assets: AssetIndexFile =
            serde_json::from_str(&asset_json).map_err(|e| format!("[assets] Parse error: {}", e))?;

        let total_assets = assets.objects.len();
        log(&format!("[assets] Total objects: {}", total_assets));
        let mut assets_downloaded = 0usize;

        for (i, (_name, obj)) in assets.objects.iter().enumerate() {
            if i % 100 == 0 {
                set_progress(
                    "assets",
                    i as f64,
                    total_assets as f64,
                    &format!("Ресурсы {}/{} (скачано: {})", i, total_assets, assets_downloaded),
                );
            }
            let hash_prefix = &obj.hash[..2];
            let asset_path = mc_dir
                .join("assets")
                .join("objects")
                .join(hash_prefix)
                .join(&obj.hash);
            if !asset_path.exists() {
                let asset_url = format!(
                    "https://resources.download.minecraft.net/{}/{}",
                    hash_prefix, obj.hash
                );
                download_file(&client, &asset_url, &asset_path).await?;
                assets_downloaded += 1;
            }
        }
        log(&format!("[assets] Done. Downloaded {} new assets", assets_downloaded));
    } else {
        log("[launch] WARNING: No asset index in version JSON");
    }


    set_progress("launch", 1.0, 1.0, "Сборка команды запуска...");

    let classpath = classpath_entries.join(";");
    let assets_dir = mc_dir.join("assets");
    let effective_assets = version_info
        .assets
        .clone()
        .or_else(|| base_version_info.as_ref().and_then(|b| b.assets.clone()))
        .unwrap_or_else(|| actual_version.clone());

    let natives_dir = version_dir.join("natives");

    let mut jvm_arg_list: Vec<String> = vec![format!("-Xmx{}M", max_memory)];

    let gpu = gpu_mode.as_deref().unwrap_or("auto");
    if gpu == "discrete" {
        jvm_arg_list.push("-Dsun.java2d.opengl=true".to_string());
    } else if gpu == "integrated" {
        jvm_arg_list.push("-Dsun.java2d.opengl=false".to_string());
    }

    if let Some(ref extra) = jvm_args {
        let trimmed = extra.trim();
        if !trimmed.is_empty() {
            for arg in trimmed.split_whitespace() {
                jvm_arg_list.push(arg.to_string());
            }
        }
    } else {
        jvm_arg_list.extend([
            "-XX:+UnlockExperimentalVMOptions".to_string(),
            "-XX:+UseG1GC".to_string(),
            "-XX:G1NewSizePercent=20".to_string(),
            "-XX:G1ReservePercent=20".to_string(),
            "-XX:MaxGCPauseMillis=50".to_string(),
            "-XX:G1HeapRegionSize=32M".to_string(),
        ]);
    }

    jvm_arg_list.push(format!("-Djava.library.path={}", natives_dir.to_string_lossy()));

    let main_class = version_info.main_class.clone();


    if let Some(ref args_obj) = version_info.arguments {
        if let Some(jvm_args_list) = &args_obj.jvm {
            for arg in jvm_args_list {
                if let Some(s) = arg.as_str() {
                    let replaced = s
                        .replace("${classpath}", &classpath)
                        .replace("${natives_directory}", &natives_dir.to_string_lossy())
                        .replace("${launcher_name}", "DanganVerseLauncher")
                        .replace("${launcher_version}", "2.10")
                        .replace("${library_directory}", &mc_dir.join("libraries").to_string_lossy())
                        .replace("${classpath_separator}", ";");
                    jvm_arg_list.push(replaced);
                }
            }
        }
    }

    jvm_arg_list.push("-cp".to_string());
    jvm_arg_list.push(classpath.clone());
    jvm_arg_list.push(main_class.clone());

    log(&format!("[launch] mainClass: {}", main_class));

    let mut args = jvm_arg_list;


    let effective_mc_args = version_info
        .minecraft_arguments
        .as_ref()
        .or_else(|| base_version_info.as_ref().and_then(|b| b.minecraft_arguments.as_ref()));

    if let Some(mc_args_str) = effective_mc_args {
        log("[launch] Using legacy minecraftArguments format");
        let game_args = mc_args_str
            .replace("${auth_player_name}", &username)
            .replace("${version_name}", &actual_version)
            .replace("${game_directory}", &mc_dir.to_string_lossy())
            .replace("${assets_root}", &assets_dir.to_string_lossy())
            .replace("${assets_index_name}", &effective_assets)
            .replace("${auth_uuid}", &uuid)
            .replace("${auth_access_token}", &access_token)
            .replace("${user_type}", "mojang")
            .replace("${version_type}", "DanganVerseLauncher");
        args.extend(game_args.split_whitespace().map(|s| s.to_string()));
    } else {
        log("[launch] Using modern arguments.game format");
        let check_args = |info: &VersionInfo| {
            info.arguments
                .as_ref()
                .and_then(|a| a.game.as_ref())
                .cloned()
        };
        let mut merged_game_args = Vec::new();
        if let Some(game_args) = check_args(&version_info) {
            merged_game_args.extend(game_args);
        }
        if let Some(base_args) = base_version_info.as_ref().and_then(|b| check_args(b)) {
            merged_game_args.extend(base_args);
        }

        if merged_game_args.is_empty() {
            log("[launch] WARNING: No game arguments found in version JSON");
        } else {
            log(&format!("[launch] Game arguments merged: {}", merged_game_args.len()));
            for arg in &merged_game_args {
                if let Some(s) = arg.as_str() {
                    let replaced = s
                        .replace("${auth_player_name}", &username)
                        .replace("${version_name}", &actual_version)
                        .replace("${game_directory}", &mc_dir.to_string_lossy())
                        .replace("${assets_root}", &assets_dir.to_string_lossy())
                        .replace("${assets_index_name}", &effective_assets)
                        .replace("${auth_uuid}", &uuid)
                        .replace("${auth_access_token}", &access_token)
                        .replace("${clientid}", "")
                        .replace("${auth_xuid}", "")
                        .replace("${user_properties}", "{}")
                        .replace("${user_type}", "mojang")
                        .replace("${version_type}", "DanganVerseLauncher");
                    args.push(replaced);
                }
            }
        }
    }

    // Add server IP args if configured
    if let Some(ref ip) = server_ip {
        let (host, port) = if ip.contains(':') {
            let parts: Vec<&str> = ip.splitn(2, ':').collect();
            (parts[0].to_string(), parts[1].parse::<u16>().unwrap_or(25565))
        } else {
            (ip.clone(), 25565u16)
        };
        args.push("--server".to_string());
        args.push(host);
        args.push("--port".to_string());
        args.push(port.to_string());
        log(&format!("[launch] Added server args: {}:{}", ip, port));
    }

    log(&format!("[launch] Total args: {}", args.len()));
    log(&format!("[launch] First 10 args: {:?}", &args[..args.len().min(10)]));

    check_launch_cancelled()?;
    set_progress("launch", 1.0, 1.0, "Запуск игры...");

    let logs_dir = mc_dir.join("logs");
    fs::create_dir_all(&logs_dir)
        .map_err(|e| format!("[launch] Cannot create logs dir: {}", e))?;

    let stdout_path = logs_dir.join("game-stdout.log");
    let stderr_path = logs_dir.join("game-stderr.log");
    let stdout_file = fs::File::create(&stdout_path)
        .map_err(|e| format!("[launch] Cannot create stdout log: {}", e))?;
    let stderr_file = fs::File::create(&stderr_path)
        .map_err(|e| format!("[launch] Cannot create stderr log: {}", e))?;

    let mut cmd = Command::new(&java_path);
    cmd.args(&args)
        .current_dir(&mc_dir)
        .stdout(Stdio::from(stdout_file))
        .stderr(Stdio::from(stderr_file));

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }

    if gpu == "discrete" {
        cmd.env("__NV_PRIME_RENDER_OFFLOAD", "1");
        cmd.env("__GLX_VENDOR_LIBRARY_NAME", "nvidia");
        cmd.env("__VK_LAYER_NV_optimus", "NVIDIA_only");
    }

    let mut child = cmd
        .spawn()
        .map_err(|e| {
            let msg = format!("[launch] Failed to spawn game: {}", e);
            log(&msg);
            msg
        })?;

    log("[launch] Game process spawned successfully");

    for _ in 0..GAME_EARLY_EXIT_CHECK_SECONDS {
        if LAUNCH_CANCELLED.load(Ordering::SeqCst) {
            let _ = child.kill();
            return Err("Запуск Minecraft отменён".to_string());
        }
        std::thread::sleep(Duration::from_secs(1));
        match child.try_wait() {
            Ok(Some(status)) => {
                let code = status.code().map_or_else(|| "unknown".to_string(), |c| c.to_string());
                let latest_log = logs_dir.join("latest.log");
                let debug_log = logs_dir.join("debug.log");
                let stderr_tail = fs::read_to_string(&stderr_path)
                    .map(|s| tail_lines(&s, GAME_LOG_TAIL_LINES))
                    .unwrap_or_default();
                let latest_tail = fs::read_to_string(&latest_log)
                    .map(|s| tail_lines(&s, GAME_LOG_TAIL_LINES))
                    .unwrap_or_default();
                let debug_tail = fs::read_to_string(&debug_log)
                    .map(|s| tail_lines(&s, GAME_LOG_TAIL_LINES))
                    .unwrap_or_default();
                let msg = format!(
                    "Minecraft закрылся во время загрузки (exit code {}).\n\nПоследние строки stderr:\n{}\n\nПоследние строки latest.log:\n{}\n\nПоследние строки debug.log:\n{}",
                    code,
                    stderr_tail,
                    latest_tail,
                    debug_tail
                );
                log(&format!("[launch] Early game exit detected: {}", msg));
                return Err(msg);
            }
            Ok(None) => {}
            Err(e) => {
                let msg = format!("[launch] Failed to check game process: {}", e);
                log(&msg);
                return Err(msg);
            }
        }
    }

    log("[launch] Game is still running after early-exit check");
    set_progress("done", 1.0, 1.0, "Игра запущена!");

    let should_close_launcher = close_launcher_on_game_start.unwrap_or(true);
    let should_reopen_launcher = reopen_launcher_after_game_close.unwrap_or(true);
    if !allow_multiple_instances.unwrap_or(false) {
        GAME_RUNNING.store(true, Ordering::SeqCst);
    }

    let exe = std::env::current_exe().ok();

    if should_close_launcher {
        if should_reopen_launcher {
            if let Some(exe_path) = exe {
                let game_pid = child.id();
                log(&format!("[launch] Starting external game watcher for PID {}", game_pid));
                #[cfg(windows)]
                {
                    let script = format!(
                        "Wait-Process -Id {}; Start-Process -FilePath '{}'",
                        game_pid,
                        exe_path.to_string_lossy().replace('\'', "''")
                    );
                    let mut watcher = Command::new("powershell.exe");
                    watcher
                        .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-WindowStyle", "Hidden", "-Command", &script])
                        .stdin(Stdio::null())
                        .stdout(Stdio::null())
                        .stderr(Stdio::null());
                    use std::os::windows::process::CommandExt;
                    watcher.creation_flags(CREATE_NO_WINDOW);
                    if let Err(e) = watcher.spawn() {
                        log(&format!("[launch] Failed to start external watcher: {}", e));
                    }
                }
                #[cfg(not(windows))]
                {
                    std::thread::spawn(move || {
                        let _ = child.wait();
                        let _ = Command::new(exe_path).spawn();
                    });
                    std::thread::sleep(Duration::from_millis(300));
                }
            }
        }
        std::process::exit(0);
    }

    std::thread::spawn(move || {
        let _ = child.wait();
        GAME_RUNNING.store(false, Ordering::SeqCst);
    });

    Ok("Game launched successfully".to_string())
}

#[tauri::command]
pub async fn get_launch_progress() -> Result<Option<LaunchProgress>, String> {
    Ok(LAUNCH_PROGRESS.lock().map_err(|e| e.to_string())?.clone())
}