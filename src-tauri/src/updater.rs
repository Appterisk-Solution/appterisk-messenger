//! Host-owned official updater. Remote pages cannot choose endpoints or execute installers.
use crate::{accounts::AccountState, desktop, health};
use futures_util::future::{AbortHandle, Abortable};
use semver::Version;
use serde::{Deserialize, Serialize};
use std::{
    sync::{Mutex, atomic::Ordering},
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tauri::{Emitter, Manager, Webview};
use tauri_plugin_updater::{Update, UpdaterExt};
type Result<T> = std::result::Result<T, String>;
const LIMIT: u64 = 256 * 1024 * 1024;
pub const PUBLIC_KEY: Option<&str> = Some(match option_env!("APPTERISK_UPDATER_PUBKEY") {
    Some(key) => key,
    None => include_str!("../update-public-key.txt"),
});
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Channel {
    #[default]
    Stable,
    Beta,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Preferences {
    pub channel: Channel,
    pub automatic: bool,
}
impl Default for Preferences {
    fn default() -> Self {
        Self {
            channel: Channel::Stable,
            automatic: true,
        }
    }
}
#[derive(Clone, Serialize)]
pub struct Snapshot {
    pub preferences: Preferences,
    pub status: String,
    pub version: Option<String>,
    pub notes: String,
    pub received: u64,
    pub total: Option<u64>,
    pub last_check: Option<u64>,
    pub error: Option<String>,
    pub configured: bool,
    pub install_supported: bool,
}
pub struct State(Mutex<Runtime>);
struct Runtime {
    snapshot: Snapshot,
    update: Option<Update>,
    bytes: Option<Vec<u8>>,
    abort: Option<AbortHandle>,
}
fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
fn endpoint(channel: &Channel) -> Result<tauri::Url> {
    let source=match channel {
        Channel::Stable=>option_env!("APPTERISK_STABLE_ENDPOINT").unwrap_or("https://raw.githubusercontent.com/Appterisk-Solution/appterisk-messenger/update-channels/stable/update.json"),
        Channel::Beta=>option_env!("APPTERISK_BETA_ENDPOINT").unwrap_or("https://raw.githubusercontent.com/Appterisk-Solution/appterisk-messenger/update-channels/beta/update.json"),
    };
    let u: tauri::Url = source
        .parse()
        .map_err(|_| "Invalid build-time update endpoint")?;
    if u.scheme() != "https"
        || !u.username().is_empty()
        || u.password().is_some()
        || u.host_str().is_none()
        || u.fragment().is_some()
    {
        return Err("Updates require a valid HTTPS endpoint".into());
    }
    Ok(u)
}
fn newer(current: &Version, remote: &Version, channel: &Channel) -> bool {
    remote > current && (matches!(channel, Channel::Beta) || remote.pre.is_empty())
}
fn artifact(url: &tauri::Url, version: &str) -> bool {
    url.scheme() == "https"
        && url.host_str() == Some("github.com")
        && url.port_or_known_default() == Some(443)
        && url.username().is_empty()
        && url.password().is_none()
        && url.query().is_none()
        && url.fragment().is_none()
        && url.path().starts_with(&format!(
            "/Appterisk-Solution/appterisk-messenger/releases/download/v{version}/"
        ))
        && url.path().ends_with("x64-setup.exe")
}
fn supported() -> bool {
    cfg!(all(windows, target_arch = "x86_64"))
        && matches!(
            tauri::utils::platform::bundle_type(),
            Some(tauri::utils::config::BundleType::Nsis)
        )
}
fn authorized(caller: &Webview, app: &tauri::AppHandle) -> Result<()> {
    crate::probe::trusted(caller)
        .map_err(|_| "FORBIDDEN: only the local shell may manage updates.")?;
    desktop::ensure_unlocked(app)?;
    if health::closing() {
        return Err("Application is preparing to close.".into());
    }
    Ok(())
}
fn busy(s: &Snapshot) -> bool {
    matches!(s.status.as_str(), "checking" | "downloading" | "installing")
}
fn emit(app: &tauri::AppHandle) {
    if let Ok(r) = app.state::<State>().0.lock() {
        let _ = app.emit_to("main", "update-state", &r.snapshot);
    }
}
fn failure(app: &tauri::AppHandle, message: &str) {
    if let Ok(mut r) = app.state::<State>().0.lock() {
        r.snapshot.status = "error".into();
        r.snapshot.error = Some(message.into());
        r.abort = None;
    }
    emit(app);
}
pub fn setup(app: &tauri::AppHandle) -> Result<()> {
    let preferences = {
        let s = app.state::<AccountState>();
        let r =
            s.0.try_lock()
                .map_err(|_| "Update preferences unavailable")?;
        r.db.read_value("updater")?
            .map(|v| serde_json::from_str(&v).map_err(|_| "Invalid update preferences"))
            .transpose()?
            .unwrap_or_default()
    };
    let configured = PUBLIC_KEY.is_some_and(|v| !v.trim().is_empty());
    if configured {
        app.plugin(
            tauri_plugin_updater::Builder::new()
                .pubkey(PUBLIC_KEY.unwrap_or_default().trim())
                .build(),
        )
        .map_err(|_| "Cannot initialize official updater")?;
    }
    app.manage(State(Mutex::new(Runtime {
        snapshot: Snapshot {
            preferences,
            status: "idle".into(),
            version: None,
            notes: String::new(),
            received: 0,
            total: None,
            last_check: None,
            error: None,
            configured,
            install_supported: supported(),
        },
        update: None,
        bytes: None,
        abort: None,
    })));
    if !crate::accounts::synthetic_mode() && !crate::accounts::diagnostic_mode() && configured {
        let a = app.clone();
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(Duration::from_secs(20)).await;
            loop {
                let automatic = a
                    .state::<State>()
                    .0
                    .lock()
                    .ok()
                    .is_some_and(|r| r.snapshot.preferences.automatic);
                if automatic && !health::closing() {
                    let _ = check(&a).await;
                }
                tokio::time::sleep(Duration::from_secs(6 * 60 * 60)).await;
            }
        });
    }
    Ok(())
}
#[tauri::command]
pub fn update_snapshot(app: tauri::AppHandle, caller: Webview) -> Result<Snapshot> {
    crate::probe::trusted(&caller).map_err(|_| "FORBIDDEN: local shell only")?;
    Ok(app
        .state::<State>()
        .0
        .lock()
        .map_err(|_| "Updater unavailable")?
        .snapshot
        .clone())
}
#[tauri::command]
pub async fn update_preferences(
    app: tauri::AppHandle,
    caller: Webview,
    preferences: Preferences,
    beta_confirmed: bool,
) -> Result<()> {
    authorized(&caller, &app)?;
    if matches!(preferences.channel, Channel::Beta) && !beta_confirmed {
        return Err("Beta updates require explicit opt-in.".into());
    }
    let s = app.state::<AccountState>();
    let mut accounts = s.0.lock().await;
    authorized(&caller, &app)?;
    {
        let state = app.state::<State>();
        let mut r = state.0.lock().map_err(|_| "Updater unavailable")?;
        if busy(&r.snapshot) {
            return Err("An update operation is already running.".into());
        }
        accounts.db.write_value(
            "updater",
            Some(
                &serde_json::to_string(&preferences)
                    .map_err(|_| "Cannot save update preferences")?,
            ),
        )?;
        r.snapshot.preferences = preferences;
        r.snapshot.status = "idle".into();
        r.snapshot.version = None;
        r.snapshot.notes.clear();
        r.snapshot.error = None;
        r.update = None;
        r.bytes = None;
    }
    emit(&app);
    Ok(())
}
async fn check(app: &tauri::AppHandle) -> Result<()> {
    let channel = {
        let s = app.state::<State>();
        let mut r = s.0.lock().map_err(|_| "Updater unavailable")?;
        if busy(&r.snapshot) || r.bytes.is_some() {
            return Err("An update operation is already running.".into());
        }
        if !r.snapshot.configured {
            return Err("Update signing is not configured for this build.".into());
        }
        r.update = None;
        r.snapshot.status = "checking".into();
        r.snapshot.error = None;
        r.snapshot.last_check = Some(now());
        r.snapshot.preferences.channel.clone()
    };
    emit(app);
    let result: Result<()> = async {
        let comparator_channel = channel.clone();
        let updater = app
            .updater_builder()
            .target("windows-x86_64-nsis")
            .endpoints(vec![endpoint(&channel)?])
            .map_err(|_| "Invalid update endpoint")?
            .timeout(Duration::from_secs(30))
            .version_comparator(move |current, remote| {
                newer(&current, &remote.version, &comparator_channel)
            })
            .build()
            .map_err(|_| "Cannot initialize update check")?;
        let update = updater
            .check()
            .await
            .map_err(|_| "Update check failed. Messaging remains available.")?;
        if let Some(ref u) = update {
            let version = Version::parse(&u.version).map_err(|_| "Invalid update version")?;
            if !newer(&app.package_info().version, &version, &channel)
                || !artifact(&u.download_url, &u.version)
                || u.signature.trim().is_empty()
            {
                return Err("Update manifest failed validation.".into());
            }
        }
        let s = app.state::<State>();
        let mut r = s.0.lock().map_err(|_| "Updater unavailable")?;
        r.snapshot.version = update.as_ref().map(|u| u.version.clone());
        r.snapshot.notes = update
            .as_ref()
            .and_then(|u| u.body.as_ref())
            .map(|n| {
                n.chars()
                    .filter(|c| !c.is_control() || matches!(c, '\n' | '\t'))
                    .take(20000)
                    .collect()
            })
            .unwrap_or_default();
        r.snapshot.status = if update.is_some() {
            "available"
        } else {
            "current"
        }
        .into();
        r.update = update;
        Ok(())
    }
    .await;
    if let Err(ref e) = result {
        failure(app, e);
    } else {
        emit(app);
    }
    result
}
#[tauri::command]
pub async fn update_check(app: tauri::AppHandle, caller: Webview) -> Result<()> {
    authorized(&caller, &app)?;
    check(&app).await
}
#[tauri::command]
pub async fn update_download(app: tauri::AppHandle, caller: Webview) -> Result<()> {
    authorized(&caller, &app)?;
    let (update, registration, abort) = {
        let s = app.state::<State>();
        let mut r = s.0.lock().map_err(|_| "Updater unavailable")?;
        if busy(&r.snapshot) {
            return Err("An update operation is already running.".into());
        }
        if !r.snapshot.install_supported {
            return Err(
                "Automatic installation currently requires the Windows x64 NSIS installation."
                    .into(),
            );
        }
        let update = r.update.clone().ok_or("Check for updates first.")?;
        let (abort, registration) = AbortHandle::new_pair();
        r.abort = Some(abort.clone());
        r.bytes = None;
        r.snapshot.status = "downloading".into();
        r.snapshot.received = 0;
        r.snapshot.total = None;
        r.snapshot.error = None;
        (update, registration, abort)
    };
    emit(&app);
    let progress = app.clone();
    let result = Abortable::new(
        update.download(
            move |count, total| {
                if let Ok(mut r) = progress.state::<State>().0.lock() {
                    r.snapshot.received = r.snapshot.received.saturating_add(count as u64);
                    r.snapshot.total = total;
                    if r.snapshot.received > LIMIT || total.is_some_and(|n| n > LIMIT) {
                        abort.abort();
                    }
                }
                emit(&progress);
            },
            || {},
        ),
        registration,
    )
    .await;
    match result {
        Ok(Ok(bytes)) => {
            let s = app.state::<State>();
            let mut r = s.0.lock().map_err(|_| "Updater unavailable")?;
            r.bytes = Some(bytes);
            r.abort = None;
            r.snapshot.status = "ready".into();
        }
        Ok(Err(_)) => {
            failure(
                &app,
                "Update download or signature verification failed. Nothing was installed.",
            );
            return Err(
                "Update download or signature verification failed. Nothing was installed.".into(),
            );
        }
        Err(_) => {
            failure(
                &app,
                "Update download cancelled or exceeded the size limit.",
            );
            return Ok(());
        }
    }
    emit(&app);
    Ok(())
}
#[tauri::command]
pub fn update_cancel(app: tauri::AppHandle, caller: Webview) -> Result<()> {
    authorized(&caller, &app)?;
    if let Ok(r) = app.state::<State>().0.lock()
        && let Some(a) = &r.abort
    {
        a.abort();
    }
    Ok(())
}
#[tauri::command]
pub async fn update_install(app: tauri::AppHandle, caller: Webview, confirmed: bool) -> Result<()> {
    authorized(&caller, &app)?;
    if !confirmed {
        return Err("Confirm installation and restart first.".into());
    }
    if !supported() {
        return Err(
            "Automatic installation currently requires the Windows x64 NSIS installation.".into(),
        );
    }
    let (update, bytes) = {
        let s = app.state::<State>();
        let mut r = s.0.lock().map_err(|_| "Updater unavailable")?;
        if r.snapshot.status != "ready" {
            return Err("Download and verify an update first.".into());
        }
        let update = r.update.clone().ok_or("No verified update")?;
        let bytes = r.bytes.take().ok_or("No verified update")?;
        update
            .verify(&bytes)
            .map_err(|_| "Update signature verification failed")?;
        r.snapshot.status = "installing".into();
        (update, bytes)
    };
    emit(&app);
    let mut owns_shutdown = false;
    let result=async {
        // This lock serializes with all existing metadata transactions before closing starts.
        {let state=app.state::<AccountState>();let _guard=state.0.lock().await;
            let d=app.state::<desktop::DesktopState>();let d=d.0.lock().map_err(|_|"Desktop unavailable")?;
            if d.downloads.values().any(|v|matches!(v.state.as_str(),"choosing"|"downloading")) || !d.prompts.is_empty(){return Err("Finish active downloads and permission prompts before installing.".into());}
            if health::SHUTTING_DOWN.compare_exchange(false,true,Ordering::SeqCst,Ordering::SeqCst).is_err(){return Err("Application is already closing.".into());} owns_shutdown=true;
        }
        crate::probe::prepare_shutdown(&app).await.map_err(|_|"Browser shutdown failed. Nothing was installed; retry after closing other instances.")?;
        crate::accounts::release_for_update(&app).await?;
        health::SHUTDOWN_FINISHED.store(true,Ordering::SeqCst);
        // The official plugin re-verifies bytes, starts NSIS, restarts and exits on Windows.
        tauri::async_runtime::spawn_blocking(move||update.install(bytes).map_err(|_|"Installer could not start. Select an account to resume messaging.".to_string())).await.map_err(|_|"Installer task failed")?
    }.await;
    if let Err(ref e) = result {
        if owns_shutdown {
            let _ = crate::accounts::recover_after_update(&app).await;
            health::SHUTDOWN_FINISHED.store(false, Ordering::SeqCst);
            health::SHUTTING_DOWN.store(false, Ordering::SeqCst);
        }
        failure(&app, e);
    }
    result
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn channel_downgrade_and_artifact_policy() {
        let v = |s| Version::parse(s).unwrap();
        assert!(newer(&v("0.5.0"), &v("0.5.1"), &Channel::Stable));
        assert!(!newer(&v("0.5.0"), &v("0.6.0-beta.1"), &Channel::Stable));
        assert!(newer(&v("0.5.0"), &v("0.6.0-beta.1"), &Channel::Beta));
        assert!(!newer(&v("0.6.0-beta.1"), &v("0.5.0"), &Channel::Stable));
        assert!(!newer(&v("0.5.0"), &v("0.5.0"), &Channel::Beta));
        for u in [
            "http://github.com/Appterisk-Solution/appterisk-messenger/releases/download/v0.5.1/App_x64-setup.exe",
            "https://evil.test/App_x64-setup.exe",
            "https://github.com/Appterisk-Solution/other/releases/download/v0.5.1/App_x64-setup.exe",
            "https://github.com/Appterisk-Solution/appterisk-messenger/releases/download/v0.5.1/App.msi",
        ] {
            assert!(!artifact(&u.parse().unwrap(), "0.5.1"));
        }
        assert!(artifact(&"https://github.com/Appterisk-Solution/appterisk-messenger/releases/download/v0.5.1/App_x64-setup.exe".parse().unwrap(),"0.5.1"));
        assert!(endpoint(&Channel::Stable).unwrap() != endpoint(&Channel::Beta).unwrap());
    }
}
