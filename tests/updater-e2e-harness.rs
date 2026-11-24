use crate::{
    accounts::{self, AccountState},
    diagnostics::evaluate,
    updater,
};
use serde_json::json;
use std::{
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};
use tauri::{Manager, WebviewUrl, webview::WebviewBuilder};
static BLOCK_ONCE: AtomicBool = AtomicBool::new(true);
pub fn block_shutdown(app: &tauri::AppHandle) -> bool {
    std::env::args().any(|a| a == "--updater-e2e")
        && app.package_info().version.to_string() == "0.5.0-e2e.1"
        && BLOCK_ONCE.swap(false, Ordering::SeqCst)
}
pub fn start(app: &tauri::AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    if !std::env::args().any(|a| a == "--updater-e2e") {
        return Ok(());
    }
    if app.config().identifier != "com.appterisk.messenger.updater-e2e" {
        return Err("E2E must use its dedicated test identifier".into());
    }
    let app = app.clone();
    std::thread::spawn(move || {
        let result = tauri::async_runtime::block_on(execute(&app));
        if let Err(error) = result {
            let root = app.path().app_local_data_dir().unwrap();
            let _ = std::fs::write(
                root.join("e2e-failure.json"),
                json!({"passed":false,"error":error}).to_string(),
            );
            app.exit(1);
        }
    });
    Ok(())
}
async fn execute(app: &tauri::AppHandle) -> Result<(), String> {
    let args: Vec<_> = std::env::args().collect();
    let run = args.last().ok_or("Missing E2E run")?;
    uuid::Uuid::parse_str(run).map_err(|_| "Invalid E2E run")?;
    let version = app.package_info().version.to_string();
    let verify = version == "0.5.1-e2e.1";
    if !verify && version != "0.5.0-e2e.1" {
        return Err("Unexpected E2E binary version".into());
    }
    let directory = app
        .path()
        .app_local_data_dir()
        .map_err(|_| "No test data directory")?;
    let ids_file = directory.join("expected-ids.json");
    let s = app.state::<AccountState>();
    {
        let mut r = s.0.lock().await;
        if !verify {
            if !r.db.accounts()?.is_empty() {
                return Err("E2E baseline must start with empty metadata".into());
            }
            for platform in ["whatsapp", "telegram", "discord"] {
                r.db.add_platform(&format!("Synthetic {platform}"), None, platform)?;
            }
            let mut settings = r.db.settings()?;
            settings.theme = "dark".into();
            settings.language = "zh-TW".into();
            r.db.save_settings(&settings)?;
            let mut config = r.db.desktop_config()?;
            config.sound = false;
            config.hide_previews = true;
            config.lock_timeout_seconds = 600;
            r.db.save_desktop_config(&config)?;
            std::fs::write(
                &ids_file,
                serde_json::to_vec(
                    &r.db
                        .accounts()?
                        .iter()
                        .map(|a| a.id.clone())
                        .collect::<Vec<_>>(),
                )
                .unwrap(),
            )
            .map_err(|_| "Cannot write fixture IDs")?;
        }
    }
    let expected: Vec<String> = serde_json::from_slice(
        &std::fs::read(&ids_file).map_err(|_| "Missing baseline fixture IDs")?,
    )
    .map_err(|_| "Invalid IDs")?;
    let list = { s.0.lock().await.db.accounts()? };
    if list.iter().map(|a| a.id.clone()).collect::<Vec<_>>() != expected {
        return Err("Metadata UUIDs/order changed".into());
    }
    for (index, account) in list.iter().enumerate() {
        let path = {
            let r = s.0.lock().await;
            crate::database::profile_path(&r.root, account)?
        };
        std::fs::create_dir_all(&path).map_err(|_| "Cannot prepare fixture profile")?;
        let canonical = path
            .canonicalize()
            .map_err(|_| "Cannot resolve fixture profile")?;
        let guard = accounts::acquire(&canonical.join("appterisk-profile.lock"))?;
        let window = app.get_window("main").ok_or("No test window")?;
        let view=window.add_child(WebviewBuilder::new(accounts::label(&account.id),WebviewUrl::App("isolation.html".into()))
   .data_directory(crate::profile_paths::browser_directory(&canonical)?).incognito(false).devtools(false)
   .initialization_script(format!("window.__isolationConfig={};",json!({"stage":if verify{"verify"}else{"seed"},"marker":format!("{run}-{index}")})))
   .initialization_script(include_str!("../diagnostics/storage.js")),tauri::LogicalPosition::new(-32000.0,-32000.0),tauri::LogicalSize::new(700.0,480.0)).map_err(|_|"Cannot create synthetic browser")?;
        view.hide().map_err(|_| "Cannot hide synthetic browser")?;
        let browser = crate::windows_profiles::attach(app, &view, &account.id, &canonical)?;
        {
            let mut r = s.0.lock().await;
            r.track_browser(&account.id, browser);
            r.track_fixture_lock(&account.id, guard);
        }
        let mut result = serde_json::Value::Null;
        for _ in 0..150 {
            result = evaluate(&view, "window.__isolationResult??null")?;
            if !result.is_null() {
                break;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        if result["cookie"] != format!("{run}-{index}")
            || result["local"] != format!("{run}-{index}")
            || result["indexedDb"] != format!("{run}-{index}")
            || result["blobOk"] != true
            || result["cryptoOk"] != true
        {
            return Err(format!("Synthetic persistence failed: {result}"));
        }
    }
    let caller = app.get_webview("main").ok_or("No trusted test shell")?;
    if verify {
        let r = s.0.lock().await;
        let settings = r.db.settings()?;
        if settings.theme != "dark"
            || settings.language != "zh-TW"
            || r.db.desktop_config()?.sound
            || r.db.desktop_config()?.lock_timeout_seconds != 600
        {
            return Err("Preferences changed during update".into());
        }
        let preferences =
            r.db.read_value("updater")?
                .ok_or("Update preferences missing")?;
        if !preferences.contains("beta") {
            return Err("Channel preference lost".into());
        }
        drop(r);
        std::fs::write(directory.join("e2e-passed.json"),json!({"passed":true,"version":version,"profiles":3,"metadata_ids_preserved":true,"durable_cookie_localstorage_idb_blob_key":true,"preferences_preserved":true,"real_provider_authentication":false}).to_string()).map_err(|_|"Cannot save E2E report")?;
        app.exit(0);
        return Ok(());
    }
    // A failed HTTPS check must not close or reset messaging views.
    if updater::update_check(app.clone(), caller.clone())
        .await
        .is_ok()
    {
        return Err("Missing Stable fixture feed unexpectedly succeeded".into());
    }
    updater::update_preferences(
        app.clone(),
        caller.clone(),
        updater::Preferences {
            channel: updater::Channel::Beta,
            automatic: false,
        },
        true,
    )
    .await?;
    updater::update_check(app.clone(), caller.clone()).await?;
    if updater::update_snapshot(app.clone(), caller.clone())?.status != "available" {
        return Err("New signed test version was not detected".into());
    }
    // Actual cancellable network future; no installer is invoked here.
    let download = updater::update_download(app.clone(), caller.clone());
    let cancel = async {
        for _ in 0..200 {
            if updater::update_snapshot(app.clone(), caller.clone())?.status == "downloading" {
                updater::update_cancel(app.clone(), caller.clone())?;
                return Ok::<(), String>(());
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        Err("Download cancellation did not start".into())
    };
    let (_, cancelled) = futures_util::future::join(download, cancel).await;
    cancelled?;
    updater::update_download(app.clone(), caller.clone()).await?;
    // The injected first shutdown failure must abort installation without forcing browsers.
    if updater::update_install(app.clone(), caller.clone(), true)
        .await
        .is_ok()
    {
        return Err("Injected shutdown failure did not abort".into());
    }
    if crate::health::closing() {
        return Err("Failed install left messaging operations blocked".into());
    }
    for a in &list {
        if app.get_webview(&accounts::label(&a.id)).is_none() {
            return Err("Injected failure destroyed a view".into());
        }
    }
    updater::update_download(app.clone(), caller.clone()).await?;
    std::fs::write(directory.join("baseline-ready.json"),json!({"version":version,"download_cancelled":true,"shutdown_failure_aborted":true,"profiles":3}).to_string()).map_err(|_|"Cannot save baseline test report")?;
    updater::update_install(app.clone(), caller, true).await?;
    Err("Windows updater unexpectedly returned instead of installing/restarting".into())
}
