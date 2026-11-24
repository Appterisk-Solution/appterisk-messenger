//! Native WebView2 notification source; no JS shim, generated messages, or payload logging.
use crate::{
    desktop::{self, DesktopState},
    desktop_policy,
    windows_profiles::string_property,
};
use std::{cell::RefCell, collections::HashMap};
use tauri::{Manager, Webview};
use webview2_com::{
    Microsoft::Web::WebView2::Win32::*, NotificationCloseRequestedEventHandler,
    NotificationReceivedEventHandler,
};
use windows::{
    Data::Xml::Dom::XmlDocument,
    Foundation::TypedEventHandler,
    UI::Notifications::{ToastDismissalReason, ToastNotification},
    core::{BOOL, HSTRING, Interface},
};
struct Pending {
    created: std::time::Instant,
    account: String,
    event: String,
    notification: ICoreWebView2Notification,
    toast: ToastNotification,
}
thread_local! {static PENDING:RefCell<HashMap<String,Pending>>=RefCell::new(HashMap::new());}
fn finish(app: &tauri::AppHandle, ticket: &str, clicked: bool) {
    let pending = PENDING.with(|map| map.borrow_mut().remove(ticket));
    if let Some(p) = pending {
        unsafe {
            if clicked {
                let _ = p.notification.ReportClicked();
            }
            let _ = p.notification.ReportClosed();
        }
        if let Ok(notifier) = crate::windows_desktop::notifier() {
            let _ = notifier.Hide(&p.toast);
        }
        let s = app.state::<DesktopState>();
        if let Ok(mut d) = s.0.lock() {
            d.gate.release(&p.event);
        }
    }
}
pub fn clear(app: &tauri::AppHandle) {
    let a = app.clone();
    let _ = app.run_on_main_thread(move || {
        let ids = PENDING.with(|p| p.borrow().keys().cloned().collect::<Vec<_>>());
        for id in ids {
            finish(&a, &id, false);
        }
    });
}
pub fn clicked(app: &tauri::AppHandle, account: &str, ticket: &str) {
    let a = app.clone();
    let account = account.to_string();
    let ticket = ticket.to_string();
    let _ = app.run_on_main_thread(move || {
        let matches = PENDING.with(|p| {
            p.borrow()
                .get(&ticket)
                .is_some_and(|n| n.account == account)
        });
        if matches {
            finish(&a, &ticket, true);
        }
    });
}
pub fn attach(app: &tauri::AppHandle, view: &Webview, id: &str) -> Result<(), String> {
    let app = app.clone();
    let id = id.to_string();
    let (tx, rx) = std::sync::mpsc::channel();
    view.with_webview(move |native| {
        let result = (|| -> windows::core::Result<()> {
            unsafe {
                let core = native
                    .controller()
                    .CoreWebView2()?
                    .cast::<ICoreWebView2_24>()?;
                let mut token = 0;
                let event_app = app.clone();
                core.add_NotificationReceived(
                    &NotificationReceivedEventHandler::create(Box::new(move |_, args| {
                        let Some(args) = args else {
                            return Ok(());
                        };
                        args.SetHandled(true)?;
                        let origin = string_property(|s| args.SenderOrigin(s))?;
                        if !tauri::Url::parse(&origin).map(|url|crate::accounts::provider_url(&url)).unwrap_or(false) {
                            return Ok(());
                        }
                        let notification = args.Notification()?;
                        let event = format!("{}:{:p}", id, notification.as_raw());
                        if PENDING.with(|p|p.borrow().values().any(|n|n.event==event)){return Ok(());}
                        let oldest=PENDING.with(|p|{let pending=p.borrow();if pending.len()>=200{pending.iter().min_by_key(|(_,n)|n.created).map(|(id,_)|id.clone())}else{None}});
                        if let Some(oldest)=oldest{finish(&event_app,&oldest,false);}
                        crate::health::log(&event_app,"notification_received",serde_json::json!({"profile_id":id}));
                        desktop::incoming(&event_app, &id);
                        let data = {
                            let s = event_app.state::<DesktopState>();
                            let mut d = s.0.lock().map_err(|_| {
                                windows::core::Error::from_hresult(windows::core::HRESULT(
                                    0x80004005u32 as i32,
                                ))
                            })?;
                            let Some(account) = d.accounts.get(&id).cloned() else {
                                return Ok(());
                            };
                            let config = d.config.clone();
                            let private = config.hide_previews || d.lock.locked;
                            if !d
                                .gate
                                .accept(&event, &config, account.notifications_enabled)
                            {
                                return Ok(());
                            }
                            (account, config, private)
                        };
                        let (account, config, private) = data;
                        let ticket = uuid::Uuid::new_v4().to_string();
                        let mut silent = BOOL(0);
                        notification.IsSilent(&mut silent)?;
                        let (title, body) = if private {
                            (String::new(), String::new())
                        } else {
                            (
                                string_property(|s| notification.Title(s))?,
                                string_property(|s| notification.Body(s))?,
                            )
                        };
                        let xml = desktop_policy::notification_xml(
                            &account.name,
                            &title,
                            &body,
                            &id,
                            &ticket,
                            private,
                            !config.sound || silent.as_bool(),
                        );
                        let document = XmlDocument::new()?;
                        document.LoadXml(&HSTRING::from(xml))?;
                        let toast = ToastNotification::CreateToastNotification(&document)?;
                        let a = event_app.clone();
                        let close_ticket = ticket.clone();
                        let close_app = event_app.clone();
                        notification.add_CloseRequested(
                            &NotificationCloseRequestedEventHandler::create(Box::new(
                                move |_, _| {
                                    finish(&close_app, &close_ticket, false);
                                    Ok(())
                                },
                            )),
                            &mut token,
                        )?;
                        let dismissed_ticket = ticket.clone();
                        toast.Dismissed(&TypedEventHandler::new(
                            move |_,
                                  args: windows::core::Ref<
                                '_,
                                windows::UI::Notifications::ToastDismissedEventArgs,
                            >| {
                                if let Some(args) = args.as_ref()
                                    && args.Reason()? == ToastDismissalReason::UserCanceled
                                {
                                    let app = a.clone();
                                    let id = dismissed_ticket.clone();
                                    let _ = a.run_on_main_thread(move || finish(&app, &id, false));
                                }
                                Ok(())
                            },
                        ))?;
                        let notifier = crate::windows_desktop::notifier()?;
                        if let Err(e) = notifier.Show(&toast) {
                            crate::health::log(&event_app,"toast_submission_failed",serde_json::json!({"profile_id":id,"hresult":e.code().0}));
                            let s = event_app.state::<DesktopState>();
                            if let Ok(mut d) = s.0.lock() {
                                d.gate.release(&event);
                            }
                            return Err(e);
                        }
                        notification.ReportShown()?;
                        crate::health::log(&event_app,"toast_submitted",serde_json::json!({"profile_id":id,"private":private,"silent":!config.sound||silent.as_bool()}));
                        // Retain callbacks/COM state for real notifications only. Bound by the notification gate.
                        PENDING.with(|p| {
                            p.borrow_mut().insert(
                                ticket,
                                Pending {
                                    created:std::time::Instant::now(),
                                    account: id.clone(),
                                    event,
                                    notification,
                                    toast,
                                },
                            )
                        });
                        Ok(())
                    })),
                    &mut token,
                )?;
                if let Ok(mut d) = app.state::<DesktopState>().0.lock() {
                    d.native_notifications = true;
                }
                Ok(())
            }
        })();
        let _ =
            tx.send(result.map_err(|_| "Native WebView2 notification API unavailable".to_string()));
    })
    .map_err(|_| "Cannot register native notifications")?;
    rx.recv_timeout(std::time::Duration::from_secs(5))
        .map_err(|_| "Notification registration timed out")?
}
