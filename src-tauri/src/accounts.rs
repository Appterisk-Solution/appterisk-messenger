//! Single owner for metadata and initialized native account views.
use crate::{
    database::{Account, Database, LEGACY_IDS, Settings, profile_path},
    health,
    probe::Bounds,
};
use serde::Serialize;
use serde_json::json;
use std::{
    collections::HashMap,
    fs::{File, OpenOptions},
    path::{Path, PathBuf},
    sync::atomic::Ordering,
};
use tauri::{
    Emitter, Manager, Webview, WebviewUrl,
    webview::{NewWindowResponse, WebviewBuilder},
};
type Result<T> = std::result::Result<T, String>;
pub static RESTART_MODE: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(0);
fn runtime_version() -> Option<String> {
    #[cfg(windows)]
    {
        crate::windows_profiles::runtime_version()
    }
    #[cfg(not(windows))]
    {
        None
    }
}

pub fn diagnostic_mode() -> bool {
    std::env::args().any(|a| matches!(a.as_str(), "--diagnostics" | "--resume-whatsapp"))
}
pub fn synthetic_mode() -> bool {
    cfg!(debug_assertions)
        && std::env::args().any(|a| {
            matches!(
                a.as_str(),
                "--isolation-diagnostic"
                    | "--manager-diagnostic"
                    | "--remote-ipc-diagnostic"
                    | "--desktop-diagnostic"
                    | "--ui-diagnostic"
            )
        })
}
pub struct AccountState(pub tauri::async_runtime::Mutex<Runtime>);
pub struct Runtime {
    pub db: Database,
    pub root: PathBuf,
    selection: Selection,
    profile_locks: HashMap<String, File>,
    #[cfg(windows)]
    browsers: HashMap<String, crate::windows_profiles::BrowserLifetime>,
    _instance: File,
}
impl Runtime {
    #[cfg(all(windows, debug_assertions))]
    pub(crate) fn track_browser(
        &mut self,
        id: &str,
        browser: crate::windows_profiles::BrowserLifetime,
    ) {
        self.browsers.insert(id.into(), browser);
    }
}
#[derive(Default)]
struct Selection {
    active: Option<String>,
    blocked: bool,
}
impl Selection {
    fn visible(&self, id: &str) -> bool {
        !self.blocked && self.active.as_deref() == Some(id)
    }
}
pub fn acquire(path: &Path) -> Result<File> {
    let file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(path)
        .map_err(|_| "Cannot open application/profile lock.")?;
    file.try_lock().map_err(
        |_| "Appterisk or this profile is already open. Close the other instance normally first.",
    )?;
    Ok(file)
}
impl AccountState {
    pub fn open(root: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&root).map_err(|_| "Cannot create application data directory.")?;
        let lock_path = root.join("application.lock");
        let start = std::time::Instant::now();
        let instance = loop {
            match acquire(&lock_path) {
                Ok(file) => break file,
                Err(error) => {
                    if !std::env::args().any(|a| a == "--wait-for-instance")
                        || start.elapsed() > std::time::Duration::from_secs(8)
                    {
                        return Err(error);
                    }
                    std::thread::sleep(std::time::Duration::from_millis(100));
                }
            }
        };
        let db = Database::open(&root.join("appterisk.sqlite3"))?;
        Ok(Self(tauri::async_runtime::Mutex::new(Runtime {
            db,
            root,
            selection: Selection::default(),
            profile_locks: HashMap::new(),
            #[cfg(windows)]
            browsers: HashMap::new(),
            _instance: instance,
        })))
    }
}
fn authorize(caller: &Webview) -> Result<()> {
    if crate::probe::trusted(caller).is_err() {
        Err("FORBIDDEN: only the trusted application shell may manage accounts.".into())
    } else {
        Ok(())
    }
}
fn production(app: &tauri::AppHandle) -> Result<()> {
    crate::desktop::ensure_unlocked(app)?;
    if diagnostic_mode() {
        Err("Account management is disabled in compatibility diagnostic mode.".into())
    } else if health::closing() {
        Err("Browser profiles are closing safely. Please wait.".into())
    } else {
        Ok(())
    }
}
pub fn label(id: &str) -> String {
    format!("account-{id}")
}
#[cfg(test)]
fn provider_url(url: &tauri::Url) -> bool {
    crate::platforms::get("whatsapp").unwrap().navigation(url)
}
pub fn external_url(url: &tauri::Url) -> bool {
    matches!(url.scheme(), "http" | "https")
        && url.host_str().is_some()
        && url.username().is_empty()
        && url.password().is_none()
        && url.as_str().len() <= 8192
}

#[derive(Serialize)]
pub struct Legacy {
    id: String,
    name: String,
}
#[derive(Serialize)]
pub struct Snapshot {
    accounts: Vec<Account>,
    settings: Settings,
    legacy: Vec<Legacy>,
    version: String,
    diagnostic: bool,
    runtime_version: Option<String>,
    synthetic: bool,
    platforms: &'static [crate::platforms::Platform],
}
fn snapshot(app: &tauri::AppHandle, r: &Runtime) -> Result<Snapshot> {
    crate::desktop::sync(app, r)?;
    let accounts = r.db.accounts()?;
    let legacy = LEGACY_IDS
        .iter()
        .enumerate()
        .filter(|(_, id)| !accounts.iter().any(|a| a.id == **id))
        .filter(|(_, id)| r.root.join("compatibility-profiles").join(id).is_dir())
        .map(|(i, id)| Legacy {
            id: (*id).into(),
            name: format!("Existing account {}", i + 1),
        })
        .collect();
    Ok(Snapshot {
        accounts,
        settings: r.db.settings()?,
        legacy,
        version: app.package_info().version.to_string(),
        diagnostic: diagnostic_mode(),
        runtime_version: runtime_version(),
        synthetic: synthetic_mode(),
        platforms: crate::platforms::REGISTRY,
    })
}
#[tauri::command]
pub async fn account_snapshot(
    app: tauri::AppHandle,
    caller: Webview,
    state: tauri::State<'_, AccountState>,
) -> Result<Snapshot> {
    authorize(&caller)?;
    let r = state.0.lock().await;
    snapshot(&app, &r)
}

#[tauri::command]
pub async fn restart_mode(app: tauri::AppHandle, caller: Webview, diagnostic: bool) -> Result<()> {
    authorize(&caller)?;
    crate::desktop::ensure_unlocked(&app)?;
    RESTART_MODE.store(if diagnostic { 1 } else { 2 }, Ordering::SeqCst);
    crate::probe::request_shutdown(&app, 0);
    Ok(())
}
#[tauri::command]
pub async fn account_add(
    app: tauri::AppHandle,
    caller: Webview,
    state: tauri::State<'_, AccountState>,
    name: String,
    legacy_id: Option<String>,
    platform: Option<String>,
) -> Result<Snapshot> {
    authorize(&caller)?;
    production(&app)?;
    let mut r = state.0.lock().await;
    if let Some(id) = legacy_id.as_deref() {
        if !LEGACY_IDS.contains(&id) {
            return Err("Unknown compatibility profile.".into());
        }
        let base = r
            .root
            .join("compatibility-profiles")
            .canonicalize()
            .map_err(|_| "Existing profile directory unavailable.")?;
        let path = base
            .join(id)
            .canonicalize()
            .map_err(|_| "Existing profile directory unavailable.")?;
        if !path.starts_with(&base) || path == base {
            return Err("Profile escaped its managed directory.".into());
        }
    }
    let platform = platform.as_deref().unwrap_or("whatsapp");
    if platform == "whatsapp" {
        r.db.add(&name, legacy_id.as_deref())?;
    } else {
        r.db.add_platform(&name, legacy_id.as_deref(), platform)?;
    }
    snapshot(&app, &r)
}
#[tauri::command]
pub async fn account_update(
    app: tauri::AppHandle,
    caller: Webview,
    state: tauri::State<'_, AccountState>,
    id: String,
    name: String,
    icon: String,
    notifications_enabled: bool,
) -> Result<Snapshot> {
    authorize(&caller)?;
    production(&app)?;
    let mut r = state.0.lock().await;
    r.db.update(&id, &name, &icon, notifications_enabled)?;
    snapshot(&app, &r)
}
#[tauri::command]
pub async fn account_reorder(
    app: tauri::AppHandle,
    caller: Webview,
    state: tauri::State<'_, AccountState>,
    ids: Vec<String>,
) -> Result<Snapshot> {
    authorize(&caller)?;
    production(&app)?;
    let mut r = state.0.lock().await;
    r.db.reorder(&ids)?;
    snapshot(&app, &r)
}
#[tauri::command]
pub async fn settings_save(
    app: tauri::AppHandle,
    caller: Webview,
    state: tauri::State<'_, AccountState>,
    settings: Settings,
) -> Result<Snapshot> {
    authorize(&caller)?;
    production(&app)?;
    let mut r = state.0.lock().await;
    // Last-active state is owned by successful native switching, never by stale UI preferences.
    let mut settings = settings;
    settings.last_active_account = r.db.settings()?.last_active_account;
    r.db.save_settings(&settings)?;
    snapshot(&app, &r)
}

pub(crate) fn layout(app: &tauri::AppHandle, r: &Runtime, bounds: Option<Bounds>) -> Result<()> {
    // Hide all before showing one; no overlapping native children during switching/dialogs.
    for a in r.db.accounts()? {
        if let Some(v) = app.get_webview(&label(&a.id)) {
            v.hide().map_err(|_| "Could not hide browser.")?;
        }
    }
    if let Some(id) = &r.selection.active
        && r.selection.visible(id)
        && !crate::desktop::blocked(app)
        && let Some(v) = app.get_webview(&label(id))
    {
        if let Some(bounds) = bounds {
            v.set_bounds(bounds.rect())
                .map_err(|_| "Could not resize browser.")?;
        }
        v.show().map_err(|_| "Could not show browser.")?;
    }
    Ok(())
}
pub(crate) fn block(
    app: &tauri::AppHandle,
    r: &mut Runtime,
    bounds: Bounds,
    blocked: bool,
) -> Result<()> {
    bounds
        .validate()
        .map_err(|_| "Invalid browser content bounds.")?;
    r.selection.blocked = blocked;
    layout(app, r, Some(bounds))
}
pub async fn initialize(
    app: &tauri::AppHandle,
    r: &mut Runtime,
    id: &str,
    bounds: Bounds,
) -> Result<()> {
    bounds
        .validate()
        .map_err(|_| "Invalid browser content bounds.")?;
    if health::overrides_present() {
        return Err("Remove WebView2 environment overrides before opening accounts. Profiles are unchanged.".into());
    }
    if !cfg!(windows) {
        return Err("This release requires Windows with WebView2 Evergreen.".into());
    }
    crate::desktop::sync(app, r)?;
    let account = r.db.account(id)?;
    let platform = crate::platforms::available(&account.platform)?;
    let name = label(id);
    if app.get_webview(&name).is_none() {
        #[cfg(windows)]
        if r.browsers
            .get(id)
            .is_some_and(|b| !b.exited.load(Ordering::SeqCst))
        {
            return Err("This profile is still closing. Retry shortly.".into());
        }
        let path = profile_path(&r.root, &account)?;
        std::fs::create_dir_all(&path)
            .map_err(|_| "Cannot create persistent account directory.")?;
        let canonical = path
            .canonicalize()
            .map_err(|_| "Cannot resolve account profile.")?;
        let root = r
            .root
            .canonicalize()
            .map_err(|_| "Cannot resolve application data.")?;
        let parent = path
            .parent()
            .ok_or("Profile parent unavailable.")?
            .canonicalize()
            .map_err(|_| "Cannot resolve profile namespace.")?;
        if !parent.starts_with(&root) || canonical != parent.join(id) {
            return Err("Profile escaped its managed directory.".into());
        }
        let browser_path = crate::profile_paths::browser_directory(&canonical)?;
        if !r.profile_locks.contains_key(id) {
            r.profile_locks.insert(
                id.into(),
                acquire(&canonical.join("appterisk-profile.lock"))?,
            );
        }
        let event_app = app.clone();
        let event_id = id.to_string();
        let title_app = app.clone();
        let title_id = id.to_string();
        let external_app = app.clone();
        let builder=WebviewBuilder::new(&name,WebviewUrl::External(platform.entry_url.parse().map_err(|_|"Invalid provider URL.")?))
            .data_directory(browser_path.clone()).incognito(false).devtools(false).disable_drag_drop_handler()
            .on_navigation(move |url| platform.navigation(url))
            .on_new_window(move|url,_|{if external_url(&url)&&!crate::desktop::locked(&external_app){let _=open::that_detached(url.as_str());}NewWindowResponse::Deny})
            .on_document_title_changed(move|_,title|crate::desktop::title(&title_app,&title_id,&title))
            .on_page_load(move|_,payload|{if platform.navigation(payload.url()){let _=event_app.emit_to("main","account-load",json!({"id":event_id,"loaded":matches!(payload.event(),tauri::webview::PageLoadEvent::Finished)}));}});
        let window = app.get_window("main").ok_or("Main window unavailable.")?;
        // Create offscreen so a new native child can never cover an open shell dialog.
        let view = window
            .add_child(
                builder,
                tauri::LogicalPosition::new(-32000.0, -32000.0),
                tauri::LogicalSize::new(bounds.width, bounds.height),
            )
            .map_err(|_| "Could not initialize account WebView. Check WebView2 and retry.")?;
        view.hide()
            .map_err(|_| "Could not hide initializing browser.")?;
        #[cfg(windows)]
        match crate::windows_profiles::attach(app, &view, id, &canonical) {
            Ok(browser) => {
                r.browsers.insert(id.into(), browser);
                crate::windows_profiles::account_events(app, &view, id)?;
                crate::windows_permissions::attach(app, &view, id)?;
                crate::windows_downloads::attach(app, &view, id)?;
                if crate::windows_notifications::attach(app, &view, id).is_err() {
                    health::log(
                        app,
                        "notification_api_unavailable",
                        json!({"profile_id":id}),
                    );
                }
            }
            Err(_) => {
                let _ = crate::windows_profiles::close_controller(&view);
                let _ = view.close();
                return Err(
                    "Persistent browser profile verification failed. Existing data is unchanged."
                        .into(),
                );
            }
        }
        health::log(
            app,
            "account_webview_created",
            json!({"profile_id":id,"absolute_directory":browser_path,"canonical_directory":canonical,"incognito":false,"cleanup":false}),
        );
    }
    Ok(())
}
pub async fn switch(
    app: &tauri::AppHandle,
    r: &mut Runtime,
    id: &str,
    bounds: Bounds,
) -> Result<()> {
    initialize(app, r, id, bounds).await?;
    let name = label(id);
    // Persist only after native creation succeeds. Switching reuses the loaded controller.
    r.db.activate(id)?;
    r.selection.active = Some(id.into());
    layout(app, r, Some(bounds))?;
    if !r.selection.blocked
        && !crate::desktop::blocked(app)
        && let Some(view) = app.get_webview(&name)
    {
        view.set_focus()
            .map_err(|_| "Could not focus the selected account.")?;
    }
    health::log(
        app,
        "account_selected",
        json!({"profile_id":id,"reload":false}),
    );
    Ok(())
}
#[tauri::command]
pub async fn account_switch(
    app: tauri::AppHandle,
    caller: Webview,
    state: tauri::State<'_, AccountState>,
    id: String,
    bounds: Bounds,
) -> Result<()> {
    authorize(&caller)?;
    production(&app)?;
    let mut r = state.0.lock().await;
    switch(&app, &mut r, &id, bounds).await?;
    crate::desktop::selected(&app, &id);
    Ok(())
}
#[tauri::command]
pub async fn accounts_initialize(
    app: tauri::AppHandle,
    caller: Webview,
    state: tauri::State<'_, AccountState>,
    bounds: Bounds,
) -> Result<()> {
    authorize(&caller)?;
    production(&app)?;
    let mut r = state.0.lock().await;
    for account in
        r.db.accounts()?
            .into_iter()
            .filter(|a| a.platform == "whatsapp")
    {
        initialize(&app, &mut r, &account.id, bounds).await?;
    }
    layout(&app, &r, Some(bounds))
}
#[tauri::command]
pub async fn account_layout(
    app: tauri::AppHandle,
    caller: Webview,
    state: tauri::State<'_, AccountState>,
    bounds: Bounds,
    blocked: bool,
) -> Result<()> {
    authorize(&caller)?;
    production(&app)?;
    bounds
        .validate()
        .map_err(|_| "Invalid browser content bounds.")?;
    let mut r = state.0.lock().await;
    block(&app, &mut r, bounds, blocked)
}
#[tauri::command]
pub async fn account_reload(
    app: tauri::AppHandle,
    caller: Webview,
    state: tauri::State<'_, AccountState>,
    id: String,
) -> Result<()> {
    authorize(&caller)?;
    production(&app)?;
    let r = state.0.lock().await;
    r.db.account(&id)?;
    app.get_webview(&label(&id))
        .ok_or("Select the account before reloading.")?
        .reload()
        .map_err(|_| "Could not reload account.".into())
}
#[tauri::command]
pub async fn account_remove(
    app: tauri::AppHandle,
    caller: Webview,
    state: tauri::State<'_, AccountState>,
    id: String,
    confirmed: bool,
) -> Result<Snapshot> {
    authorize(&caller)?;
    production(&app)?;
    let mut r = state.0.lock().await;
    remove_account(&app, &mut r, &id, confirmed)?;
    snapshot(&app, &r)
}
pub(crate) fn remove_account(
    app: &tauri::AppHandle,
    r: &mut Runtime,
    id: &str,
    confirmed: bool,
) -> Result<()> {
    if !confirmed {
        return Err("Removal requires confirmation.".into());
    }
    r.db.account(id)?;
    if let Some(v) = app.get_webview(&label(id)) {
        #[cfg(windows)]
        crate::windows_profiles::close_controller(&v)?;
        v.close()
            .map_err(|_| "Could not close this account. No profile was deleted.")?;
    }
    r.db.remove(id, true)?;
    if r.selection.active.as_deref() == Some(id) {
        r.selection.active = None;
    }
    health::log(
        app,
        "account_removed",
        json!({"profile_id":id,"browser_storage_deleted":false}),
    );
    Ok(())
}
#[cfg(windows)]
pub async fn close_for_shutdown(
    app: &tauri::AppHandle,
) -> Result<Vec<crate::windows_profiles::BrowserLifetime>> {
    let state = app.state::<AccountState>();
    let mut r = state.0.lock().await;
    for a in r.db.accounts()? {
        if let Some(v) = app.get_webview(&label(&a.id)) {
            crate::windows_profiles::close_controller(&v)?;
            v.close().map_err(|_| "Could not close account browser.")?;
        }
    }
    r.selection.active = None;
    Ok(r.browsers.values().cloned().collect())
}
#[cfg(not(windows))]
pub async fn close_for_shutdown(_: &tauri::AppHandle) -> Result<Vec<()>> {
    Ok(vec![])
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn selection_reuse_and_dialog_visibility() {
        let mut s = Selection {
            active: Some("a".into()),
            ..Default::default()
        };
        assert!(s.visible("a"));
        assert!(!s.visible("b"));
        s.blocked = true;
        assert!(!s.visible("a"));
        s.active = Some("b".into());
        assert!(!s.visible("b"));
        s.blocked = false;
        assert!(s.visible("b"));
    }
    #[test]
    fn profile_and_instance_lock_excludes_second_owner() {
        let path = std::env::temp_dir().join(format!("appterisk-lock-{}", uuid::Uuid::new_v4()));
        let lock = acquire(&path).unwrap();
        assert!(acquire(&path).is_err());
        drop(lock);
        drop(acquire(&path).unwrap());
        std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn navigation_rejects_untrusted_schemes_and_credentials() {
        assert!(provider_url(&"https://web.whatsapp.com/".parse().unwrap()));
        for url in [
            "https://web.whatsapp.com.evil.test/",
            "http://web.whatsapp.com/",
            "https://user@web.whatsapp.com/",
            "https://web.whatsapp.com:444/",
        ] {
            assert!(!provider_url(&url.parse().unwrap()));
        }
        for url in [
            "file:///C:/Windows",
            "javascript:alert(1)",
            "https://user:pass@example.com/",
        ] {
            assert!(!external_url(&url.parse().unwrap()));
        }
    }
}
