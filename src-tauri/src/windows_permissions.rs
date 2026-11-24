//! Origin-scoped host prompts. Default deny for unknown origins/kinds, never implicit allow.
use crate::{
    desktop::{self, DesktopState, Prompt},
    desktop_policy,
    windows_profiles::string_property,
};
use std::{cell::RefCell, collections::HashMap};
use tauri::{Manager, Webview};
use webview2_com::{
    Microsoft::Web::WebView2::Win32::*, PermissionRequestedEventHandler,
    SetPermissionStateCompletedHandler,
};
use windows::core::{HSTRING, Interface};
struct Pending {
    args: ICoreWebView2PermissionRequestedEventArgs,
    deferral: ICoreWebView2Deferral,
}
thread_local! {static PENDING:RefCell<HashMap<String,Pending>>=RefCell::new(HashMap::new());}
fn kind(value: COREWEBVIEW2_PERMISSION_KIND) -> Option<&'static str> {
    match value {
        COREWEBVIEW2_PERMISSION_KIND_MICROPHONE => Some("microphone"),
        COREWEBVIEW2_PERMISSION_KIND_CAMERA => Some("camera"),
        COREWEBVIEW2_PERMISSION_KIND_NOTIFICATIONS => Some("notifications"),
        COREWEBVIEW2_PERMISSION_KIND_CLIPBOARD_READ => Some("clipboard"),
        COREWEBVIEW2_PERMISSION_KIND_AUTOPLAY => Some("autoplay"),
        _ => None,
    }
}
fn native_kind(value: &str) -> Option<COREWEBVIEW2_PERMISSION_KIND> {
    match value {
        "microphone" => Some(COREWEBVIEW2_PERMISSION_KIND_MICROPHONE),
        "camera" => Some(COREWEBVIEW2_PERMISSION_KIND_CAMERA),
        "notifications" => Some(COREWEBVIEW2_PERMISSION_KIND_NOTIFICATIONS),
        "clipboard" => Some(COREWEBVIEW2_PERMISSION_KIND_CLIPBOARD_READ),
        "autoplay" => Some(COREWEBVIEW2_PERMISSION_KIND_AUTOPLAY),
        _ => None,
    }
}
pub fn reply(app: &tauri::AppHandle, id: &str, allow: bool) {
    let id = id.to_owned();
    let app = app.clone();
    let dispatcher = app.clone();
    let _ = dispatcher.run_on_main_thread(move || {
        let pending = PENDING.with(|map| map.borrow_mut().remove(&id));
        if let Some(p) = pending {
            unsafe {
                let _ = p.args.SetState(if allow && !desktop::locked(&app) {
                    COREWEBVIEW2_PERMISSION_STATE_ALLOW
                } else {
                    COREWEBVIEW2_PERMISSION_STATE_DENY
                });
                let _ = p.deferral.Complete();
            }
        }
    });
}
pub fn deny_pending(app: &tauri::AppHandle) {
    let _ = app.run_on_main_thread(move || {
        let pending = PENDING.with(|map| map.take());
        for (_, p) in pending {
            unsafe {
                let _ = p.args.SetState(COREWEBVIEW2_PERMISSION_STATE_DENY);
                let _ = p.deferral.Complete();
            }
        }
    });
}
pub fn apply(app: &tauri::AppHandle, id: &str, kind_name: &str, reload_page: bool) {
    let Some(kind) = native_kind(kind_name) else {
        return;
    };
    let decision = {
        let s = app.state::<DesktopState>();
        let Ok(d) = s.0.lock() else {
            return;
        };
        d.permissions
            .iter()
            .find(|p| p.account_id == id && p.kind == kind_name)
            .map(|p| p.decision.clone())
            .unwrap_or_else(|| "ask".into())
    };
    let permission_state = match decision.as_str() {
        "allow" => COREWEBVIEW2_PERMISSION_STATE_ALLOW,
        "deny" => COREWEBVIEW2_PERMISSION_STATE_DENY,
        _ => COREWEBVIEW2_PERMISSION_STATE_DEFAULT,
    };
    if let Some(view) = app.get_webview(&crate::accounts::label(id)) {
        let reload = view.clone();
        let _ = view.with_webview(move |native| unsafe {
            if let Ok(core) = native.controller().CoreWebView2()
                && let Ok(c) = core.cast::<ICoreWebView2_13>()
                && let Ok(p) = c.Profile()
                && let Ok(p) = p.cast::<ICoreWebView2Profile4>()
            {
                let _ = p.SetPermissionState(
                    kind,
                    &HSTRING::from(desktop_policy::ORIGIN),
                    permission_state,
                    &SetPermissionStateCompletedHandler::create(Box::new(move |status| {
                        if status.is_ok() && reload_page {
                            let _ = reload.reload();
                        }
                        Ok(())
                    })),
                );
            }
        });
    }
}
pub fn attach(app: &tauri::AppHandle, view: &Webview, id: &str) -> Result<(), String> {
    let app = app.clone();
    let id = id.to_owned();
    let (tx, rx) = std::sync::mpsc::channel();
    view.with_webview(move |native| {
        let result = (|| -> windows::core::Result<()> {
            unsafe {
                let core = native.controller().CoreWebView2()?;
                let mut token = 0;
                core.add_PermissionRequested(
                    &PermissionRequestedEventHandler::create(Box::new(move |_, args| {
                        let Some(args) = args else {
                            return Ok(());
                        };
                        let args3 = args.cast::<ICoreWebView2PermissionRequestedEventArgs3>()?;
                        args3.SetSavesInProfile(false)?;
                        if let Ok(args2) = args.cast::<ICoreWebView2PermissionRequestedEventArgs2>()
                        {
                            args2.SetHandled(true)?;
                        }
                        let uri = string_property(|s| args.Uri(s))?;
                        let origin = tauri::Url::parse(&uri)
                            .ok()
                            .filter(crate::accounts::provider_url)
                            .map(|u| u.origin().ascii_serialization());
                        let mut permission = COREWEBVIEW2_PERMISSION_KIND_UNKNOWN_PERMISSION;
                        args.PermissionKind(&mut permission)?;
                        let Some(kind) = kind(permission)
                            .filter(|_| origin.as_deref() == Some(desktop_policy::ORIGIN))
                        else {
                            args.SetState(COREWEBVIEW2_PERMISSION_STATE_DENY)?;
                            return Ok(());
                        };
                        let decision = {
                            let s = app.state::<DesktopState>();
                            let d = s.0.lock().map_err(|_| {
                                windows::core::Error::from_hresult(windows::core::HRESULT(
                                    0x80004005u32 as i32,
                                ))
                            })?;
                            if d.lock.locked || d.prompts.len() >= 8 {
                                "deny".to_owned()
                            } else {
                                d.permissions
                                    .iter()
                                    .find(|p| {
                                        p.account_id == id
                                            && p.kind == kind
                                            && p.origin == desktop_policy::ORIGIN
                                    })
                                    .map(|p| p.decision.clone())
                                    .unwrap_or_else(|| "ask".into())
                            }
                        };
                        if decision != "ask" {
                            args.SetState(if decision == "allow" {
                                COREWEBVIEW2_PERMISSION_STATE_ALLOW
                            } else {
                                COREWEBVIEW2_PERMISSION_STATE_DENY
                            })?;
                            return Ok(());
                        }
                        let ticket = uuid::Uuid::new_v4().to_string();
                        let deferral = args.GetDeferral()?;
                        PENDING.with(|p| {
                            p.borrow_mut().insert(
                                ticket.clone(),
                                Pending {
                                    args: args.clone(),
                                    deferral,
                                },
                            )
                        });
                        {
                            let s = app.state::<DesktopState>();
                            if let Ok(mut d) = s.0.lock() {
                                d.prompts.insert(
                                    ticket.clone(),
                                    Prompt {
                                        id: ticket.clone(),
                                        account_id: id.clone(),
                                        kind: kind.into(),
                                        origin: desktop_policy::ORIGIN.into(),
                                    },
                                );
                            }
                        }
                        let a = app.clone();
                        tauri::async_runtime::spawn(async move {
                            let _ = desktop::relayout(&a).await;
                            desktop::restore(&a, false);
                            desktop::emit(&a);
                        });
                        let a = app.clone();
                        std::thread::spawn(move || {
                            std::thread::sleep(std::time::Duration::from_secs(60));
                            let present = {
                                let s = a.state::<DesktopState>();
                                s.0.lock()
                                    .ok()
                                    .is_some_and(|mut d| d.prompts.remove(&ticket).is_some())
                            };
                            reply(&a, &ticket, false);
                            if present {
                                desktop::emit(&a);
                                let a = a.clone();
                                tauri::async_runtime::spawn(async move {
                                    let _ = desktop::relayout(&a).await;
                                });
                            }
                        });
                        Ok(())
                    })),
                    &mut token,
                )?;
                Ok(())
            }
        })();
        let _ = tx.send(result.map_err(|_| "Native permission management unavailable".to_string()));
    })
    .map_err(|_| "Cannot register native permissions")?;
    rx.recv_timeout(std::time::Duration::from_secs(5))
        .map_err(|_| "Permission registration timed out")?
}
