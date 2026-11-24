//! Debug-only integration exercise. Separate disposable profiles, no real credentials.
use crate::{accounts, database, diagnostics::evaluate, probe::Bounds};
use serde_json::{Value, json};
use std::{
    sync::mpsc,
    time::{Duration, Instant},
};
use tauri::{Manager, Webview};
use windows::core::BOOL;
type Result<T> = std::result::Result<T, String>;

fn visible(view: &Webview) -> Result<bool> {
    let (tx, rx) = mpsc::channel();
    view.with_webview(move |native| {
        let mut flag = BOOL(0);
        let result = unsafe { native.controller().IsVisible(&mut flag) }
            .map(|()| flag.as_bool())
            .map_err(|_| "Native visibility getter failed".into());
        let _ = tx.send(result);
    })
    .map_err(|_| "Cannot schedule visibility check")?;
    rx.recv_timeout(Duration::from_secs(5))
        .map_err(|_| "Visibility check timed out")?
}
fn ready(view: &Webview) -> Result<()> {
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(45) {
        if evaluate(
            view,
            "location.origin === 'https://web.whatsapp.com' && document.readyState === 'complete'",
        )
        .unwrap_or(Value::Null)
            == true
        {
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(150));
    }
    Err("Official remote origin did not finish loading; network test is inconclusive.".into())
}
fn remote_denial(view: &Webview) -> Result<Value> {
    evaluate(
        view,
        r#"window.__appteriskIpcDiagnostic=null;void(async()=>{const out={origin:location.origin};for(const command of ['account_snapshot','account_add','restart_mode','reload_probe']){if(!window.__TAURI_INTERNALS__?.invoke){out[command]='no-bridge';continue;}try{await window.__TAURI_INTERNALS__.invoke(command,{});out[command]='ACCEPTED';}catch(e){out[command]=/forbidden|not allowed|denied/i.test(String(e))?'denied':'unexpected-error';}}window.__appteriskIpcDiagnostic=out;})();true"#,
    )?;
    for _ in 0..100 {
        let result = evaluate(view, "window.__appteriskIpcDiagnostic ?? null")?;
        if !result.is_null() {
            if result["origin"] != "https://web.whatsapp.com"
                || [
                    "account_snapshot",
                    "account_add",
                    "restart_mode",
                    "reload_probe",
                ]
                .iter()
                .any(|c| !matches!(result[*c].as_str(), Some("denied" | "no-bridge")))
            {
                return Err(format!("Remote IPC denial failed: {result}"));
            }
            return Ok(result);
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    Err("Remote IPC diagnostic timed out".into())
}
pub fn start(app: &tauri::AppHandle) -> std::result::Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    let Some(index) = args.iter().position(|a| a == "--manager-diagnostic") else {
        return Ok(());
    };
    let stage = args.get(index + 1).ok_or("Missing manager stage")?.clone();
    let run = args.get(index + 2).ok_or("Missing run ID")?.clone();
    if !matches!(stage.as_str(), "seed" | "verify" | "remove")
        || uuid::Uuid::parse_str(&run).is_err()
    {
        return Err("Invalid manager diagnostic arguments".into());
    }
    let app = app.clone();
    std::thread::spawn(move || {
        let result = tauri::async_runtime::block_on(execute(&app, &stage));
        let report = match result {
            Ok(value) => value,
            Err(error) => json!({"passed":false,"error":error}),
        };
        if let Ok(root) = app.path().app_local_data_dir() {
            let _ = std::fs::write(
                root.join("diagnostic-runs")
                    .join(&run)
                    .join(format!("manager-{stage}.json")),
                report.to_string(),
            );
        }
        println!("{report}");
        app.exit(if report["passed"] == true { 0 } else { 1 });
    });
    Ok(())
}
async fn execute(app: &tauri::AppHandle, stage: &str) -> Result<Value> {
    let state = app.state::<accounts::AccountState>();
    let mut r = state.0.lock().await;
    let manifest = r.root.join("expected-ids.json");
    if stage == "seed" {
        let legacy = r
            .root
            .join("compatibility-profiles")
            .join(database::LEGACY_IDS[0]);
        std::fs::create_dir_all(&legacy).map_err(|_| "Cannot prepare synthetic adoption")?;
        std::fs::write(
            legacy.join("synthetic-adoption-marker.txt"),
            b"public fixture only",
        )
        .map_err(|_| "Cannot write public marker")?;
        r.db.add("Adopted fixture", Some(database::LEGACY_IDS[0]))?;
        r.db.add("Second fixture", None)?;
        r.db.add("Third fixture", None)?;
        let ids: Vec<_> =
            r.db.accounts()?
                .iter()
                .rev()
                .map(|a| a.id.clone())
                .collect();
        r.db.reorder(&ids)?;
        std::fs::write(
            &manifest,
            serde_json::to_vec(&ids).map_err(|_| "Cannot encode fixture IDs")?,
        )
        .map_err(|_| "Cannot record fixture IDs")?;
    }
    let ids: Vec<String> =
        serde_json::from_slice(&std::fs::read(&manifest).map_err(|_| "Missing fixture IDs")?)
            .map_err(|_| "Invalid fixture IDs")?;
    let accounts = r.db.accounts()?;
    if accounts.iter().map(|a| a.id.clone()).collect::<Vec<_>>() != ids {
        return Err("Account identifiers/order were not preserved across restart".into());
    }
    if stage != "seed" && r.db.settings()?.last_active_account.as_ref() != ids.last() {
        return Err("Last selected account was not restored in SQLite".into());
    }
    let bounds = Bounds {
        x: 90.0,
        y: 90.0,
        width: 850.0,
        height: 600.0,
    };
    let resized = Bounds {
        width: 700.0,
        height: 480.0,
        ..bounds
    };
    let mut remote_results = Vec::new();
    for account in &accounts {
        accounts::switch(app, &mut r, &account.id, bounds).await?;
        let view = app
            .get_webview(&accounts::label(&account.id))
            .ok_or("Missing initialized view")?;
        ready(&view)?;
        evaluate(
            &view,
            "window.__appteriskManagerFixture = 'synthetic-no-reload-marker';true",
        )?;
        remote_results.push(remote_denial(&view)?);
    }
    for account in &accounts {
        accounts::switch(app, &mut r, &account.id, resized).await?;
        for other in &accounts {
            let view = app
                .get_webview(&accounts::label(&other.id))
                .ok_or("Missing reused view")?;
            if visible(&view)? != (other.id == account.id) {
                return Err("Switch showed more than the selected native view".into());
            }
            if evaluate(
                &view,
                "window.__appteriskManagerFixture === 'synthetic-no-reload-marker'",
            )? != true
            {
                return Err("Switch recreated/reloaded a document".into());
            }
        }
    }
    accounts::block(app, &mut r, resized, true)?;
    for account in &accounts {
        if visible(
            &app.get_webview(&accounts::label(&account.id))
                .ok_or("Missing view")?,
        )? {
            return Err("Dialog failed to hide a native view".into());
        }
    }
    accounts::block(app, &mut r, resized, false)?;
    let active = app
        .get_webview(&accounts::label(ids.last().ok_or("Missing last ID")?))
        .ok_or("Missing active view")?;
    let size = active.size().map_err(|_| "Cannot inspect resized view")?;
    let scale = app
        .get_window("main")
        .ok_or("Missing main")?
        .scale_factor()
        .map_err(|_| "No DPI scale")?;
    if (f64::from(size.width) / scale - resized.width).abs() > 1.0
        || (f64::from(size.height) / scale - resized.height).abs() > 1.0
    {
        return Err("Native resize did not match logical content bounds".into());
    }
    if stage == "remove" {
        let removed = &accounts[0];
        let path = database::profile_path(&r.root, removed)?;
        accounts::remove_account(app, &mut r, &removed.id, true)?;
        if !path.is_dir() || r.db.accounts()?.len() != 2 {
            return Err("Removal changed profile storage or sibling metadata".into());
        }
        for a in accounts.iter().skip(1) {
            if app.get_webview(&accounts::label(&a.id)).is_none() {
                return Err("Removal destroyed another account view".into());
            }
        }
    }
    let legacy = r
        .root
        .join("compatibility-profiles")
        .join(database::LEGACY_IDS[0])
        .join("synthetic-adoption-marker.txt");
    if std::fs::read(&legacy).map_err(|_| "Adoption moved original profile")?
        != b"public fixture only"
    {
        return Err("Adoption changed original fixture".into());
    }
    drop(r);
    // Exercise the same centralized close path as X/Exit. RunEvent waits for acknowledgments.
    Ok(
        json!({"passed":true,"stage":stage,"account_ids":ids,"remote_origin_ipc":remote_results,"single_visible_view":true,"switch_without_reload":true,"dialog_hidden":true,"native_resize":true,"adoption_in_place":true,"scope":"Disposable profiles only. No real login, notification or media claims."}),
    )
}
