mod accounts;
mod app_lock;
mod database;
mod desktop;
#[cfg(all(debug_assertions, target_os = "windows"))]
mod desktop_diagnostics;
mod desktop_policy;
#[cfg(all(debug_assertions, target_os = "windows"))]
mod diagnostics;
mod health;
#[cfg(all(debug_assertions, target_os = "windows"))]
mod manager_diagnostics;
mod platforms;
mod probe;
mod profile_paths;
mod tray;
#[cfg(windows)]
mod windows_desktop;
#[cfg(windows)]
mod windows_downloads;
#[cfg(windows)]
mod windows_notifications;
#[cfg(windows)]
mod windows_permissions;
#[cfg(windows)]
mod windows_profiles;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    if std::env::args().any(|a| a == "--wait-for-instance")
        && let Some(local) = std::env::var_os("LOCALAPPDATA")
    {
        let path = std::path::PathBuf::from(local).join("com.appterisk.messenger/application.lock");
        let start = std::time::Instant::now();
        while accounts::acquire(&path).is_err()
            && start.elapsed() < std::time::Duration::from_secs(8)
        {
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
    }
    let mut builder = tauri::Builder::default();
    if !accounts::synthetic_mode() {
        builder = builder.plugin(tauri_plugin_single_instance::init(|app, args, _| {
            desktop::second_instance(app, args)
        }));
    }
    builder
        .plugin(tauri_plugin_dialog::init())
        .manage(probe::ProbeState::default())
        .setup(|_app| {
            health::startup(_app.handle());
            let mut root = _app.path().app_local_data_dir()?;
            if accounts::synthetic_mode() {
                let args: Vec<_> = std::env::args().collect();
                let run = args.last().ok_or("Missing synthetic test run ID")?;
                // Synthetic fixtures never initialize the real metadata database.
                if uuid::Uuid::parse_str(run).is_err() && run != "--verbatim-path" {
                    return Err("Invalid synthetic run ID".into());
                }
                let run = args
                    .iter()
                    .find(|a| uuid::Uuid::parse_str(a).is_ok())
                    .ok_or("Missing synthetic run ID")?;
                root = root.join("diagnostic-runs").join(run).join("manager");
            }
            _app.manage(accounts::AccountState::open(root).map_err(std::io::Error::other)?);
            desktop::setup(_app.handle()).map_err(std::io::Error::other)?;
            if accounts::synthetic_mode()
                && std::env::args().any(|a| a == "--ui-diagnostic")
                && let Some(w) = _app.get_window("main")
            {
                w.set_title("Appterisk 0.3.0 — UI review")?;
            }
            #[cfg(all(debug_assertions, target_os = "windows"))]
            diagnostics::start(_app.handle())?;
            #[cfg(all(debug_assertions, target_os = "windows"))]
            manager_diagnostics::start(_app.handle())?;
            #[cfg(all(debug_assertions, target_os = "windows"))]
            desktop_diagnostics::start(_app.handle())?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            probe::open_probe,
            probe::set_probe_bounds,
            probe::set_probe_visibility,
            probe::reload_probe,
            probe::capture_profile_metadata,
            probe::resume_probe_on_startup,
            accounts::account_snapshot,
            accounts::account_add,
            accounts::account_update,
            accounts::account_reorder,
            accounts::account_switch,
            accounts::account_layout,
            accounts::account_reload,
            accounts::account_remove,
            accounts::settings_save,
            accounts::restart_mode,
            accounts::accounts_initialize,
            desktop::desktop_snapshot,
            desktop::desktop_save,
            desktop::desktop_lock,
            desktop::desktop_unlock,
            desktop::desktop_password,
            desktop::desktop_download_directory,
            desktop::desktop_download_cancel,
            desktop::desktop_permission,
            desktop::desktop_permission_reply
        ])
        .on_window_event(|window, event| {
            if window.label() == "main"
                && let tauri::WindowEvent::CloseRequested { api, .. } = event
                && !health::SHUTDOWN_FINISHED.load(std::sync::atomic::Ordering::SeqCst)
            {
                api.prevent_close();
                if tray::close_to_tray(window.app_handle()) && !health::closing() {
                    let _ = window.hide();
                    return;
                }
                probe::request_shutdown(window.app_handle(), 0);
            }
            if window.label() == "main"
                && matches!(event, tauri::WindowEvent::Resized(_))
                && window.is_minimized().unwrap_or(false)
            {
                tray::minimized(window.app_handle());
            }
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| {
            if let tauri::RunEvent::ExitRequested { api, code, .. } = event
                && !health::SHUTDOWN_FINISHED.load(std::sync::atomic::Ordering::SeqCst)
            {
                api.prevent_exit();
                probe::request_shutdown(app, code.unwrap_or(0));
            }
        });
}
use tauri::Manager;
