use base64::{engine::general_purpose, Engine as _};
use std::fs;
use std::path::{Path, PathBuf};
use tauri::AppHandle;

fn data_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".danganverse")
}

fn path_to_data_url(path: &Path) -> Option<String> {
    let ext = path.extension()?.to_str()?.to_lowercase();
    let mime = match ext.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        _ => "image/png",
    };
    let data = fs::read(path).ok()?;
    let b64 = general_purpose::STANDARD.encode(&data);
    Some(format!("data:{};base64,{}", mime, b64))
}


#[tauri::command]
pub async fn save_avatar(source_path: String) -> Result<String, String> {
    let src = PathBuf::from(&source_path);
    let ext = src
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("png")
        .to_lowercase();

    let dest_dir = data_dir();
    fs::create_dir_all(&dest_dir).map_err(|e| format!("Не удалось создать папку: {e}"))?;

    let dest = dest_dir.join(format!("avatar.{ext}"));
    fs::copy(&src, &dest).map_err(|e| format!("Не удалось скопировать аватарку: {e}"))?;

    path_to_data_url(&dest).ok_or_else(|| "Не удалось прочитать аватарку".to_string())
}


#[tauri::command]
pub fn get_avatar() -> Option<String> {
    let base = data_dir();
    for ext in ["gif", "webp", "png", "jpg", "jpeg"] {
        let p = base.join(format!("avatar.{ext}"));
        if p.exists() {
            return path_to_data_url(&p);
        }
    }
    None
}


fn open_folder_path(dir: &Path) -> Result<(), String> {
    fs::create_dir_all(dir).map_err(|e| format!("Не удалось создать папку: {e}"))?;

    #[cfg(windows)]
    {
        std::process::Command::new("explorer")
            .arg(dir.to_str().unwrap_or("."))
            .spawn()
            .map_err(|e| format!("Не удалось открыть папку: {e}"))?;
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(dir.to_str().unwrap_or("."))
            .spawn()
            .map_err(|e| format!("Не удалось открыть папку: {e}"))?;
    }
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(dir.to_str().unwrap_or("."))
            .spawn()
            .map_err(|e| format!("Не удалось открыть папку: {e}"))?;
    }

    Ok(())
}

fn builtin_modpacks_root() -> PathBuf {
    data_dir().join("modpacks")
}

fn sanitize_modpack_name(name: &str) -> Result<String, String> {
    match name {
        "danganverse" => Ok(name.to_string()),
        _ => Err("Эту встроенную сборку нельзя изменить этой командой".to_string()),
    }
}

#[tauri::command]
pub fn open_data_folder() -> Result<(), String> {
    open_folder_path(&data_dir())
}

#[tauri::command]
pub fn open_path(path: String) -> Result<(), String> {
    open_folder_path(&PathBuf::from(path))
}

#[tauri::command]
pub fn get_builtin_modpack_dir(modpack_name: String) -> Result<String, String> {
    let safe_name = sanitize_modpack_name(&modpack_name)?;
    Ok(builtin_modpacks_root()
        .join(safe_name)
        .to_string_lossy()
        .to_string())
}

#[tauri::command]
pub fn open_builtin_modpack_folder(modpack_name: String) -> Result<(), String> {
    let safe_name = sanitize_modpack_name(&modpack_name)?;
    open_folder_path(&builtin_modpacks_root().join(safe_name))
}

#[tauri::command]
pub fn delete_builtin_modpack(modpack_name: String) -> Result<(), String> {
    let safe_name = sanitize_modpack_name(&modpack_name)?;
    let dir = builtin_modpacks_root().join(safe_name);
    if dir.exists() {
        fs::remove_dir_all(&dir).map_err(|e| format!("Не удалось удалить сборку: {e}"))?;
    }
    Ok(())
}

// ─── Delete launcher ──────────────────────────────────────────────────────────

#[tauri::command]
pub async fn delete_launcher(app: AppHandle) -> Result<(), String> {
    let dir = data_dir();
    if dir.exists() {
        fs::remove_dir_all(&dir).map_err(|e| format!("Не удалось удалить данные: {e}"))?;
    }

    #[cfg(windows)]
    {
        let uninstall_keys = [
            r"Software\Microsoft\Windows\CurrentVersion\Uninstall\com.darkspark.launcher_is1",
            r"Software\Microsoft\Windows\CurrentVersion\Uninstall\DarkSpark Launcher_is1",
        ];

        use winreg::enums::*;
        use winreg::RegKey;

        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        for key_path in &uninstall_keys {
            if let Ok(key) = hkcu.open_subkey(key_path) {
                if let Ok(uninstall_str) = key.get_value::<String, _>("UninstallString") {
                    let _ = std::process::Command::new(&uninstall_str)
                        .args(["/S"])
                        .spawn();
                    break;
                }
            }
        }
    }

    app.exit(0);
    Ok(())
}
