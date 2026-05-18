mod commands;

use commands::{auth, builds, downloader, java, launcher, logger, settings, updater, versions};
use tauri::Manager;

#[cfg(windows)]
fn ensure_single_instance_or_exit() {
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::{ERROR_ALREADY_EXISTS, GetLastError};
    use windows::Win32::System::Threading::CreateMutexW;

    let name: Vec<u16> = "Global\\DarkSparkLauncherSingleInstance\0".encode_utf16().collect();
    unsafe {
        let handle = CreateMutexW(None, false, PCWSTR(name.as_ptr()));
        if GetLastError() == ERROR_ALREADY_EXISTS {
            close_existing_launcher_processes();
            std::thread::sleep(std::time::Duration::from_millis(900));


            let retry_handle = CreateMutexW(None, false, PCWSTR(name.as_ptr()));
            if GetLastError() == ERROR_ALREADY_EXISTS {
                std::process::exit(0);
            }
            let _ = Box::leak(Box::new(retry_handle));
            return;
        }

        let _ = Box::leak(Box::new(handle));
    }
}

#[cfg(windows)]
fn close_existing_launcher_processes() {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x08000000;
    let current_pid = std::process::id().to_string();
    let script = format!(
        "Get-Process darkspark-launcher -ErrorAction SilentlyContinue | Where-Object {{ $_.Id -ne {} }} | Stop-Process -Force",
        current_pid
    );
    let _ = std::process::Command::new("powershell.exe")
        .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", &script])
        .creation_flags(CREATE_NO_WINDOW)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .stdin(std::process::Stdio::null())
        .status();
}

#[cfg(windows)]
fn set_windows_app_user_model_id() {
    use windows::core::HSTRING;
    use windows::Win32::UI::Shell::SetCurrentProcessExplicitAppUserModelID;

    let app_id = HSTRING::from("com.darkspark.launcher");
    unsafe {
        let _ = SetCurrentProcessExplicitAppUserModelID(&app_id);
    }
}

#[cfg(windows)]
fn force_windows_taskbar_icon(window: &tauri::WebviewWindow) {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use std::path::PathBuf;
    use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
    use windows::Win32::UI::WindowsAndMessaging::{
        LoadImageW, SendMessageW, IMAGE_ICON, LR_LOADFROMFILE, WM_SETICON,
    };

    let Ok(handle) = window.window_handle() else { return; };
    let RawWindowHandle::Win32(win32_handle) = handle.as_raw() else { return; };
    let hwnd = HWND(win32_handle.hwnd.get() as *mut _);

    let icon_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("icons").join("icon.ico");
    let icon_path_wide: Vec<u16> = OsStr::new(&icon_path)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    unsafe {
        let hicon_big = LoadImageW(
            None,
            windows::core::PCWSTR(icon_path_wide.as_ptr()),
            IMAGE_ICON,
            256,
            256,
            LR_LOADFROMFILE,
        );
        if let Ok(icon) = hicon_big {
            SendMessageW(hwnd, WM_SETICON, WPARAM(1), LPARAM(icon.0 as isize));
        }

        let hicon_small = LoadImageW(
            None,
            windows::core::PCWSTR(icon_path_wide.as_ptr()),
            IMAGE_ICON,
            64,
            64,
            LR_LOADFROMFILE,
        );
        if let Ok(icon) = hicon_small {
            SendMessageW(hwnd, WM_SETICON, WPARAM(0), LPARAM(icon.0 as isize));
        }
    }
}

#[tauri::command]
fn open_url(url: String) -> Result<(), String> {
    open::that(&url).map_err(|e| format!("Не удалось открыть URL: {e}"))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    #[cfg(windows)]
    {
        ensure_single_instance_or_exit();
        set_windows_app_user_model_id();
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_http::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.eval("window.addEventListener('contextmenu', event => event.preventDefault(), { capture: true });");
            }


            if let Some(window) = app.get_webview_window("main") {
                let _ = window.set_icon(tauri::include_image!("icons/128x128.png"));
                #[cfg(windows)]
                force_windows_taskbar_icon(&window);
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![

            auth::login_offline,
            auth::login_darkspark,
            auth::login_microsoft,
            auth::get_saved_account,
            auth::get_admin_token,
            auth::save_admin_token,
            auth::get_saved_theme,
            auth::save_theme,
            auth::get_saved_offline_profile,
            auth::save_offline_profile,
            auth::clear_offline_profile,
            auth::get_admin_accounts,
            auth::encrypt_admin_accounts,
            auth::commit_admin_accounts,
            auth::logout,

            builds::get_build_manifest,
            builds::commit_build_manifest,
            builds::upload_build_mod,
            builds::upload_modpack_build,
            builds::get_upload_progress,
            builds::get_build_download_dir,
            builds::set_build_download_dir,
            builds::download_build_mod_file,
            builds::download_build_bundle,


            launcher::launch_game,
            launcher::get_launch_progress,
            launcher::is_game_running,
            launcher::cancel_launch,
            launcher::sync_modpack_files,
            launcher::get_modpack_discord_url,
            launcher::update_cached_discord_url,
            launcher::get_modpack_manifest_hash,

            downloader::download_modpack,
            downloader::get_download_progress,
            downloader::check_modpack_update,
            downloader::cancel_download,

            updater::check_launcher_update,
            updater::update_launcher,
            updater::check_just_updated,

            java::find_java,
            java::download_java,

            versions::get_mc_versions,
            versions::get_loader_versions,
            versions::get_custom_modpacks,
            versions::delete_custom_modpack,
            versions::install_custom_modpack,

            settings::save_avatar,
            settings::get_avatar,
            settings::open_data_folder,
            settings::open_path,
            settings::get_builtin_modpack_dir,
            settings::open_builtin_modpack_folder,
            settings::delete_builtin_modpack,
            settings::delete_launcher,

            logger::set_logging_enabled,
            logger::get_log,
            logger::clear_log,
            logger::get_log_path,

            open_url,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}