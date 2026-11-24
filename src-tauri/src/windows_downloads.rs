//! Supported WebView2 download events. Staging plus atomic no-clobber finalization.
use crate::{
    desktop::{self, DesktopState, Download},
    desktop_policy,
    windows_profiles::string_property,
};
use std::{cell::RefCell, collections::HashMap, path::PathBuf};
use tauri::{Manager, Webview};
use tauri_plugin_dialog::DialogExt;
use webview2_com::{
    BytesReceivedChangedEventHandler, DownloadStartingEventHandler,
    Microsoft::Web::WebView2::Win32::*, StateChangedEventHandler,
};
use windows::core::{HSTRING, Interface};
struct Request {
    args: ICoreWebView2DownloadStartingEventArgs,
    deferral: ICoreWebView2Deferral,
    operation: ICoreWebView2DownloadOperation,
}
struct Transfer {
    operation: ICoreWebView2DownloadOperation,
    file: tempfile::TempPath,
    directory: PathBuf,
    name: String,
}
struct Destination {
    file: tempfile::TempPath,
    directory: PathBuf,
    name: String,
}
thread_local! {static REQUESTS:RefCell<HashMap<String,Request>>=RefCell::new(HashMap::new());static TRANSFERS:RefCell<HashMap<String,Transfer>>=RefCell::new(HashMap::new());}
fn update(app: &tauri::AppHandle, id: &str, state: &str, path: Option<String>) {
    let s = app.state::<DesktopState>();
    if let Ok(mut d) = s.0.lock()
        && let Some(item) = d.downloads.get_mut(id)
    {
        item.state = state.into();
        item.saved_path = path;
    }
    desktop::emit(app);
}
fn destination(
    app: &tauri::AppHandle,
    name: &str,
    config: &desktop_policy::DesktopConfig,
) -> Result<Option<Destination>, String> {
    let fallback = config
        .download_directory
        .as_ref()
        .map(PathBuf::from)
        .or_else(|| app.path().download_dir().ok())
        .ok_or("Downloads directory unavailable")?;
    let selected = if config.ask_before_saving {
        app.dialog()
            .file()
            .set_directory(&fallback)
            .set_file_name(name)
            .blocking_save_file()
            .map(|p| p.into_path().map_err(|_| "Invalid save path"))
            .transpose()?
    } else {
        Some(fallback.join(name))
    };
    let Some(selected) = selected else {
        return Ok(None);
    };
    let directory = selected
        .parent()
        .ok_or("Save directory unavailable")?
        .canonicalize()
        .map_err(|_| "Save directory unavailable")?;
    if !directory.is_absolute() || !directory.is_dir() {
        return Err("Invalid save directory".into());
    }
    let name = desktop_policy::safe_filename(
        &selected
            .file_name()
            .ok_or("Invalid filename")?
            .to_string_lossy(),
    );
    let file = tempfile::Builder::new()
        .prefix(".appterisk-download-")
        .suffix(".part")
        .tempfile_in(&directory)
        .map_err(|_| "Cannot create download staging file")?
        .into_temp_path();
    Ok(Some(Destination {
        file,
        directory,
        name,
    }))
}
fn complete_request(
    app: &tauri::AppHandle,
    id: &str,
    destination: Result<Option<Destination>, String>,
) {
    let request = REQUESTS.with(|p| p.borrow_mut().remove(id));
    let Some(request) = request else {
        return;
    };
    unsafe {
        match destination {
            Ok(Some(destination)) => {
                let path = crate::profile_paths::browser_directory(
                    &destination
                        .file
                        .canonicalize()
                        .unwrap_or_else(|_| destination.file.to_path_buf()),
                );
                if let Ok(path) = path
                    && request
                        .args
                        .SetResultFilePath(&HSTRING::from(path.to_string_lossy().as_ref()))
                        .is_ok()
                {
                    TRANSFERS.with(|p| {
                        p.borrow_mut().insert(
                            id.into(),
                            Transfer {
                                operation: request.operation,
                                file: destination.file,
                                directory: destination.directory,
                                name: destination.name,
                            },
                        )
                    });
                    update(app, id, "downloading", None);
                } else {
                    let _ = request.args.SetCancel(true);
                    update(app, id, "failed", None);
                }
            }
            Ok(None) => {
                let _ = request.args.SetCancel(true);
                update(app, id, "cancelled", None);
            }
            Err(_) => {
                let _ = request.args.SetCancel(true);
                update(app, id, "failed", None);
            }
        }
        let _ = request.deferral.Complete();
    }
}
pub fn cancel(app: &tauri::AppHandle, id: &str) {
    let a = app.clone();
    let id = id.to_string();
    let _ = app.run_on_main_thread(move || {
        let request = REQUESTS.with(|p| p.borrow_mut().remove(&id));
        if let Some(p) = request {
            unsafe {
                let _ = p.args.SetCancel(true);
                let _ = p.deferral.Complete();
            }
        }
        let operation = TRANSFERS.with(|p| p.borrow().get(&id).map(|t| t.operation.clone()));
        if let Some(op) = operation {
            unsafe {
                let _ = op.Cancel();
            }
        }
        update(&a, &id, "cancelled", None);
    });
}
pub fn shutdown(app: &tauri::AppHandle) {
    let _ = app.run_on_main_thread(move || {
        let requests = REQUESTS.with(|p| p.take());
        for (_, r) in requests {
            unsafe {
                let _ = r.args.SetCancel(true);
                let _ = r.deferral.Complete();
            }
        }
        let transfers = TRANSFERS.with(|p| p.take());
        for (_, t) in transfers {
            unsafe {
                let _ = t.operation.Cancel();
            }
        }
    });
}
pub fn attach(app: &tauri::AppHandle, view: &Webview, account: &str) -> Result<(), String> {
    let app = app.clone();
    let account = account.to_string();
    let (tx, rx) = std::sync::mpsc::channel();
    view.with_webview(move |native| {
        let result = (|| -> windows::core::Result<()> {
            unsafe {
                let core = native
                    .controller()
                    .CoreWebView2()?
                    .cast::<ICoreWebView2_4>()?;
                let mut token = 0;
                core.add_DownloadStarting(
                    &DownloadStartingEventHandler::create(Box::new(move |_, args| {
                        let Some(args) = args else {
                            return Ok(());
                        };
                        args.SetHandled(true)?;
                        if desktop::locked(&app) || REQUESTS.with(|p| p.borrow().len() >= 4) {
                            args.SetCancel(true)?;
                            return Ok(());
                        }
                        let suggested = string_property(|s| args.ResultFilePath(s))?;
                        let name = desktop_policy::safe_filename(&suggested);
                        let id = uuid::Uuid::new_v4().to_string();
                        let operation = args.DownloadOperation()?;
                        let deferral = args.GetDeferral()?;
                        let progress_app = app.clone();
                        let progress_id = id.clone();
                        operation.add_BytesReceivedChanged(
                            &BytesReceivedChangedEventHandler::create(Box::new(
                                move |sender, _| {
                                    if let Some(op) = sender {
                                        let mut received = 0;
                                        let mut total = 0;
                                        op.BytesReceived(&mut received)?;
                                        op.TotalBytesToReceive(&mut total)?;
                                        {
                                            let s = progress_app.state::<DesktopState>();
                                            if let Ok(mut d) = s.0.lock()
                                                && let Some(item) =
                                                    d.downloads.get_mut(&progress_id)
                                            {
                                                item.received = received.max(0) as u64;
                                                item.total = if total >= 0 {
                                                    Some(total as u64)
                                                } else {
                                                    None
                                                };
                                            }
                                        }
                                        desktop::emit(&progress_app);
                                    }
                                    Ok(())
                                },
                            )),
                            &mut token,
                        )?;
                        let state_app = app.clone();
                        let state_id = id.clone();
                        operation.add_StateChanged(
                            &StateChangedEventHandler::create(Box::new(move |sender, _| {
                                let Some(op) = sender else {
                                    return Ok(());
                                };
                                let mut state = COREWEBVIEW2_DOWNLOAD_STATE_IN_PROGRESS;
                                op.State(&mut state)?;
                                if state == COREWEBVIEW2_DOWNLOAD_STATE_COMPLETED {
                                    let transfer =
                                        TRANSFERS.with(|p| p.borrow_mut().remove(&state_id));
                                    if let Some(t) = transfer {
                                        match desktop_policy::persist_download(
                                            t.file,
                                            &t.directory,
                                            &t.name,
                                        ) {
                                            Ok(path) => update(
                                                &state_app,
                                                &state_id,
                                                "completed",
                                                Some(path.to_string_lossy().into()),
                                            ),
                                            Err(_) => update(&state_app, &state_id, "failed", None),
                                        }
                                    }
                                } else if state == COREWEBVIEW2_DOWNLOAD_STATE_INTERRUPTED {
                                    let mut reason=COREWEBVIEW2_DOWNLOAD_INTERRUPT_REASON_NONE;op.InterruptReason(&mut reason)?;
                                    crate::health::log(&state_app,"download_interrupted",serde_json::json!({"download_id":state_id,"reason":reason.0}));
                                    TRANSFERS.with(|p| p.borrow_mut().remove(&state_id));
                                    update(&state_app, &state_id, "interrupted", None);
                                }
                                Ok(())
                            })),
                            &mut token,
                        )?;
                        REQUESTS.with(|p| {
                            p.borrow_mut().insert(
                                id.clone(),
                                Request {
                                    args: args.clone(),
                                    deferral,
                                    operation,
                                },
                            )
                        });
                        let config = {
                            let s = app.state::<DesktopState>();
                            let mut d = s.0.lock().map_err(|_| {
                                windows::core::Error::from_hresult(windows::core::HRESULT(
                                    0x80004005u32 as i32,
                                ))
                            })?;
                            if d.downloads.len() >= 100
                                && let Some(old) = d
                                    .downloads
                                    .iter()
                                    .find(|(_, v)| {
                                        !matches!(v.state.as_str(), "downloading" | "choosing")
                                    })
                                    .map(|(id, _)| id.clone())
                            {
                                d.downloads.remove(&old);
                            }
                            d.downloads.insert(
                                id.clone(),
                                Download {
                                    id: id.clone(),
                                    account_id: account.clone(),
                                    name: name.clone(),
                                    received: 0,
                                    total: None,
                                    state: "choosing".into(),
                                    saved_path: None,
                                },
                            );
                            d.config.clone()
                        };
                        desktop::emit(&app);
                        let a = app.clone();
                        tauri::async_runtime::spawn(async move {
                            desktop::overlay(&a, true).await;
                            let worker = a.clone();
                            let selected = tauri::async_runtime::spawn_blocking(move || {
                                destination(&worker, &name, &config)
                            })
                            .await
                            .unwrap_or_else(|_| Err("Save dialog failed".into()));
                            let main = a.clone();
                            let _ = a
                                .run_on_main_thread(move || complete_request(&main, &id, selected));
                            desktop::overlay(&a, false).await;
                        });
                        Ok(())
                    })),
                    &mut token,
                )?;
                Ok(())
            }
        })();
        let _ = tx.send(result.map_err(|_| "Native download management unavailable".to_string()));
    })
    .map_err(|_| "Cannot register native downloads")?;
    rx.recv_timeout(std::time::Duration::from_secs(5))
        .map_err(|_| "Download registration timed out")?
}
