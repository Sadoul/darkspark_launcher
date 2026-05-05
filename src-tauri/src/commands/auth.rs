use base64::{engine::general_purpose, Engine as _};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

const ACCOUNTS_KEY: &[u8] = b"DarkSparkLauncherFriendsOnlyKey_v1";
const ADMIN_USERNAME: &str = "Sadoul";
const ACCOUNTS_REPO_API: &str = "https://api.github.com/repos/Sadoul/darkspark_launcher/contents/public/auth/offline_accounts.rpwenc";
const ACCOUNTS_BRANCH: &str = "main";

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Account {
    pub username: String,
    pub uuid: String,
    pub access_token: String,
    pub account_type: String,
    #[serde(default)]
    pub is_admin: bool,
    #[serde(default)]
    pub is_owner: bool,
    #[serde(default)]
    pub role: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OfflineCredential {
    pub username: String,
    pub password: String,
    #[serde(default)]
    pub role: String,
}


#[derive(Debug, Serialize, Deserialize)]
struct OfflineCredentialFile {
    accounts: Vec<OfflineCredential>,
}

#[derive(Debug, Deserialize)]
struct GitHubContentResponse {
    sha: String,
    #[serde(default)]
    content: String,
}

#[derive(Debug, Deserialize)]
struct GitHubCommitFileResponse {
    content: GitHubContentResponse,
    commit: GitHubCommitInfo,
}

#[derive(Debug, Deserialize)]
struct GitHubCommitInfo {
    sha: String,
    html_url: String,
}

fn get_config_dir() -> PathBuf {
    let dir = dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".rpworld");
    fs::create_dir_all(&dir).ok();
    dir
}

fn get_account_file() -> PathBuf {
    get_config_dir().join("account.json")
}

fn get_admin_token_file() -> PathBuf {
    get_config_dir().join("admin_token.txt")
}

fn get_theme_file() -> PathBuf {
    get_config_dir().join("theme.txt")
}

fn get_offline_profile_file() -> PathBuf {
    get_config_dir().join("offline_profile.json")
}

#[tauri::command]
pub async fn get_saved_theme() -> Result<String, String> {
    let path = get_theme_file();
    if !path.exists() {
        return Ok(String::new());
    }
    fs::read_to_string(path).map(|s| s.trim().to_string()).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn save_theme(theme: String) -> Result<(), String> {
    let value = theme.trim();
    if value != "light" && value != "dark" {
        return Err("Неверная тема".to_string());
    }
    fs::write(get_theme_file(), value).map_err(|e| e.to_string())
}

fn get_accounts_cache_file() -> PathBuf {
    get_config_dir().join("offline_accounts.rpwenc")
}

fn xor_bytes(data: &[u8]) -> Vec<u8> {
    data.iter()
        .enumerate()
        .map(|(i, byte)| byte ^ ACCOUNTS_KEY[i % ACCOUNTS_KEY.len()])
        .collect()
}

fn decrypt_accounts_payload(encrypted: &str) -> Result<OfflineCredentialFile, String> {
    let compact = encrypted.trim().replace(['\r', '\n', ' '], "");
    let encrypted_bytes = general_purpose::STANDARD
        .decode(compact)
        .map_err(|e| format!("Не удалось прочитать файл аккаунтов: {e}"))?;
    let json_bytes = xor_bytes(&encrypted_bytes);
    let json = String::from_utf8(json_bytes)
        .map_err(|e| format!("Файл аккаунтов повреждён: {e}"))?;
    serde_json::from_str(&json).map_err(|e| format!("Ошибка JSON аккаунтов: {e}"))
}

fn encrypt_accounts_payload(accounts: &OfflineCredentialFile) -> Result<String, String> {
    let json = serde_json::to_string(accounts).map_err(|e| e.to_string())?;
    Ok(general_purpose::STANDARD.encode(xor_bytes(json.as_bytes())))
}

async fn load_accounts() -> Result<OfflineCredentialFile, String> {
    let client = reqwest::Client::builder()
        .user_agent("RPWLauncher/Accounts")
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| e.to_string())?;

    let remote = client
        .get(ACCOUNTS_REPO_API)
        .query(&[("ref", ACCOUNTS_BRANCH), ("cache_bust", &chrono::Utc::now().timestamp_millis().to_string())])
        .send()
        .await;

    if let Ok(response) = remote {
        if response.status().is_success() {
            if let Ok(file) = response.json::<GitHubContentResponse>().await {
                let encrypted = file.content.replace(['\r', '\n', ' '], "");
                if let Ok(accounts) = decrypt_accounts_payload(&encrypted) {
                    let _ = fs::write(get_accounts_cache_file(), encrypted);
                    return Ok(accounts);
                }
            }
        }
    }

    if let Ok(cached) = fs::read_to_string(get_accounts_cache_file()) {
        if let Ok(accounts) = decrypt_accounts_payload(&cached) {
            return Ok(accounts);
        }
    }

    decrypt_accounts_payload(include_str!("../../../public/auth/offline_accounts.rpwenc"))
}

fn is_owner(username: &str) -> bool {
    username.eq_ignore_ascii_case(ADMIN_USERNAME)
}

fn is_moderator(account: &OfflineCredential) -> bool {
    account.role.eq_ignore_ascii_case("moderator")
}

async fn has_admin_panel_access(username: &str) -> Result<bool, String> {
    if is_owner(username) {
        return Ok(true);
    }
    let accounts = load_accounts().await?;
    Ok(accounts.accounts.iter().any(|account| {
        account.username.eq_ignore_ascii_case(username) && is_moderator(account)
    }))
}

fn build_account(credential: &OfflineCredential) -> Account {
    let owner = is_owner(&credential.username);
    let moderator = is_moderator(credential);
    Account {
        username: credential.username.clone(),
        uuid: uuid::Uuid::new_v4().to_string().replace('-', ""),
        access_token: "0".to_string(),
        account_type: "rpworld".to_string(),
        is_admin: owner || moderator,
        is_owner: owner,
        role: if owner { "owner".to_string() } else { credential.role.clone() },
    }
}


#[derive(Debug, Serialize, Deserialize)]
pub struct SavedOfflineProfile {
    pub username: String,
    pub password: String,
}

#[tauri::command]
pub async fn get_saved_offline_profile() -> Result<Option<SavedOfflineProfile>, String> {
    let path = get_offline_profile_file();
    if !path.exists() {
        return Ok(None);
    }
    let content = fs::read_to_string(path).map_err(|e| e.to_string())?;
    serde_json::from_str(&content).map(Some).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn save_offline_profile(username: String, password: String) -> Result<(), String> {
    let profile = SavedOfflineProfile {
        username: username.trim().to_string(),
        password,
    };
    if profile.username.is_empty() || profile.password.is_empty() {
        return Err("Ник и пароль не могут быть пустыми".to_string());
    }
    let json = serde_json::to_string_pretty(&profile).map_err(|e| e.to_string())?;
    fs::write(get_offline_profile_file(), json).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn clear_offline_profile() -> Result<(), String> {
    let path = get_offline_profile_file();
    if path.exists() {
        fs::remove_file(path).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub async fn login_offline(username: String) -> Result<Account, String> {
    let username = username.trim().to_string();
    if username.len() < 3 || username.len() > 16 || !username.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Err("Ник должен быть 3-16 символов: латиница, цифры и _".to_string());
    }
    let credentials = load_accounts().await?;
    if credentials.accounts.iter().any(|account| account.username.eq_ignore_ascii_case(&username)) {
        return Err("Этот ник занят RPWorld аккаунтом. Используйте вход RPWorld аккаунт.".to_string());
    }

    let account = Account {
        username,
        uuid: uuid::Uuid::new_v4().to_string().replace('-', ""),
        access_token: "0".to_string(),
        account_type: "offline".to_string(),
        is_admin: false,
        is_owner: false,
        role: String::new(),
    };
    let json = serde_json::to_string_pretty(&account).map_err(|e| e.to_string())?;
    fs::write(get_account_file(), json).map_err(|e| e.to_string())?;
    Ok(account)
}

#[tauri::command]
pub async fn login_rpworld(username: String, password: String) -> Result<Account, String> {
    let username = username.trim().to_string();
    let credentials = load_accounts().await?;
    let expected = credentials
        .accounts
        .iter()
        .find(|account| account.username.eq_ignore_ascii_case(&username))
        .ok_or_else(|| "Аккаунт не найден в списке RPWorld".to_string())?;

    if expected.password != password {
        return Err("Неверный пароль".to_string());
    }

    let account = build_account(expected);
    let json = serde_json::to_string_pretty(&account).map_err(|e| e.to_string())?;
    fs::write(get_account_file(), json).map_err(|e| e.to_string())?;
    Ok(account)
}

#[tauri::command]
pub async fn get_admin_token(current_username: String) -> Result<String, String> {
    if !has_admin_panel_access(&current_username).await? {
        return Err("Доступ запрещён".to_string());
    }
    let path = get_admin_token_file();

    if !path.exists() {
        return Ok(String::new());
    }
    fs::read_to_string(path).map(|s| s.trim().to_string()).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn save_admin_token(current_username: String, github_token: String) -> Result<(), String> {
    if !has_admin_panel_access(&current_username).await? {
        return Err("Доступ запрещён".to_string());
    }
    fs::write(get_admin_token_file(), github_token.trim()).map_err(|e| e.to_string())
}


#[tauri::command]
pub async fn get_admin_accounts(current_username: String) -> Result<Vec<OfflineCredential>, String> {
    if !has_admin_panel_access(&current_username).await? {
        return Err("Доступ запрещён".to_string());
    }
    let accounts = load_accounts().await?;
    if is_owner(&current_username) {
        return Ok(accounts.accounts);
    }
    Ok(accounts
        .accounts
        .into_iter()
        .filter(|account| !is_owner(&account.username))
        .collect())
}


fn normalized_accounts(accounts: Vec<OfflineCredential>) -> Result<OfflineCredentialFile, String> {
    let mut result: Vec<OfflineCredential> = Vec::new();
    for account in accounts {
        let username = account.username.trim().to_string();
        let password = account.password.trim().to_string();
        if username.is_empty() || password.is_empty() {
            return Err("Ник и пароль не могут быть пустыми".to_string());
        }
        if result.iter().any(|existing| existing.username.eq_ignore_ascii_case(&username)) {
            return Err(format!("Дубликат аккаунта: {username}"));
        }
        let role = if username.eq_ignore_ascii_case(ADMIN_USERNAME) {
            "owner".to_string()
        } else if account.role.eq_ignore_ascii_case("moderator") {
            "moderator".to_string()
        } else {
            String::new()
        };
        result.push(OfflineCredential { username, password, role });

    }
    if !result.iter().any(|a| a.username.eq_ignore_ascii_case(ADMIN_USERNAME)) {
        return Err("Нельзя удалить аккаунт Sadoul".to_string());
    }
    Ok(OfflineCredentialFile { accounts: result })
}

#[tauri::command]
pub async fn encrypt_admin_accounts(accounts: Vec<OfflineCredential>) -> Result<String, String> {
    encrypt_accounts_payload(&normalized_accounts(accounts)?)
}

#[tauri::command]
pub async fn commit_admin_accounts(
    current_username: String,
    github_token: String,
    accounts: Vec<OfflineCredential>,
) -> Result<String, String> {
    if !has_admin_panel_access(&current_username).await? {
        return Err("Доступ запрещён".to_string());
    }
    let owner = is_owner(&current_username);
    let credential_file = if owner {
        normalized_accounts(accounts.clone())?
    } else {
        let current_accounts = load_accounts().await?;
        if accounts.iter().any(|account| is_owner(&account.username)) {
            return Err("Модератор не может видеть или менять аккаунт Sadoul".to_string());
        }

        let submitted_regular = normalized_accounts({
            let mut list = accounts.clone();
            if !list.iter().any(|account| is_owner(&account.username)) {
                list.push(OfflineCredential {
                    username: ADMIN_USERNAME.to_string(),
                    password: "preserved".to_string(),
                    role: "owner".to_string(),
                });
            }
            list
        })?;

        let mut merged: Vec<OfflineCredential> = Vec::new();
        for current in &current_accounts.accounts {
            if is_owner(&current.username) || !current.role.is_empty() {
                let password = submitted_regular
                    .accounts
                    .iter()
                    .find(|next| next.username.eq_ignore_ascii_case(&current.username))
                    .map(|next| next.password.clone())
                    .unwrap_or_else(|| current.password.clone());
                merged.push(OfflineCredential {
                    username: current.username.clone(),
                    password,
                    role: current.role.clone(),
                });
            }
        }
        for next in submitted_regular.accounts {
            if is_owner(&next.username) || current_accounts.accounts.iter().any(|current| current.username.eq_ignore_ascii_case(&next.username) && !current.role.is_empty()) {
                continue;
            }
            merged.push(OfflineCredential {
                username: next.username,
                password: next.password,
                role: String::new(),
            });
        }
        normalized_accounts(merged)?
    };

    let token = github_token.trim();

    if token.is_empty() {
        return Err("Введите GitHub token с доступом Contents: Read and write".to_string());
    }
    fs::write(get_admin_token_file(), token).map_err(|e| format!("Не удалось сохранить токен: {e}"))?;

    let encrypted = encrypt_accounts_payload(&credential_file)?;
    let client = reqwest::Client::builder()
        .user_agent("RPWLauncher-AdminPanel")
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| e.to_string())?;

    let current = client
        .get(ACCOUNTS_REPO_API)
        .bearer_auth(token)
        .query(&[("ref", ACCOUNTS_BRANCH)])
        .send()
        .await
        .map_err(|e| format!("Не удалось получить текущий файл с GitHub: {e}"))?;

    if !current.status().is_success() {
        let status = current.status();
        let body = current.text().await.unwrap_or_default();
        return Err(format!("GitHub не отдал текущий файл: {status}. {body}"));
    }

    let current: GitHubContentResponse = current
        .json()
        .await
        .map_err(|e| format!("Не удалось разобрать ответ GitHub: {e}"))?;

    let payload = serde_json::json!({
        "message": "chore: update offline account passwords from launcher admin panel",
        "content": general_purpose::STANDARD.encode(encrypted.as_bytes()),
        "sha": current.sha,
        "branch": ACCOUNTS_BRANCH,
    });

    let update = client
        .put(ACCOUNTS_REPO_API)
        .bearer_auth(token)
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("Не удалось отправить commit на GitHub: {e}"))?;

    if !update.status().is_success() {
        let status = update.status();
        let body = update.text().await.unwrap_or_default();
        return Err(format!("GitHub отклонил commit: {status}. {body}"));
    }

    let updated: GitHubCommitFileResponse = update
        .json()
        .await
        .map_err(|e| format!("Commit создан, но ответ GitHub не разобран: {e}"))?;
    fs::write(get_accounts_cache_file(), &encrypted)
        .map_err(|e| format!("Commit создан, но локальный cache не сохранён: {e}"))?;

    Ok(format!(
        "Пароли обновлены. Commit: {}\n{}\nНовый SHA файла: {}",
        updated.commit.sha,
        updated.commit.html_url,
        updated.content.sha
    ))
}

#[tauri::command]
pub async fn login_microsoft() -> Result<Account, String> {
    let client_id = "00000000402b5328";
    let redirect_uri = "https://login.live.com/oauth20_desktop.srf";
    let auth_url = format!(
        "https://login.live.com/oauth20_authorize.srf?client_id={}&response_type=code&redirect_uri={}&scope=XboxLive.signin%20offline_access",
        client_id, redirect_uri
    );
    open::that(&auth_url).map_err(|e| e.to_string())?;
    Err("Microsoft auth requires user to complete login in browser. Full implementation pending - please use offline mode for now.".to_string())
}

#[tauri::command]
pub async fn get_saved_account() -> Result<Option<Account>, String> {
    let path = get_account_file();
    if !path.exists() {
        return Ok(None);
    }

    let content = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let mut account: Account = serde_json::from_str(&content).map_err(|e| e.to_string())?;
    account.is_owner = is_owner(&account.username);
    if account.is_owner {
        account.is_admin = true;
        account.role = "owner".to_string();
    } else if let Ok(accounts) = load_accounts().await {
        if let Some(credential) = accounts.accounts.iter().find(|item| item.username.eq_ignore_ascii_case(&account.username)) {
            account.is_admin = is_moderator(credential);
            account.role = credential.role.clone();
        }
    }

    Ok(Some(account))
}

#[tauri::command]
pub async fn logout() -> Result<(), String> {
    let path = get_account_file();
    if path.exists() {
        fs::remove_file(path).map_err(|e| e.to_string())?;
    }
    Ok(())
}