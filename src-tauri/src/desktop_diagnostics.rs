//! Synthetic attachment fixture using real WebView2 download events, never WhatsApp credentials.
use crate::{accounts::AccountState, desktop::DesktopState, diagnostics::evaluate};
use serde_json::json;
use std::time::{Duration, Instant};
use tauri::{Manager, WebviewUrl, webview::WebviewBuilder};
pub fn start(app: &tauri::AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    let Some(index) = args.iter().position(|a| a == "--desktop-diagnostic") else {
        return Ok(());
    };
    let run = args
        .get(index + 1)
        .ok_or("Missing desktop fixture run")?
        .clone();
    uuid::Uuid::parse_str(&run)?;
    let app = app.clone();
    std::thread::spawn(move || {
        let result = tauri::async_runtime::block_on(execute(&app));
        let report = match result {
            Ok(v) => v,
            Err(error) => json!({"passed":false,"error":error}),
        };
        if let Ok(root) = app.path().app_local_data_dir() {
            let _ = std::fs::write(
                root.join("diagnostic-runs").join(run).join("desktop.json"),
                report.to_string(),
            );
        }
        println!("{report}");
        app.exit(if report["passed"] == true { 0 } else { 1 });
    });
    Ok(())
}
async fn execute(app: &tauri::AppHandle) -> Result<serde_json::Value, String> {
    let state = app.state::<AccountState>();
    let mut r = state.0.lock().await;
    let account = r.db.add("Synthetic download fixture", None)?;
    let directory = r.root.join("attachments");
    std::fs::create_dir_all(&directory).map_err(|_| "Cannot prepare attachment fixture")?;
    std::fs::write(
        directory.join("fixture.txt"),
        b"existing fixture; never overwrite",
    )
    .map_err(|_| "Cannot prepare collision fixture")?;
    {
        let s = app.state::<DesktopState>();
        let mut d = s.0.lock().map_err(|_| "Desktop unavailable")?;
        d.config.ask_before_saving = false;
        d.config.download_directory = Some(directory.to_string_lossy().into());
        d.accounts.insert(account.id.clone(), account.clone());
    }
    let profile = r.root.join("download-browser");
    std::fs::create_dir_all(&profile).map_err(|_| "Cannot prepare download browser")?;
    let canonical = profile
        .canonicalize()
        .map_err(|_| "Cannot resolve fixture browser")?;
    drop(r);
    let window = app.get_window("main").ok_or("No fixture window")?;
    let view = window
        .add_child(
            WebviewBuilder::new(
                crate::accounts::label(&account.id),
                WebviewUrl::App("isolation.html".into()),
            )
            .data_directory(crate::profile_paths::browser_directory(&canonical)?)
            .incognito(false)
            .devtools(false),
            tauri::LogicalPosition::new(100.0, 100.0),
            tauri::LogicalSize::new(800.0, 500.0),
        )
        .map_err(|_| "Cannot create download fixture")?;
    let browser = crate::windows_profiles::attach(app, &view, &account.id, &canonical)?;
    state
        .0
        .lock()
        .await
        .track_browser(&account.id, browser.clone());
    crate::windows_downloads::attach(app, &view, &account.id)?;
    let start = Instant::now();
    while evaluate(&view, "document.readyState === 'complete'").unwrap_or_default() != true {
        if start.elapsed() > Duration::from_secs(15) {
            return Err("Download fixture did not load".into());
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    evaluate(
        &view,
        "(()=>{const a=document.createElement('a');a.href=URL.createObjectURL(new Blob(['Appterisk public attachment fixture'],{type:'application/octet-stream'}));a.download='fixture.txt';document.body.append(a);a.click();a.remove();return true;})()",
    )?;
    let start = Instant::now();
    let saved = loop {
        let download = {
            let s = app.state::<DesktopState>();
            let d = s.0.lock().map_err(|_| "Desktop unavailable")?;
            d.downloads
                .values()
                .find(|v| v.account_id == account.id)
                .cloned()
        };
        if let Some(download) = download {
            if download.state == "completed" {
                break download.saved_path.ok_or("Missing final attachment path")?;
            }
            if matches!(
                download.state.as_str(),
                "failed" | "interrupted" | "cancelled"
            ) {
                return Err(format!("Native fixture download {}", download.state));
            }
        }
        if start.elapsed() > Duration::from_secs(25) {
            return Err("Native download completion timed out".into());
        }
        std::thread::sleep(Duration::from_millis(100));
    };
    let bytes = std::fs::read(&saved).map_err(|_| "Cannot read public fixture attachment")?;
    if bytes != b"Appterisk public attachment fixture"
        || std::fs::read(directory.join("fixture.txt")).map_err(|_| "Collision fixture missing")?
            != b"existing fixture; never overwrite"
    {
        return Err("Native attachment was empty, corrupt or overwrote an existing file".into());
    }
    if !std::path::Path::new(&saved).starts_with(
        directory
            .canonicalize()
            .map_err(|_| "Cannot resolve attachment directory")?,
    ) {
        return Err("Download escaped fixture directory".into());
    }
    crate::windows_profiles::close_controller(&view)?;
    view.close().map_err(|_| "Cannot close fixture browser")?;
    let start = Instant::now();
    while !browser.exited.load(std::sync::atomic::Ordering::SeqCst) {
        if start.elapsed() > Duration::from_secs(15) {
            return Err("Download browser did not exit normally".into());
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    Ok(
        json!({"passed":true,"native_download":true,"payload_integrity":true,"collision_preserved":true,"profile_id":account.id,"scope":"Public synthetic attachment only; real WhatsApp media and uploads require manual validation."}),
    )
}
