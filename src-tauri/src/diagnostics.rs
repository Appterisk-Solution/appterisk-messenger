//! Credential-free, debug-only native engine test. No production/probe profiles are touched.
use serde_json::{Value, json};
use std::{sync::mpsc, thread, time::Duration};
use tauri::{Manager, Webview, WebviewUrl, webview::WebviewBuilder};

pub fn start(app: &tauri::AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let Some(position) = args.iter().position(|arg| arg == "--isolation-diagnostic") else {
        return Ok(());
    };
    let stage = args
        .get(position + 1)
        .ok_or("Missing diagnostic stage")?
        .clone();
    let run = args
        .get(position + 2)
        .ok_or("Missing diagnostic run ID")?
        .clone();
    if !matches!(stage.as_str(), "seed" | "verify")
        || run.len() != 36
        || !run.chars().all(|c| c.is_ascii_hexdigit() || c == '-')
    {
        return Err("Invalid diagnostic arguments".into());
    }
    let app = app.clone();
    thread::spawn(move || {
        let result = execute(&app, &stage, &run);
        let report = match &result {
            Ok(report) => report.clone(),
            Err(error) => json!({"stage": stage, "run": run, "passed": false, "error": error}),
        };
        if let Ok(root) = app.path().app_local_data_dir() {
            let path = root.join("diagnostic-runs").join(&run);
            if let Err(error) = std::fs::create_dir_all(&path).and_then(|()| {
                std::fs::write(path.join(format!("{stage}.json")), report.to_string())
            }) {
                eprintln!("Could not save diagnostic report: {error}");
                app.exit(1);
                return;
            }
        }
        println!("{report}");
        app.exit(if result.is_ok() { 0 } else { 1 });
    });
    Ok(())
}

pub(crate) fn evaluate(view: &Webview, script: &str) -> Result<Value, String> {
    let (sender, receiver) = mpsc::channel();
    view.eval_with_callback(script, move |raw| {
        let _ = sender.send(raw);
    })
    .map_err(|error| error.to_string())?;
    let raw = receiver
        .recv_timeout(Duration::from_secs(5))
        .map_err(|_| "JavaScript callback timed out")?;
    serde_json::from_str(&raw).map_err(|_| "Invalid diagnostic response".into())
}

fn wait_result(view: &Webview) -> Result<Value, String> {
    for _ in 0..100 {
        let result = evaluate(view, "window.__isolationResult ?? null")?;
        if !result.is_null() {
            return Ok(result);
        }
        thread::sleep(Duration::from_millis(100));
    }
    Err("Local fixture timed out".into())
}

fn execute(app: &tauri::AppHandle, stage: &str, run: &str) -> Result<Value, String> {
    let window = app.get_window("main").ok_or("Missing diagnostic window")?;
    window
        .set_title("Appterisk — Local storage diagnostic")
        .map_err(|e| e.to_string())?;
    let root = app
        .path()
        .app_local_data_dir()
        .map_err(|e| e.to_string())?
        .join("diagnostic-runs")
        .join(run);
    let mut views = Vec::new();
    let mut browsers = Vec::new();
    for index in 0..3 {
        let config = json!({"stage": stage, "marker": format!("{run}-{index}")});
        let path = root.join(format!("profile-{index}"));
        std::fs::create_dir_all(&path).map_err(|e| e.to_string())?;
        let browser_path = if std::env::args().any(|arg| arg == "--verbatim-path") {
            path.canonicalize()
                .map_err(|_| "Cannot canonicalize test profile")?
        } else {
            crate::profile_paths::browser_directory(
                &path
                    .canonicalize()
                    .map_err(|_| "Cannot canonicalize test profile")?,
            )?
        };
        let builder = WebviewBuilder::new(
            format!("isolation-{index}"),
            WebviewUrl::App("isolation.html".into()),
        )
        .data_directory(browser_path)
        .incognito(false)
        .devtools(false)
        .initialization_script(format!("window.__isolationConfig = {config};"))
        .initialization_script(include_str!("../diagnostics/storage.js"))
        .on_navigation(|url| {
            let local = (url.scheme() == "tauri" && url.host_str() == Some("localhost"))
                || (matches!(url.scheme(), "http" | "https")
                    && url.host_str() == Some("tauri.localhost"))
                || (url.scheme() == "http"
                    && url.host_str() == Some("localhost")
                    && url.port() == Some(1420));
            local && url.path() == "/isolation.html"
        });
        let view = window
            .add_child(
                builder,
                tauri::LogicalPosition::new(index as f64 * 400.0, 0.0),
                tauri::LogicalSize::new(390.0, 500.0),
            )
            .map_err(|error| error.to_string())?;
        let result = wait_result(&view)?;
        if result.get("error").is_some() {
            return Err(result.to_string());
        }
        browsers.push(crate::windows_profiles::attach(
            app,
            &view,
            &format!("isolation-{index}"),
            &root
                .join(format!("profile-{index}"))
                .canonicalize()
                .map_err(|_| "Cannot resolve diagnostic profile")?,
        )?);
        views.push(view);
    }
    let mut profiles = Vec::new();
    // Re-read after all three writes: a shared profile would now contain the last marker.
    for (index, view) in views.iter().enumerate() {
        evaluate(
            view,
            "window.__isolationResult = null; void window.__readIsolation()",
        )?;
        let result = wait_result(view)?;
        let marker = format!("{run}-{index}");
        if result["local"] != marker
            || result["cookie"] != marker
            || result["indexedDb"] != marker
            || result["ipcDenied"] != true
            || result["blobOk"] != true
            || result["cryptoOk"] != true
            || result["sessionCookiePresent"] != (stage == "seed")
        {
            return Err(format!("Profile {index} failed: {result}"));
        }
        profiles.push(json!({"profile": index, "storage": result}));
    }
    for view in views {
        crate::windows_profiles::close_controller(&view)?;
        view.close().map_err(|error| error.to_string())?;
    }
    let start = std::time::Instant::now();
    while !browsers
        .iter()
        .all(|browser| browser.exited.load(std::sync::atomic::Ordering::SeqCst))
    {
        if start.elapsed() > Duration::from_secs(15) {
            return Err("Browser process did not exit gracefully".into());
        }
        thread::sleep(Duration::from_millis(50));
    }
    Ok(
        json!({"stage": stage, "run": run, "passed": true, "profiles": profiles,
        "scope": "Local fixture only; real provider logins and notifications are untested"}),
    )
}
