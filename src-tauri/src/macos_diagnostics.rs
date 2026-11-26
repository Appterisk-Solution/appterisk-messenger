//! Debug-only, disposable native storage test. No real account stores or logins.
use serde_json::{Value, json};
use std::{
    io::{Read, Write},
    sync::{Arc, Mutex},
    time::Duration,
};
use tauri::{
    Manager, WebviewUrl,
    webview::{PageLoadEvent, WebviewBuilder},
};

async fn compatibility_metadata(view: &tauri::Webview) -> Result<Value, String> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    let tx = Mutex::new(Some(tx));
    // This is only used with a new, unlinked diagnostic account. Return flags,
    // never page text, QR codes, cookies or authentication values.
    view.eval_with_callback(r#"JSON.stringify({userAgent:navigator.userAgent,unsupported:/WhatsApp works with Safari|update Safari/i.test(document.body?.innerText??''),loginPrompt:/scan to log in|log in with phone number/i.test(document.body?.innerText??'')})"#, move |value| {
        if let Some(tx) = tx.lock().unwrap().take() { let _ = tx.send(value); }
    }).map_err(|e| e.to_string())?;
    let encoded = tokio::time::timeout(Duration::from_secs(5), rx)
        .await
        .map_err(|_| "Compatibility check timeout")?
        .map_err(|_| "Compatibility check failed")?;
    let decoded: String = serde_json::from_str(&encoded).map_err(|e| e.to_string())?;
    serde_json::from_str(&decoded).map_err(|e| e.to_string())
}

pub fn start(app: &tauri::AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    if !args.iter().any(|a| a == "--macos-isolation-diagnostic") {
        return Ok(());
    }
    let run = args
        .iter()
        .find_map(|s| uuid::Uuid::parse_str(s).ok())
        .ok_or("Missing run UUID")?;
    let phase: u8 = args
        .windows(2)
        .find(|pair| pair[0] == "--macos-isolation-diagnostic")
        .ok_or("Missing phase")?[1]
        .parse()?;
    let provider_smoke = args.iter().any(|arg| arg == "--macos-provider-smoke");
    if phase > 3 {
        return Err("Invalid phase".into());
    }
    // Same loopback origin in every store and every process restart.
    let server = std::net::TcpListener::bind("127.0.0.1:18765")?;
    std::thread::spawn(move || {
        for mut stream in server.incoming().flatten() {
            let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
            let mut buffer = [0; 4096];
            let Ok(n) = stream.read(&mut buffer) else {
                continue;
            };
            let worker = String::from_utf8_lossy(&buffer[..n]).starts_with("GET /worker.js ");
            let (mime, body) = if worker {
                (
                    "application/javascript",
                    "self.addEventListener('install',()=>self.skipWaiting()); self.addEventListener('activate',e=>e.waitUntil(self.clients.claim()));",
                )
            } else {
                (
                    "text/html",
                    include_str!("../diagnostics/macos-isolation.html"),
                )
            };
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: {mime}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            let _ = stream.write_all(response.as_bytes());
        }
    });
    let report = crate::profile_paths::app_data_root(app)?
        .join("diagnostic-runs")
        .join(run.to_string());
    std::fs::create_dir_all(&report)?;
    let results = Arc::new(Mutex::new(std::collections::BTreeMap::<usize, Value>::new()));
    let handle = app.clone();
    tauri::async_runtime::spawn(async move {
        let result: Result<(), String> = async {
            // Exercise the production account initializer against an empty, unlinked profile.
            // This database is under diagnostic-runs/<run>/manager, never the owner's database.
            {
                let state = handle.state::<crate::accounts::AccountState>();
                let mut runtime = state.0.lock().await;
                let account = match runtime.db.accounts()?.into_iter().next() {
                    Some(account) => account,
                    None => runtime.db.add("Disposable Mac smoke test", None)?,
                };
                let bounds = crate::probe::Bounds {
                    x: 180.0,
                    y: 180.0,
                    width: 600.0,
                    height: 400.0,
                };
                crate::accounts::switch(&handle, &mut runtime, &account.id, bounds).await?;
                let count = handle.webviews().len();
                crate::accounts::switch(&handle, &mut runtime, &account.id, bounds).await?;
                if handle.webviews().len() != count {
                    return Err("Account switching recreated its view".into());
                }
                crate::accounts::block(&handle, &mut runtime, bounds, true)?;
                crate::accounts::block(&handle, &mut runtime, bounds, false)?;
                let mut measurements = Vec::new();
                for (width, height) in [(1280.0, 860.0), (1100.0, 760.0), (1600.0, 1000.0)] {
                    handle
                        .get_window("main")
                        .unwrap()
                        .set_size(tauri::LogicalSize::new(width, height))
                        .map_err(|e| e.to_string())?;
                    tokio::time::sleep(Duration::from_millis(150)).await;
                    let shell = handle.get_webview("main").unwrap();
                    let shell_geometry = crate::macos_profiles::geometry(&shell).await;
                    let inset = shell_geometry["insetTop"]
                        .as_f64()
                        .ok_or("Missing shell inset")?;
                    let bounds = crate::probe::Bounds {
                        x: 88.0,
                        y: 76.0,
                        width: width - 88.0,
                        height: height - inset - 76.0 - 28.0,
                    };
                    crate::accounts::block(&handle, &mut runtime, bounds, false)?;
                    // Drain the shell conversion, then inspect the child on the UI thread.
                    let _ = crate::macos_profiles::geometry(&shell).await;
                    let account_geometry = crate::macos_profiles::geometry(
                        &handle
                            .get_webview(&crate::accounts::label(&account.id))
                            .unwrap(),
                    )
                    .await;
                    for (key, expected) in [
                        ("x", bounds.x),
                        ("y", bounds.y + inset),
                        ("width", bounds.width),
                        ("height", bounds.height),
                    ] {
                        if (account_geometry[key]
                            .as_f64()
                            .ok_or("Missing child bounds")?
                            - expected)
                            .abs()
                            > 0.5
                        {
                            return Err(format!("Native account alignment mismatch: {key}"));
                        }
                    }
                    measurements.push(json!({"shell":shell_geometry,"account":account_geometry}));
                }
                let _ = std::fs::write(
                    report.join("geometry.json"),
                    json!({"pass":true,"sizes":measurements}).to_string(),
                );
                if phase == 0 && provider_smoke {
                    let view = handle
                        .get_webview(&crate::accounts::label(&account.id))
                        .unwrap();
                    let start = std::time::Instant::now();
                    loop {
                        let mut metadata = match compatibility_metadata(&view).await {
                            Ok(metadata) => metadata,
                            Err(_) if start.elapsed() < Duration::from_secs(45) => {
                                tokio::time::sleep(Duration::from_secs(1)).await;
                                continue;
                            }
                            Err(error) => return Err(error),
                        };
                        if metadata["loginPrompt"] == true {
                            tokio::time::sleep(Duration::from_secs(3)).await;
                            metadata = compatibility_metadata(&view).await?;
                        }
                        if metadata["loginPrompt"] == true
                            || metadata["unsupported"] == true
                            || start.elapsed() > Duration::from_secs(45)
                        {
                            std::fs::write(
                                report.join("provider-compatibility.json"),
                                metadata.to_string(),
                            )
                            .map_err(|e| e.to_string())?;
                            if metadata["unsupported"] == true || metadata["loginPrompt"] != true {
                                return Err(
                                    "WhatsApp did not reach its unlinked login screen".into()
                                );
                            }
                            break;
                        }
                        tokio::time::sleep(Duration::from_secs(1)).await;
                    }
                }
            }
            for i in 0..3 {
                let mut bytes = run.into_bytes();
                bytes[15] ^= i as u8;
                let id = uuid::Uuid::from_bytes(bytes).to_string();
                let load_id = id.clone();
                let callback_app = handle.clone();
                let callback_report = report.clone();
                let results = results.clone();
                let builder = WebviewBuilder::new(
                    format!("macos-fixture-{i}"),
                    WebviewUrl::External(
                        format!("http://127.0.0.1:18765/?index={i}&phase={phase}")
                            .parse()
                            .unwrap(),
                    ),
                )
                .on_navigation(|url| {
                    url.scheme() == "http"
                        && url.host_str() == Some("127.0.0.1")
                        && url.port() == Some(18765)
                })
                .on_page_load(move |view, payload| {
                    if matches!(payload.event(), PageLoadEvent::Finished) {
                        let view = view.clone();
                        let id = load_id.clone();
                        tauri::async_runtime::spawn(async move {
                            if crate::macos_profiles::verify(&view, &id).await.is_ok() {
                                let _ = view.eval("window.runFixture()");
                            } else {
                                let _ = view
                                    .eval("document.title='FIXTURE:{\"index\":0,\"pass\":false}'");
                            }
                        });
                    }
                })
                .on_document_title_changed(move |_, title| {
                    let Some(payload) = title.strip_prefix("FIXTURE:") else {
                        return;
                    };
                    let Ok(value) = serde_json::from_str::<Value>(payload) else {
                        return;
                    };
                    let mut results = results.lock().unwrap();
                    if results.insert(i, value).is_some() || results.len() != 3 {
                        return;
                    }
                    let pass = results.values().all(|v| v["pass"] == true);
                    let evidence =
                        json!({"phase":phase,"pass":pass,"accountManager":true,"stores":*results});
                    let _ = std::fs::write(
                        callback_report.join(format!("phase-{phase}.json")),
                        serde_json::to_vec_pretty(&evidence).unwrap(),
                    );
                    let app = callback_app.clone();
                    tauri::async_runtime::spawn(async move {
                        tokio::time::sleep(Duration::from_secs(2)).await;
                        for i in 0..3 {
                            if let Some(v) = app.get_webview(&format!("macos-fixture-{i}")) {
                                let _ = v.close();
                            }
                        }
                        crate::probe::request_shutdown(&app, if pass { 0 } else { 1 });
                    });
                });
                let builder = crate::macos_profiles::configure(builder, &id)?;
                handle
                    .get_window("main")
                    .ok_or("Missing window")?
                    .add_child(
                        builder,
                        tauri::LogicalPosition::new(180.0 + i as f64 * 300.0, 180.0),
                        tauri::LogicalSize::new(280.0, 400.0),
                    )
                    .map_err(|e| e.to_string())?;
            }
            Ok(())
        }
        .await;
        if let Err(error) = result {
            let _ = std::fs::write(
                report.join(format!("phase-{phase}.json")),
                json!({"pass":false,"error":error}).to_string(),
            );
            crate::probe::request_shutdown(&handle, 1);
        }
    });
    Ok(())
}
