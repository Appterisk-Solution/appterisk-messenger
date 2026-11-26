//! Trusted desktop orchestration. External payloads never choose commands or filesystem paths.
use crate::{
    accounts::{self, AccountState, Runtime},
    app_lock::AppLock,
    database::Account,
    desktop_policy::{self, DesktopConfig, NotificationGate, Permission, Unread},
};
use serde::Serialize;
use std::{collections::HashMap, sync::Mutex, time::Instant};
use tauri::{Emitter, Manager, Webview};
use zeroize::Zeroizing;
type Result<T> = std::result::Result<T, String>;
pub struct DesktopState(pub Mutex<DesktopRuntime>);
pub struct DesktopRuntime {
    pub config: DesktopConfig,
    pub accounts: HashMap<String, Account>,
    pub permissions: Vec<Permission>,
    pub language: String,
    pub lock: AppLock,
    pub unread: HashMap<String, Unread>,
    pub downloads: HashMap<String, Download>,
    pub prompts: HashMap<String, Prompt>,
    pub gate: NotificationGate,
    pub overlays: u32,
    pub pending_account: Option<String>,
    pub pending_ticket: Option<String>,
    pub native_notifications: bool,
}
#[derive(Clone, Serialize)]
pub struct Download {
    pub id: String,
    pub account_id: String,
    pub name: String,
    pub received: u64,
    pub total: Option<u64>,
    pub state: String,
    pub saved_path: Option<String>,
}
#[derive(Clone, Serialize)]
pub struct Prompt {
    pub id: String,
    pub account_id: String,
    pub kind: String,
    pub origin: String,
}
#[derive(Clone, Serialize)]
pub struct DesktopSnapshot {
    pub config: DesktopConfig,
    pub locked: bool,
    pub lock_enabled: bool,
    pub unread: HashMap<String, Unread>,
    pub downloads: Vec<Download>,
    pub prompts: Vec<Prompt>,
    pub permissions: Vec<Permission>,
    pub pending_account: Option<String>,
    pub native_notifications: bool,
}
impl DesktopRuntime {
    fn snapshot(&self) -> DesktopSnapshot {
        DesktopSnapshot {
            config: self.config.clone(),
            locked: self.lock.locked,
            lock_enabled: self.lock.hash.is_some(),
            unread: self.unread.clone(),
            downloads: self.downloads.values().cloned().collect(),
            prompts: self.prompts.values().cloned().collect(),
            permissions: self.permissions.clone(),
            pending_account: self.pending_account.clone(),
            native_notifications: self.native_notifications,
        }
    }
}
pub fn setup(app: &tauri::AppHandle) -> Result<()> {
    let state = app.state::<AccountState>();
    let r = state.0.try_lock().map_err(|_| "Account startup is busy")?;
    let config = r.db.desktop_config()?;
    let hash = r.db.lock_hash()?;
    let permissions = r.db.permissions()?;
    let language = r.db.settings()?.language;
    let pending = std::env::args()
        .find_map(|a| desktop_policy::activation(&a))
        .map(|(id, _)| id);
    app.manage(DesktopState(Mutex::new(DesktopRuntime {
        config: config.clone(),
        accounts: r
            .db
            .accounts()?
            .into_iter()
            .map(|a| (a.id.clone(), a))
            .collect(),
        permissions,
        language,
        lock: AppLock::new(hash),
        unread: HashMap::new(),
        downloads: HashMap::new(),
        prompts: HashMap::new(),
        gate: NotificationGate::default(),
        overlays: 0,
        pending_account: pending,
        pending_ticket: std::env::args()
            .find_map(|a| desktop_policy::activation(&a))
            .map(|(_, ticket)| ticket),
        native_notifications: false,
    })));
    drop(r);
    if !accounts::synthetic_mode() {
        crate::tray::setup(app)?;
        #[cfg(windows)]
        crate::windows_desktop::register(app)?;
        if config.start_minimized
            && let Some(window) = app.get_window("main")
        {
            if config.minimize_to_tray {
                let _ = window.hide();
            } else {
                let _ = window.minimize();
            }
        }
        let app = app.clone();
        std::thread::spawn(move || {
            let mut last_input = 0;
            loop {
                std::thread::sleep(std::time::Duration::from_millis(750));
                if crate::health::closing() {
                    break;
                }
                #[cfg(windows)]
                let input = crate::windows_desktop::activity();
                #[cfg(not(windows))]
                let input = None;
                let should_lock = {
                    let state = app.state::<DesktopState>();
                    let Ok(mut d) = state.0.lock() else {
                        continue;
                    };
                    if let Some(input) = input
                        && input != last_input
                    {
                        last_input = input;
                        d.lock.last_activity = Instant::now();
                    }
                    cfg!(windows) && d.lock.should_lock(d.config.lock_timeout_seconds)
                };
                if should_lock {
                    let app = app.clone();
                    tauri::async_runtime::spawn(async move {
                        let _ = lock_now(&app).await;
                    });
                }
            }
        });
    }
    Ok(())
}
pub fn authorized(caller: &Webview) -> Result<()> {
    if crate::probe::trusted(caller).is_err() {
        Err("FORBIDDEN: trusted shell required.".into())
    } else {
        Ok(())
    }
}
pub fn locked(app: &tauri::AppHandle) -> bool {
    app.try_state::<DesktopState>()
        .and_then(|s| s.0.lock().ok().map(|d| d.lock.locked))
        .unwrap_or(false)
}
pub fn blocked(app: &tauri::AppHandle) -> bool {
    app.try_state::<DesktopState>()
        .and_then(|s| {
            s.0.lock()
                .ok()
                .map(|d| d.lock.locked || d.overlays > 0 || !d.prompts.is_empty())
        })
        .unwrap_or(false)
}
pub fn ensure_unlocked(app: &tauri::AppHandle) -> Result<()> {
    if crate::health::closing() {
        Err("Application is preparing to close.".into())
    } else if locked(app) {
        Err("APP_LOCKED".into())
    } else {
        Ok(())
    }
}
pub fn emit(app: &tauri::AppHandle) {
    if let Some(state) = app.try_state::<DesktopState>()
        && let Ok(d) = state.0.lock()
    {
        let _ = app.emit_to("main", "desktop-state", d.snapshot());
    }
}
pub fn sync(app: &tauri::AppHandle, r: &Runtime) -> Result<()> {
    if let Some(state) = app.try_state::<DesktopState>() {
        let mut d = state.0.lock().map_err(|_| "Desktop state unavailable")?;
        d.accounts =
            r.db.accounts()?
                .into_iter()
                .map(|a| (a.id.clone(), a))
                .collect();
        d.permissions = r.db.permissions()?;
        d.language = r.db.settings()?.language;
        let ids: Vec<_> = d.accounts.keys().cloned().collect();
        d.unread.retain(|id, _| ids.contains(id));
        drop(d);
        crate::tray::refresh(app);
        emit(app);
    }
    Ok(())
}
pub fn title(app: &tauri::AppHandle, id: &str, title: &str) {
    let platform = app.try_state::<DesktopState>().and_then(|s| {
        s.0.lock().ok().and_then(|d| {
            d.accounts
                .get(id)
                .and_then(|a| crate::platforms::get(&a.platform))
        })
    });
    if let Some(unread) = platform.and_then(|p| p.unread(title)) {
        if let Some(state) = app.try_state::<DesktopState>()
            && let Ok(mut d) = state.0.lock()
        {
            if !d.accounts.contains_key(id) {
                return;
            }
            d.unread.insert(id.into(), unread);
        }
        emit(app);
        crate::tray::refresh(app);
    }
}
pub fn incoming(app: &tauri::AppHandle, id: &str) {
    if let Some(state) = app.try_state::<DesktopState>()
        && let Ok(mut d) = state.0.lock()
        && d.accounts.contains_key(id)
    {
        let badge = d.unread.entry(id.into()).or_default();
        if !badge.unread {
            *badge = Unread {
                count: None,
                unread: true,
            };
        }
    }
    emit(app);
    crate::tray::refresh(app);
}
pub fn restore(app: &tauri::AppHandle, settings: bool) {
    if let Some(window) = app.get_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
    if let Some(shell) = app.get_webview("main") {
        let _ = shell.set_focus();
    }
    if settings {
        let _ = app.emit_to("main", "desktop-settings", true);
    }
}
pub fn activate(app: &tauri::AppHandle, id: &str, ticket: Option<&str>) {
    let Some(state) = app.try_state::<DesktopState>() else {
        return;
    };
    let Ok(mut d) = state.0.lock() else {
        return;
    };
    if !d.accounts.contains_key(id) {
        return;
    }
    d.pending_account = Some(id.into());
    d.pending_ticket = ticket.map(str::to_owned);
    let is_locked = d.lock.locked;
    drop(d);
    #[cfg(windows)]
    if let Some(ticket) = ticket
        && !is_locked
    {
        crate::windows_notifications::clicked(app, id, ticket);
    }
    restore(app, false);
    emit(app);
    let _ = app.emit_to("main", "desktop-select", id);
}
pub fn selected(app: &tauri::AppHandle, id: &str) {
    if let Some(s) = app.try_state::<DesktopState>()
        && let Ok(mut d) = s.0.lock()
        && d.pending_account.as_deref() == Some(id)
    {
        d.pending_account = None;
        d.pending_ticket = None;
    }
    emit(app);
}
pub fn second_instance(app: &tauri::AppHandle, args: Vec<String>) {
    if let Some((id, ticket)) = args.iter().find_map(|a| desktop_policy::activation(a)) {
        activate(app, &id, Some(&ticket));
    } else {
        restore(app, false);
    }
}
pub async fn relayout(app: &tauri::AppHandle) -> Result<()> {
    let state = app.state::<AccountState>();
    let r = state.0.lock().await;
    accounts::layout(app, &r, None)
}
pub async fn overlay(app: &tauri::AppHandle, begin: bool) {
    if let Some(s) = app.try_state::<DesktopState>()
        && let Ok(mut d) = s.0.lock()
    {
        d.overlays = if begin {
            d.overlays.saturating_add(1)
        } else {
            d.overlays.saturating_sub(1)
        };
    }
    let _ = relayout(app).await;
}
pub async fn lock_now(app: &tauri::AppHandle) -> Result<()> {
    {
        let state = app.state::<DesktopState>();
        let mut d = state.0.lock().map_err(|_| "Lock unavailable")?;
        if d.lock.hash.is_none() {
            return Err("Configure an application lock password first.".into());
        }
        d.lock.locked = true;
        d.prompts.clear();
    }
    // Hide every native child, including an optional diagnostic panel.
    for (label, view) in app.webviews() {
        if label != "main" {
            view.hide().map_err(|_| "Could not hide native view")?;
        }
    }
    if let Some(shell) = app.get_webview("main") {
        let _ = shell.set_focus();
    }
    #[cfg(windows)]
    {
        crate::windows_notifications::clear(app);
        crate::windows_permissions::deny_pending(app);
    }
    emit(app);
    crate::tray::refresh(app);
    Ok(())
}
#[tauri::command]
pub async fn desktop_snapshot(app: tauri::AppHandle, caller: Webview) -> Result<DesktopSnapshot> {
    authorized(&caller)?;
    let s = app.state::<DesktopState>();
    let d = s.0.lock().map_err(|_| "Desktop state unavailable")?;
    Ok(d.snapshot())
}
#[tauri::command]
pub async fn desktop_save(
    app: tauri::AppHandle,
    caller: Webview,
    config: DesktopConfig,
) -> Result<()> {
    authorized(&caller)?;
    save_config(&app, config).await
}
pub async fn save_config(app: &tauri::AppHandle, config: DesktopConfig) -> Result<()> {
    ensure_unlocked(app)?;
    config.validate()?;
    let state = app.state::<AccountState>();
    let mut r = state.0.lock().await;
    ensure_unlocked(app)?;
    let old = r.db.desktop_config()?;
    #[cfg(windows)]
    if old.launch_on_startup != config.launch_on_startup && !accounts::synthetic_mode() {
        crate::windows_desktop::startup(app, config.launch_on_startup)?;
    }
    r.db.save_desktop_config(&config)?;
    {
        let s = app.state::<DesktopState>();
        s.0.lock().map_err(|_| "Desktop state unavailable")?.config = config.clone();
    }
    if old.hide_previews != config.hide_previews || !config.notifications || config.dnd {
        #[cfg(windows)]
        crate::windows_notifications::clear(app);
    }
    drop(r);
    emit(app);
    crate::tray::refresh(app);
    Ok(())
}
#[tauri::command]
pub async fn desktop_lock(app: tauri::AppHandle, caller: Webview) -> Result<()> {
    authorized(&caller)?;
    lock_now(&app).await
}
#[tauri::command]
pub async fn desktop_unlock(
    app: tauri::AppHandle,
    caller: Webview,
    password: String,
) -> Result<()> {
    authorized(&caller)?;
    let password = Zeroizing::new(password);
    let hash = {
        let s = app.state::<DesktopState>();
        let d = s.0.lock().map_err(|_| "Lock unavailable")?;
        if !d.lock.allowed() {
            return Err("Wait before trying another password.".into());
        }
        d.lock
            .hash
            .clone()
            .ok_or("No application lock configured")?
    };
    let verify_hash = hash.clone();
    let valid = tauri::async_runtime::spawn_blocking(move || {
        crate::app_lock::verify(&password, &verify_hash)
    })
    .await
    .map_err(|_| "Unlock failed")?;
    {
        let s = app.state::<DesktopState>();
        let mut d = s.0.lock().map_err(|_| "Lock unavailable")?;
        if !valid || d.lock.hash.as_ref() != Some(&hash) {
            d.lock.failed();
            return Err("Incorrect password.".into());
        }
        d.lock.unlocked();
    }
    #[cfg(windows)]
    {
        let route = {
            let s = app.state::<DesktopState>();
            let d = s.0.lock().map_err(|_| "Lock unavailable")?;
            d.pending_account.clone().zip(d.pending_ticket.clone())
        };
        if let Some((id, ticket)) = route {
            crate::windows_notifications::clicked(&app, &id, &ticket);
        }
    }
    relayout(&app).await?;
    emit(&app);
    crate::tray::refresh(&app);
    Ok(())
}
#[tauri::command]
pub async fn desktop_password(
    app: tauri::AppHandle,
    caller: Webview,
    current_password: String,
    new_password: Option<String>,
) -> Result<()> {
    authorized(&caller)?;
    ensure_unlocked(&app)?;
    let current = Zeroizing::new(current_password);
    let new = new_password.map(Zeroizing::new);
    let state = app.state::<AccountState>();
    let mut r = state.0.lock().await;
    ensure_unlocked(&app)?;
    let old = r.db.lock_hash()?;
    let hash = tauri::async_runtime::spawn_blocking(move || {
        if old
            .as_ref()
            .is_some_and(|h| !crate::app_lock::verify(&current, h))
        {
            return Err("Incorrect current password.".into());
        }
        new.map(|p| crate::app_lock::hash(&p)).transpose()
    })
    .await
    .map_err(|_| "Password operation failed")??;
    r.db.save_lock_hash(hash.as_deref())?;
    {
        let s = app.state::<DesktopState>();
        let mut d = s.0.lock().map_err(|_| "Lock unavailable")?;
        d.lock.hash = hash;
        d.lock.unlocked();
    }
    drop(r);
    emit(&app);
    Ok(())
}
#[tauri::command]
pub async fn desktop_download_directory(
    app: tauri::AppHandle,
    caller: Webview,
) -> Result<Option<String>> {
    use tauri_plugin_dialog::DialogExt;
    authorized(&caller)?;
    ensure_unlocked(&app)?;
    overlay(&app, true).await;
    let a = app.clone();
    let selected =
        tauri::async_runtime::spawn_blocking(move || a.dialog().file().blocking_pick_folder())
            .await;
    overlay(&app, false).await;
    let path = selected
        .map_err(|_| "Folder picker failed")?
        .map(|f| f.into_path().map_err(|_| "Invalid folder path"))
        .transpose()?;
    path.map(|p| {
        p.canonicalize()
            .map_err(|_| "Cannot resolve download folder")
            .and_then(|p| {
                crate::profile_paths::browser_directory(&p)
                    .map(|p| p.to_string_lossy().into_owned())
            })
    })
    .transpose()
    .map_err(str::to_string)
}
#[tauri::command]
pub async fn desktop_download_cancel(
    app: tauri::AppHandle,
    caller: Webview,
    id: String,
) -> Result<()> {
    authorized(&caller)?;
    ensure_unlocked(&app)?;
    #[cfg(windows)]
    crate::windows_downloads::cancel(&app, &id);
    Ok(())
}
#[tauri::command]
pub async fn desktop_permission(
    app: tauri::AppHandle,
    caller: Webview,
    account_id: String,
    kind: String,
    decision: String,
) -> Result<()> {
    authorized(&caller)?;
    ensure_unlocked(&app)?;
    let state = app.state::<AccountState>();
    let mut r = state.0.lock().await;
    ensure_unlocked(&app)?;
    r.db.permission_save(&account_id, &kind, &decision)?;
    sync(&app, &r)?;
    drop(r);
    #[cfg(windows)]
    crate::windows_permissions::apply(&app, &account_id, &kind, true);
    Ok(())
}
#[tauri::command]
pub async fn desktop_permission_reply(
    app: tauri::AppHandle,
    caller: Webview,
    id: String,
    allow: bool,
    remember: bool,
) -> Result<()> {
    authorized(&caller)?;
    ensure_unlocked(&app)?;
    let prompt = {
        let s = app.state::<DesktopState>();
        s.0.lock()
            .map_err(|_| "Permission state unavailable")?
            .prompts
            .remove(&id)
            .ok_or("Permission request expired")?
    };
    if remember || allow && prompt.kind == "notifications" {
        let state = app.state::<AccountState>();
        let mut r = state.0.lock().await;
        ensure_unlocked(&app)?;
        if remember {
            r.db.permission_save(
                &prompt.account_id,
                &prompt.kind,
                if allow { "allow" } else { "deny" },
            )?;
        }
        if allow && prompt.kind == "notifications" {
            let a = r.db.account(&prompt.account_id)?;
            r.db.update(&a.id, &a.name, &a.icon, true)?;
        }
        sync(&app, &r)?;
    }
    #[cfg(windows)]
    crate::windows_permissions::reply(&app, &id, allow);
    #[cfg(windows)]
    if remember {
        crate::windows_permissions::apply(&app, &prompt.account_id, &prompt.kind, false);
    }
    relayout(&app).await?;
    emit(&app);
    Ok(())
}
