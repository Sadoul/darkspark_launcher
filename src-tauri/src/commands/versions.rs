use serde::{Deserialize, Serialize};
use std::path::PathBuf;


#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct McVersion {
    pub id: String,
    #[serde(rename = "type")]
    pub version_type: String,
}

#[derive(Debug, Deserialize)]
struct McManifest {
    versions: Vec<McVersion>,
}


#[tauri::command]
pub async fn get_mc_versions() -> Result<Vec<McVersion>, String> {
    let client = reqwest::Client::builder()
        .user_agent("RPWorld-Launcher/2.0")
        .build()
        .map_err(|e| e.to_string())?;

    let manifest: McManifest = client
        .get("https://launchermeta.mojang.com/mc/game/version_manifest_v2.json")
        .send()
        .await
        .map_err(|_| "Не удалось подключиться к Mojang API".to_string())?
        .json()
        .await
        .map_err(|e| format!("Ошибка разбора ответа Mojang: {e}"))?;

    Ok(manifest.versions)
}


#[derive(Debug, Serialize, Clone)]
pub struct LoaderVersion {
    pub id: String,
    pub stable: bool,
}


#[tauri::command]
pub async fn get_loader_versions(loader: String, mc_version: String) -> Result<Vec<LoaderVersion>, String> {
    match loader.as_str() {
        "fabric" => get_fabric_versions(&mc_version).await,
        "forge" => get_forge_versions(&mc_version).await,
        "neoforge" => get_neoforge_versions(&mc_version).await,
        "optifine" => get_optifine_versions(&mc_version).await,
        _ => Err(format!("Неизвестный загрузчик: {loader}")),
    }
}


#[derive(Deserialize)]
struct FabricLoaderEntry {
    loader: FabricLoaderInfo,
}

#[derive(Deserialize)]
struct FabricLoaderInfo {
    version: String,
    stable: bool,
}

async fn get_fabric_versions(mc_version: &str) -> Result<Vec<LoaderVersion>, String> {
    let client = reqwest::Client::builder()
        .user_agent("RPWorld-Launcher/2.0")
        .build()
        .map_err(|e| e.to_string())?;

    let url = format!(
        "https://meta.fabricmc.net/v2/versions/loader/{}",
        mc_version
    );

    let entries: Vec<FabricLoaderEntry> = client
        .get(&url)
        .send()
        .await
        .map_err(|_| "Не удалось подключиться к Fabric Meta API".to_string())?
        .json()
        .await
        .map_err(|e| format!("Ошибка разбора Fabric API: {e}"))?;

    Ok(entries
        .into_iter()
        .map(|e| LoaderVersion {
            id: e.loader.version,
            stable: e.loader.stable,
        })
        .collect())
}


#[derive(Deserialize)]
struct ForgePromoSlim {
    promos: std::collections::HashMap<String, String>,
}

async fn get_forge_versions(mc_version: &str) -> Result<Vec<LoaderVersion>, String> {
    let client = reqwest::Client::builder()
        .user_agent("RPWorld-Launcher/2.0")
        .build()
        .map_err(|e| e.to_string())?;


    let url = "https://files.minecraftforge.net/net/minecraftforge/forge/promotions_slim.json";

    let promos: ForgePromoSlim = client
        .get(url)
        .send()
        .await
        .map_err(|_| "Не удалось подключиться к Forge Maven".to_string())?
        .json()
        .await
        .map_err(|e| format!("Ошибка разбора Forge API: {e}"))?;

    let mut versions: Vec<LoaderVersion> = Vec::new();


    let rec_key = format!("{mc_version}-recommended");
    let lat_key = format!("{mc_version}-latest");

    if let Some(v) = promos.promos.get(&rec_key) {
        versions.push(LoaderVersion {
            id: format!("{mc_version}-{v}"),
            stable: true,
        });
    }
    if let Some(v) = promos.promos.get(&lat_key) {
        let id = format!("{mc_version}-{v}");
        if !versions.iter().any(|x| x.id == id) {
            versions.push(LoaderVersion {
                id,
                stable: false,
            });
        }
    }

    if versions.is_empty() {
        return Err(format!(
            "Forge не поддерживает Minecraft {mc_version} или версии временно недоступны"
        ));
    }

    Ok(versions)
}


#[derive(Deserialize)]
struct NeoForgeApiResponse {
    versions: Vec<String>,
}

async fn get_neoforge_versions(mc_version: &str) -> Result<Vec<LoaderVersion>, String> {
    let client = reqwest::Client::builder()
        .user_agent("RPWorld-Launcher/2.0")
        .build()
        .map_err(|e| e.to_string())?;


    let url = "https://maven.neoforged.net/api/maven/versions/releases/net/neoforged/neoforge";

    let resp: NeoForgeApiResponse = client
        .get(url)
        .send()
        .await
        .map_err(|_| "Не удалось подключиться к NeoForge Maven".to_string())?
        .json()
        .await
        .map_err(|e| format!("Ошибка разбора NeoForge API: {e}"))?;


    let mc_suffix = mc_version.trim_start_matches("1.");
    let mut filtered: Vec<LoaderVersion> = resp
        .versions
        .iter()
        .filter(|v| v.starts_with(mc_suffix))
        .rev()
        .take(20)
        .enumerate()
        .map(|(i, v)| LoaderVersion {
            id: v.clone(),
            stable: i == 0,
        })
        .collect();

    if filtered.is_empty() {
        return Err(format!("NeoForge не поддерживает Minecraft {mc_version}"));
    }

    Ok(filtered)
}


async fn get_optifine_versions(mc_version: &str) -> Result<Vec<LoaderVersion>, String> {

    let known: &[(&str, &[&str])] = &[
        ("1.20.1", &["HD_U_I6", "HD_U_I5", "HD_U_I4", "HD_U_I3", "HD_U_I2", "HD_U_I1"]),
        ("1.20",   &["HD_U_I3", "HD_U_I2", "HD_U_I1"]),
        ("1.19.4", &["HD_U_H9", "HD_U_H8", "HD_U_H7"]),
        ("1.19.2", &["HD_U_H9", "HD_U_H8", "HD_U_H7", "HD_U_H6", "HD_U_H5"]),
        ("1.18.2", &["HD_U_H7", "HD_U_H6", "HD_U_H5"]),
        ("1.17.1", &["HD_U_H1"]),
        ("1.16.5", &["HD_U_G8", "HD_U_G7", "HD_U_G6", "HD_U_G5"]),
    ];

    for (ver, builds) in known {
        if *ver == mc_version {
            return Ok(builds
                .iter()
                .enumerate()
                .map(|(i, b)| LoaderVersion {
                    id: format!("OptiFine_{ver}_{b}"),
                    stable: i == 0,
                })
                .collect());
        }
    }

    Err(format!("OptiFine не имеет известных версий для Minecraft {mc_version}. Проверьте https://optifine.net вручную."))
}


#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CustomModpackManifest {
    pub name: String,
    pub loader: String,
    pub mc_version: String,
    pub loader_version: String,
    pub max_memory: u32,
    pub jvm_args: String,
    pub created_at: String,
    #[serde(default)]
    pub game_dir: String,
}

fn custom_modpacks_root() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".darkspark")
        .join("modpacks")
}

fn sanitize_modpack_dir_name(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .collect::<String>()
        .trim_matches('_')
        .to_string()
}

#[tauri::command]
pub fn get_custom_modpacks() -> Result<Vec<CustomModpackManifest>, String> {
    let root = custom_modpacks_root();
    if !root.exists() {
        return Ok(Vec::new());
    }

    let mut modpacks = Vec::new();
    for entry in std::fs::read_dir(&root).map_err(|e| format!("Не удалось прочитать модпаки: {e}"))? {
        let entry = entry.map_err(|e| e.to_string())?;
        let manifest_path = entry.path().join("modpack.json");
        if manifest_path.exists() {
            let text = std::fs::read_to_string(&manifest_path)
                .map_err(|e| format!("Не удалось прочитать {}: {e}", manifest_path.display()))?;
            if let Ok(manifest) = serde_json::from_str::<CustomModpackManifest>(&text) {
                modpacks.push(manifest);
            }
        }
    }
    modpacks.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(modpacks)
}

#[tauri::command]
pub fn delete_custom_modpack(name: String) -> Result<(), String> {
    let dir_name = sanitize_modpack_dir_name(&name);
    if dir_name.is_empty() {
        return Err("Некорректное имя модпака".to_string());
    }
    let path = custom_modpacks_root().join(dir_name);
    if path.exists() {
        std::fs::remove_dir_all(&path)
            .map_err(|e| format!("Не удалось удалить модпак: {e}"))?;
    }
    Ok(())
}

#[tauri::command]
pub async fn install_custom_modpack(
    name: String,
    loader: String,
    mc_version: String,
    loader_version: String,
    max_memory: u32,
    jvm_args: String,
    previous_name: Option<String>,
) -> Result<(), String> {
    use std::fs;

    let dir_name = sanitize_modpack_dir_name(&name);
    if dir_name.is_empty() {
        return Err("Введите корректное название модпака".to_string());
    }

    let modpacks_dir = custom_modpacks_root().join(&dir_name);


    if let Some(prev) = previous_name.as_ref() {
        let prev_dir_name = sanitize_modpack_dir_name(prev);
        if !prev_dir_name.is_empty() && prev_dir_name != dir_name {
            let prev_dir = custom_modpacks_root().join(&prev_dir_name);
            if prev_dir.exists() {
                if modpacks_dir.exists() {
                    return Err(format!("Модпак с именем «{name}» уже существует. Выберите другое название."));
                }
                fs::rename(&prev_dir, &modpacks_dir)
                    .map_err(|e| format!("Не удалось переименовать модпак: {e}"))?;
            }
        }
    }

    fs::create_dir_all(&modpacks_dir)
        .map_err(|e| format!("Не удалось создать папку модпака: {e}"))?;


    let manifest = serde_json::json!({
        "name": name,
        "loader": loader,
        "mc_version": mc_version,
        "loader_version": loader_version,
        "max_memory": max_memory,
        "jvm_args": jvm_args,
        "created_at": chrono::Utc::now().to_rfc3339(),
        "game_dir": modpacks_dir.to_string_lossy(),
    });

    fs::write(
        modpacks_dir.join("modpack.json"),
        serde_json::to_string_pretty(&manifest).unwrap(),
    )
    .map_err(|e| format!("Не удалось записать манифест: {e}"))?;


    fs::create_dir_all(modpacks_dir.join("mods"))
        .map_err(|e| format!("Не удалось создать папку mods: {e}"))?;

    Ok(())
}