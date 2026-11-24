mod accounts;
mod database;
#[cfg(all(debug_assertions, target_os = "windows"))]
mod diagnostics;
mod health;
#[cfg(all(debug_assertions, target_os = "windows"))]
mod manager_diagnostics;
mod probe;
mod profile_paths;
#[cfg(windows)]
mod windows_profiles;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
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
            #[cfg(all(debug_assertions, target_os = "windows"))]
            diagnostics::start(_app.handle())?;
            #[cfg(all(debug_assertions, target_os = "windows"))]
            manager_diagnostics::start(_app.handle())?;
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
            accounts::restart_mode
        ])
        .on_window_event(|window, event| {
            if window.label() == "main"
                && let tauri::WindowEvent::CloseRequested { api, .. } = event
                && !health::SHUTDOWN_FINISHED.load(std::sync::atomic::Ordering::SeqCst)
            {
                api.prevent_close();
                probe::request_shutdown(window.app_handle(), 0);
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
