use crate::health;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::atomic::Ordering;
use tauri::{
    Emitter, Manager, Webview, WebviewUrl,
    webview::{NewWindowResponse, PermissionResponse, WebviewBuilder},
};

// Fixed opaque IDs are intentional: the compatibility probe must survive restart.
const IDS: [&str; 3] = [
    "c4b9dc51-80ce-4e57-a738-4c182a216b01",
    "c4b9dc51-80ce-4e57-a738-4c182a216b02",
    "c4b9dc51-80ce-4e57-a738-4c182a216b03",
];
#[derive(Default)]
pub struct ProbeState(tauri::async_runtime::Mutex<ProbeRuntime>);
#[derive(Default)]
struct ProbeRuntime {
    fixture: Option<bool>,
    #[cfg(windows)]
    browsers: Vec<crate::windows_profiles::BrowserLifetime>,
}
#[derive(Serialize)]
pub struct Error {
    code: &'static str,
    message: &'static str,
}
type Result<T> = std::result::Result<T, Error>;
fn err(code: &'static str, message: &'static str) -> Error {
    Error { code, message }
}
fn label(i: usize) -> String {
    format!("probe-{}", IDS[i])
}
pub(crate) fn trusted(caller: &Webview) -> Result<()> {
    let url = caller
        .url()
        .map_err(|_| err("FORBIDDEN", "Cannot verify caller."))?;
    let local = (url.scheme() == "tauri" && url.host_str() == Some("localhost"))
        || (matches!(url.scheme(), "http" | "https") && url.host_str() == Some("tauri.localhost"))
        || (cfg!(debug_assertions)
            && url.scheme() == "http"
            && url.host_str() == Some("localhost")
            && url.port() == Some(1420));
    if caller.label() != "main" || !local {
        return Err(err(
            "FORBIDDEN",
            "Only the trusted shell can control profiles.",
        ));
    }
    Ok(())
}
#[derive(Clone, Copy, Deserialize)]
pub struct Bounds {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}
impl Bounds {
    pub(crate) fn validate(self) -> Result<()> {
        if [self.x, self.y, self.width, self.height]
            .iter()
            .any(|v| !v.is_finite())
            || self.x < 0.0
            || self.y < 0.0
            || self.width < 30.0
            || self.height < 30.0
            || self.x + self.width > 32768.0
            || self.y + self.height > 32768.0
        {
            return Err(err("INVALID_BOUNDS", "Invalid content geometry."));
        }
        Ok(())
    }
    pub(crate) fn rect(self) -> tauri::Rect {
        tauri::Rect {
            position: tauri::LogicalPosition::new(self.x, self.y).into(),
            size: tauri::LogicalSize::new(self.width, self.height).into(),
        }
    }
}
fn validate(bounds: &[Bounds]) -> Result<()> {
    if bounds.len() != 3 {
        return Err(err("INVALID_BOUNDS", "Exactly three panels required."));
    }
    for b in bounds {
        b.validate()?;
    }
    Ok(())
}
fn allowed(url: &tauri::Url, fixture: bool) -> bool {
    if fixture {
        let local = (url.scheme() == "tauri" && url.host_str() == Some("localhost"))
            || (matches!(url.scheme(), "http" | "https")
                && url.host_str() == Some("tauri.localhost"))
            || (cfg!(debug_assertions)
                && url.scheme() == "http"
                && url.host_str() == Some("localhost")
                && url.port() == Some(1420));
        return local && url.path() == "/isolation.html";
    }
    url.scheme() == "https"
        && url.host_str() == Some("web.whatsapp.com")
        && url.port_or_known_default() == Some(443)
        && url.username().is_empty()
        && url.password().is_none()
}
fn close_all(app: &tauri::AppHandle) -> Result<()> {
    for (i, id) in IDS.iter().enumerate() {
        if let Some(view) = app.get_webview(&label(i)) {
            health::log(
                app,
                "webview_close_requested",
                json!({"profile_id":id,"cleanup":false}),
            );
            #[cfg(windows)]
            crate::windows_profiles::close_controller(&view).map_err(|_| {
                err(
                    "CLOSE_FAILED",
                    "Native browser controller could not close. Retry; profile data is unchanged.",
                )
            })?;
            view.close().map_err(|_| {
                err(
                    "CLOSE_FAILED",
                    "Could not close panel. Restart before retrying.",
                )
            })?;
        }
    }
    Ok(())
}

pub fn request_shutdown(app: &tauri::AppHandle, exit_code: i32) {
    if health::SHUTTING_DOWN.swap(true, Ordering::SeqCst) {
        return;
    }
    health::log(app, "shutdown_requested", json!({"exit_code":exit_code}));
    let _ = app.emit_to("main", "probe-shutdown", json!({"state":"waiting"}));
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        if prepare_shutdown(&app).await.is_err() {
            shutdown_blocked(
                &app,
                "Browser shutdown could not finish. Retry closing the window.",
            );
            return;
        }
        health::log(
            &app,
            "shutdown_complete",
            json!({"cleanup":false,"browser_processes_exited":true}),
        );
        health::SHUTDOWN_FINISHED.store(true, Ordering::SeqCst);
        let mode = crate::accounts::RESTART_MODE.swap(0, Ordering::SeqCst);
        if mode != 0 {
            let launched = std::env::current_exe().ok().and_then(|exe| {
                let mut child = std::process::Command::new(exe);
                child.arg("--wait-for-instance");
                if mode == 1 {
                    child.arg("--diagnostics");
                }
                child.spawn().ok()
            });
            if launched.is_none() {
                health::SHUTDOWN_FINISHED.store(false, Ordering::SeqCst);
                shutdown_blocked(
                    &app,
                    "Could not restart. Close and reopen Appterisk normally.",
                );
                return;
            }
        }
        app.exit(exit_code);
    });
}

fn shutdown_blocked(app: &tauri::AppHandle, message: &'static str) {
    health::log(
        app,
        "shutdown_blocked",
        json!({"message":message,"forced_termination":false}),
    );
    health::SHUTTING_DOWN.store(false, Ordering::SeqCst);
    let _ = app.emit_to(
        "main",
        "probe-shutdown",
        json!({"state":"blocked","message":message}),
    );
}
#[tauri::command]
pub async fn open_probe(
    app: tauri::AppHandle,
    caller: Webview,
    state: tauri::State<'_, ProbeState>,
    bounds: Vec<Bounds>,
    fixture: bool,
) -> Result<()> {
    trusted(&caller)?;
    if crate::desktop::locked(&app) {
        return Err(err("APP_LOCKED", "Unlock the application first."));
    }
    if !crate::accounts::diagnostic_mode() {
        return Err(err(
            "DIAGNOSTIC_MODE",
            "Reopen with --diagnostics to use the compatibility probe.",
        ));
    }
    validate(&bounds)?;
    let mut runtime = state.0.lock().await;
    if health::closing() {
        return Err(err("CLOSING", "Profiles are closing safely."));
    }
    if health::overrides_present() {
        return Err(err(
            "ENVIRONMENT_OVERRIDE",
            "WebView2 environment overrides are set. Remove overrides from the launching shell before opening isolated profiles; no existing profile was changed.",
        ));
    }
    if !cfg!(target_os = "windows") {
        return Err(err(
            "UNSUPPORTED_PLATFORM",
            "Windows probe only. Other platform adapters require validation.",
        ));
    }
    let window = app
        .get_window("main")
        .ok_or_else(|| err("WINDOW_MISSING", "Main window unavailable."))?;
    if runtime.fixture == Some(fixture) && (0..3).all(|i| app.get_webview(&label(i)).is_some()) {
        for (i, b) in bounds.iter().enumerate() {
            if let Some(view) = app.get_webview(&label(i)) {
                view.set_bounds(b.rect())
                    .and_then(|()| view.show())
                    .map_err(|_| err("LAYOUT_FAILED", "Could not restore existing panel."))?;
            }
        }
        health::log(&app, "views_reused", json!({"fixture":fixture}));
        return Ok(());
    }
    close_all(&app)?;
    #[cfg(windows)]
    {
        let start = std::time::Instant::now();
        while !runtime
            .browsers
            .iter()
            .all(|browser| browser.exited.load(Ordering::SeqCst))
        {
            if start.elapsed() > std::time::Duration::from_secs(15) {
                return Err(err(
                    "PROFILE_BUSY",
                    "Previous browser process is still closing. Retry; no profile data has been removed.",
                ));
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        runtime.browsers.clear();
    }
    runtime.fixture = None;
    let root = crate::profile_paths::app_data_root(&app)
        .map_err(|_| err("PROFILE_STORAGE", "Application data unavailable."))?
        .join("compatibility-profiles");
    std::fs::create_dir_all(&root)
        .map_err(|_| err("PROFILE_STORAGE", "Cannot create profile root."))?;
    let root = root
        .canonicalize()
        .map_err(|_| err("PROFILE_STORAGE", "Cannot resolve profile root."))?;
    for (i, b) in bounds.into_iter().enumerate() {
        let path = root.join(IDS[i]);
        let existed = path.is_dir();
        std::fs::create_dir_all(&path)
            .map_err(|_| err("PROFILE_STORAGE", "Cannot create profile."))?;
        let path = path
            .canonicalize()
            .map_err(|_| err("PROFILE_STORAGE", "Cannot resolve profile."))?;
        if !path.starts_with(&root) || path == root {
            return Err(err("PROFILE_STORAGE", "Profile escaped managed root."));
        }
        let browser_path=crate::profile_paths::browser_directory(&path).map_err(|_|err("PROFILE_PATH","The existing profile cannot be represented as a normal absolute browser path. No data was moved or deleted."))?;
        health::log(
            &app,
            "profile_open",
            json!({"profile_id":IDS[i],"absolute_directory":browser_path,"canonical_directory":path,"existed":existed,"created":!existed,"incognito":false,"fixture":fixture,"cleanup":false}),
        );
        let url = if fixture {
            WebviewUrl::App("isolation.html".into())
        } else {
            WebviewUrl::External(
                "https://web.whatsapp.com/"
                    .parse()
                    .map_err(|_| err("INVALID_URL", "Invalid provider configuration."))?,
            )
        };
        // Async command avoids documented Windows synchronous WebView creation deadlock.
        let page_app = app.clone();
        let permission_app = app.clone();
        let builder = WebviewBuilder::new(label(i), url)
            .data_directory(browser_path)
            .incognito(false)
            .devtools(false)
            .disable_drag_drop_handler()
            .initialization_script(include_str!("../diagnostics/error-counts.js"))
            .on_page_load(move |view, payload| {
                if !fixture
                    && allowed(payload.url(), false)
                    && matches!(payload.event(), tauri::webview::PageLoadEvent::Finished)
                {
                    health::log(
                        &page_app,
                        "provider_page_loaded",
                        json!({"profile_id":IDS[i],"origin":"https://web.whatsapp.com"}),
                    );
                    capture_one(&page_app, view, IDS[i]);
                }
            })
            .on_navigation(move |url| allowed(url, fixture))
            .on_new_window(|_, _| NewWindowResponse::Deny)
            .on_download(|_, _| false)
            .on_permission_request(move |_, kind| {
                health::log(
                    &permission_app,
                    "permission_request",
                    json!({"profile_id":IDS[i],"kind":format!("{kind:?}"),"decision":"deny"}),
                );
                PermissionResponse::Deny
            });
        let created = window
            .add_child(
                builder,
                tauri::LogicalPosition::new(b.x, b.y),
                tauri::LogicalSize::new(b.width, b.height),
            )
            .map_err(|_| {
                err(
                    "WEBVIEW_FAILED",
                    "Could not create isolated panels. Check WebView2 and restart.",
                )
            })?;
        #[cfg(windows)]
        match crate::windows_profiles::attach(&app, &created, IDS[i], &path) {
            Ok(lifetime) => runtime.browsers.push(lifetime),
            Err(_) => {
                let _ = created.close();
                return Err(err(
                    "PROFILE_VERIFICATION",
                    "WebView2 did not confirm the expected persistent profile. Existing data is unchanged; see logs/persistence.jsonl.",
                ));
            }
        }
    }
    runtime.fixture = Some(fixture);
    Ok(())
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct StorageMetadata {
    local_storage_entries: Option<u32>,
    indexed_db_databases: Option<u32>,
    service_worker_registrations: Option<u32>,
    active_service_workers: Option<u32>,
    cache_storage_caches: Option<u32>,
    persistent_storage: Option<bool>,
    script_errors: Option<u32>,
    unhandled_rejections: Option<u32>,
    quota_usage_bytes: Option<u64>,
    quota_limit_bytes: Option<u64>,
}

#[tauri::command]
pub async fn capture_profile_metadata(app: tauri::AppHandle, caller: Webview) -> Result<()> {
    trusted(&caller)?;
    for (i, id) in IDS.iter().enumerate() {
        if let Some(view) = app.get_webview(&label(i)) {
            if !view.url().map(|url| allowed(&url, false)).unwrap_or(false) {
                continue;
            }
            capture_one(&app, view, id);
        }
    }
    Ok(())
}

#[tauri::command]
pub fn resume_probe_on_startup(caller: Webview) -> Result<bool> {
    trusted(&caller)?;
    Ok(std::env::args().any(|arg| arg == "--resume-whatsapp"))
}

fn capture_one(app: &tauri::AppHandle, view: Webview, id: &'static str) {
    #[cfg(windows)]
    crate::windows_profiles::cookie_count(app, &view, id);
    let app = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let script = include_str!("../diagnostics/origin-metadata.js");
        if view.eval(script).is_err() {
            return;
        }
        for _ in 0..50 {
            let (sender, receiver) = std::sync::mpsc::channel();
            if view
                .eval_with_callback("window.__appteriskStorageMetadata ?? null", move |raw| {
                    let _ = sender.send(raw);
                })
                .is_err()
            {
                break;
            }
            if let Ok(raw) = receiver.recv_timeout(std::time::Duration::from_secs(2))
                && raw.len() < 16384
                && let Ok(metadata) = serde_json::from_str::<StorageMetadata>(&raw)
            {
                health::log(
                    &app,
                    "origin_storage_metadata",
                    json!({"profile_id":id,"origin":"https://web.whatsapp.com","metadata":metadata}),
                );
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        health::log(
            &app,
            "origin_metadata_unavailable",
            json!({"profile_id":id}),
        );
    });
}
#[tauri::command]
pub async fn set_probe_bounds(
    app: tauri::AppHandle,
    caller: Webview,
    state: tauri::State<'_, ProbeState>,
    bounds: Vec<Bounds>,
) -> Result<()> {
    trusted(&caller)?;
    validate(&bounds)?;
    let _serial = state.0.lock().await;
    for (i, b) in bounds.into_iter().enumerate() {
        if let Some(view) = app.get_webview(&label(i)) {
            view.set_bounds(b.rect())
                .map_err(|_| err("LAYOUT_FAILED", "Could not resize panel."))?;
        }
    }
    Ok(())
}
#[tauri::command]
pub async fn set_probe_visibility(
    app: tauri::AppHandle,
    caller: Webview,
    state: tauri::State<'_, ProbeState>,
    visible: bool,
) -> Result<()> {
    trusted(&caller)?;
    if visible && crate::desktop::locked(&app) {
        return Err(err("APP_LOCKED", "Unlock the application first."));
    }
    let _serial = state.0.lock().await;
    for i in 0..3 {
        if let Some(view) = app.get_webview(&label(i)) {
            let result = if visible { view.show() } else { view.hide() };
            result.map_err(|_| err("VISIBILITY_FAILED", "Could not change panel visibility."))?;
        }
    }
    Ok(())
}
#[tauri::command]
pub async fn reload_probe(
    app: tauri::AppHandle,
    caller: Webview,
    state: tauri::State<'_, ProbeState>,
    index: usize,
) -> Result<()> {
    trusted(&caller)?;
    if index >= 3 {
        return Err(err("INVALID_PROFILE", "Unknown profile."));
    }
    let _serial = state.0.lock().await;
    app.get_webview(&label(index))
        .ok_or_else(|| err("VIEW_MISSING", "Open the probe first."))?
        .reload()
        .map_err(|_| err("RELOAD_FAILED", "Could not reload panel."))
}
pub async fn prepare_shutdown(app: &tauri::AppHandle) -> std::result::Result<(), String> {
    #[cfg(windows)]
    {
        crate::windows_permissions::deny_pending(app);
        crate::windows_notifications::clear(app);
        crate::windows_downloads::shutdown(app);
    }
    let state = app.state::<ProbeState>();
    let mut runtime = state.0.lock().await;
    let account_browsers = match crate::accounts::close_for_shutdown(app).await {
        Ok(browsers) => browsers,
        Err(_) => {
            return Err("Could not close account views.".into());
        }
    };
    if close_all(app).is_err() {
        return Err("Could not close native views.".into());
    }
    runtime.fixture = None;
    #[cfg(windows)]
    {
        let mut browsers = runtime.browsers.clone();
        browsers.extend(account_browsers);
        let completed = tauri::async_runtime::spawn_blocking(move || {
            let start = std::time::Instant::now();
            while !browsers
                .iter()
                .all(|browser| browser.exited.load(Ordering::SeqCst))
            {
                if start.elapsed() > std::time::Duration::from_secs(15) {
                    return false;
                }
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
            true
        })
        .await
        .unwrap_or(false);
        if !completed {
            return Err("WebView2 has not finished releasing a profile.".into());
        }
    }

    drop(runtime);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn metadata_rejects_private_values_and_arbitrary_fields() {
        assert!(
            serde_json::from_str::<StorageMetadata>(r#"{"localStorageEntries":"secret"}"#).is_err()
        );
        assert!(serde_json::from_str::<StorageMetadata>(r#"{"token":"secret"}"#).is_err());
        assert!(serde_json::from_str::<StorageMetadata>(r#"{"localStorageEntries":-1}"#).is_err());
        let data: StorageMetadata =
            serde_json::from_str(r#"{"localStorageEntries":4,"persistentStorage":false}"#).unwrap();
        assert_eq!(data.local_storage_entries, Some(4));
    }
    #[test]
    fn navigation_policy() {
        for url in [
            "http://web.whatsapp.com/",
            "https://web.whatsapp.com.evil.test/",
            "https://web.whatsapp.com:444/",
            "https://user@web.whatsapp.com/",
            "file:///etc/passwd",
        ] {
            assert!(!allowed(&url.parse().unwrap(), false));
        }
        assert!(allowed(
            &"https://web.whatsapp.com/".parse().unwrap(),
            false
        ));
    }
    #[test]
    fn invalid_geometry() {
        let b = Bounds {
            x: 72.0,
            y: 80.0,
            width: 400.0,
            height: 600.0,
        };
        assert!(b.validate().is_ok());
        assert!(
            Bounds {
                width: f64::NAN,
                ..b
            }
            .validate()
            .is_err()
        );
        assert!(Bounds { x: -1.0, ..b }.validate().is_err());
        assert!(validate(&[b]).is_err());
    }
    #[test]
    fn unique_profiles() {
        assert_eq!(
            IDS.iter().collect::<std::collections::HashSet<_>>().len(),
            3
        );
        assert!(IDS.iter().all(|id| !id.contains('/') && !id.contains('\\')));
        assert_ne!(label(0), label(1));
    }
}
