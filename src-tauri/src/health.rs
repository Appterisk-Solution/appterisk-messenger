//! Metadata-only diagnostics. Never log provider URLs, storage keys/values or console text.
use serde_json::{Value, json};
use std::{
    fs::OpenOptions,
    io::Write,
    sync::{
        Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::{SystemTime, UNIX_EPOCH},
};
use tauri::Manager;

static LOG_LOCK: Mutex<()> = Mutex::new(());
pub static SHUTTING_DOWN: AtomicBool = AtomicBool::new(false);
pub static SHUTDOWN_FINISHED: AtomicBool = AtomicBool::new(false);

pub fn log(app: &tauri::AppHandle, event: &'static str, fields: Value) {
    let Ok(_guard) = LOG_LOCK.lock() else {
        return;
    };
    let Ok(root) = app.path().app_local_data_dir() else {
        return;
    };
    let root = root.join("logs");
    if std::fs::create_dir_all(&root).is_err() {
        return;
    }
    let Ok(mut file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open(root.join("persistence.jsonl"))
    else {
        return;
    };
    let record = json!({"unix_ms":SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis(),"pid":std::process::id(),"event":event,"fields":fields});
    let _ = writeln!(file, "{record}");
}

pub fn startup(app: &tauri::AppHandle) {
    let overrides: Vec<_> = [
        "WEBVIEW2_USER_DATA_FOLDER",
        "WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS",
        "WEBVIEW2_BROWSER_EXECUTABLE_FOLDER",
        "WEBVIEW2_RELEASE_CHANNEL_PREFERENCE",
    ]
    .iter()
    .map(|name| json!({"name":name,"present":std::env::var_os(name).is_some()}))
    .collect();
    log(
        app,
        "startup",
        json!({"executable":std::env::current_exe().ok(),"version":app.package_info().version.to_string(),"overrides":overrides,"storage_cleanup":"never performed"}),
    );
}

pub fn overrides_present() -> bool {
    [
        "WEBVIEW2_USER_DATA_FOLDER",
        "WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS",
        "WEBVIEW2_BROWSER_EXECUTABLE_FOLDER",
    ]
    .iter()
    .any(|name| std::env::var_os(name).is_some())
}

pub fn closing() -> bool {
    SHUTTING_DOWN.load(Ordering::SeqCst)
}
