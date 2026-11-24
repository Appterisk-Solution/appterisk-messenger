//! One remote top-level view. No application scripts, Vue, plugins, commands or capability grants.
//! Stock Tauri still installs its own internal initialization/IPC scripts; this is the test variable.
use appterisk_control_support::{BrowserWatch, Profile, append};
use serde_json::json;
use std::{
    cell::RefCell,
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};
use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};
thread_local! {static WATCH:RefCell<Option<BrowserWatch>>=const {RefCell::new(None)};}
static CLOSING: AtomicBool = AtomicBool::new(false);
static FINISHED: AtomicBool = AtomicBool::new(false);

pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let profile = Profile::acquire("minimal-tauri")?;
            let directory = profile.directory.clone();
            app.manage(profile);
            let page_app = app.handle().clone();
            let view = WebviewWindowBuilder::new(
                app,
                "tauri-control",
                WebviewUrl::External("https://web.whatsapp.com/".parse()?),
            )
            .title("Appterisk Minimal Tauri — baseline 0.1.2")
            .inner_size(1100.0, 800.0)
            .data_directory(directory)
            .incognito(false)
            .on_page_load(move |_, payload| {
                if matches!(payload.event(), tauri::webview::PageLoadEvent::Finished) {
                    append(
                        &page_app.state::<Profile>().log,
                        "navigation_completed",
                        json!({"authentication_verified":false}),
                    );
                    if std::env::args().any(|arg| arg == "--smoke") {
                        begin_close(&page_app);
                    }
                }
            })
            .build()?;
            let handle = app.handle().clone();
            view.with_webview(move |native| {
                let result = unsafe { native.controller().CoreWebView2() }.and_then(|core| {
                    BrowserWatch::attach(
                        &native.environment(),
                        &core,
                        &handle.state::<Profile>(),
                        Box::new(|| {}),
                    )
                });
                match result {
                    Ok(watch) => WATCH.with(|cell| *cell.borrow_mut() = Some(watch)),
                    Err(error) => {
                        append(
                            &handle.state::<Profile>().log,
                            "initialization_failed",
                            json!({"hresult":error.code().0}),
                        );
                        handle.exit(1);
                    }
                }
            })?;
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event
                && !FINISHED.load(Ordering::SeqCst)
            {
                api.prevent_close();
                begin_close(window.app_handle());
            }
        })
        .build(tauri::generate_context!())
        .expect("Could not initialize the minimal Tauri control")
        .run(|app, event| {
            if let tauri::RunEvent::ExitRequested { api, .. } = event
                && !FINISHED.load(Ordering::SeqCst)
            {
                api.prevent_exit();
                begin_close(app);
            }
        });
}

fn begin_close(app: &tauri::AppHandle) {
    if CLOSING.swap(true, Ordering::SeqCst) {
        return;
    }
    append(
        &app.state::<Profile>().log,
        "close_requested",
        json!({"forced":false}),
    );
    let Some(view) = app.get_webview_window("tauri-control") else {
        FINISHED.store(true, Ordering::SeqCst);
        app.exit(1);
        return;
    };
    let app = app.clone();
    let _ = view.with_webview(move |native| {
        let _ = unsafe { native.controller().Close() };
        let flags = WATCH.with(|cell| {
            cell.borrow()
                .as_ref()
                .map(|watch| (watch.exited.clone(), watch.normal.clone()))
        });
        tauri::async_runtime::spawn_blocking(move || {
            let started = std::time::Instant::now();
            if let Some((exited, normal)) = flags {
                while !exited.load(Ordering::SeqCst) {
                    if started.elapsed() > Duration::from_secs(15) {
                        append(
                            &app.state::<Profile>().log,
                            "shutdown_timeout",
                            json!({"forced":false}),
                        );
                        CLOSING.store(false, Ordering::SeqCst);
                        if let Some(view) = app.get_webview_window("tauri-control") {
                            let _ = view.set_title("Minimal Tauri: browser still closing; retry X");
                        }
                        return;
                    }
                    std::thread::sleep(Duration::from_millis(50));
                }
                append(
                    &app.state::<Profile>().log,
                    "shutdown_complete",
                    json!({"normal":normal.load(Ordering::SeqCst)}),
                );
            }
            FINISHED.store(true, Ordering::SeqCst);
            app.exit(0);
        });
    });
}
