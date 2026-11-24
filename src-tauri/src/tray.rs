use crate::{
    accounts,
    desktop::{self, DesktopState},
    desktop_policy,
};
use tauri::{
    Manager,
    menu::{CheckMenuItem, Menu, MenuItem, Submenu},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
};
fn text<'a>(lang: &str, en: &'a str, cn: &'a str, tw: &'a str) -> &'a str {
    match lang {
        "zh" | "zh-CN" => cn,
        "zh-TW" => tw,
        _ => en,
    }
}
fn menu(app: &tauri::AppHandle) -> tauri::Result<Menu<tauri::Wry>> {
    let state = app.state::<DesktopState>();
    let d = state.0.lock().expect("desktop state poisoned");
    let lang = &d.language;
    let open = MenuItem::with_id(
        app,
        "open",
        text(
            lang,
            "Open Appterisk Messenger",
            "打开 Appterisk Messenger",
            "開啟 Appterisk Messenger",
        ),
        true,
        None::<&str>,
    )?;
    let accounts = Submenu::new(
        app,
        text(lang, "Select account", "选择账号", "選擇帳號"),
        true,
    )?;
    let mut list: Vec<_> = d.accounts.values().collect();
    list.sort_by_key(|a| a.display_order);
    for a in list {
        accounts.append(&MenuItem::with_id(
            app,
            format!("account:{}", a.id),
            &a.name,
            true,
            None::<&str>,
        )?)?;
    }
    let enabled = CheckMenuItem::with_id(
        app,
        "notifications",
        text(lang, "Notifications", "通知", "通知"),
        !d.lock.locked,
        d.config.notifications,
        None::<&str>,
    )?;
    let dnd = CheckMenuItem::with_id(
        app,
        "dnd",
        text(lang, "Do Not Disturb", "勿扰模式", "勿擾模式"),
        !d.lock.locked,
        d.config.dnd,
        None::<&str>,
    )?;
    let settings = MenuItem::with_id(
        app,
        "settings",
        text(lang, "Settings", "设置", "設定"),
        true,
        None::<&str>,
    )?;
    let lock = MenuItem::with_id(
        app,
        "lock",
        text(lang, "Lock application", "锁定应用", "鎖定應用程式"),
        d.lock.hash.is_some(),
        None::<&str>,
    )?;
    let quit = MenuItem::with_id(
        app,
        "quit",
        text(lang, "Quit", "退出", "結束"),
        true,
        None::<&str>,
    )?;
    Menu::with_items(
        app,
        &[&open, &accounts, &enabled, &dnd, &settings, &lock, &quit],
    )
}
pub fn setup(app: &tauri::AppHandle) -> Result<(), String> {
    let icon = app
        .default_window_icon()
        .cloned()
        .ok_or("Application tray icon missing")?;
    TrayIconBuilder::with_id("appterisk")
        .icon(icon)
        .tooltip("Appterisk Messenger")
        .menu(&menu(app).map_err(|_| "Tray menu initialization failed")?)
        .show_menu_on_left_click(false)
        .on_tray_icon_event(|tray, event| {
            if matches!(
                event,
                TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                }
            ) {
                desktop::restore(tray.app_handle(), false);
            }
        })
        .on_menu_event(|app, event| {
            let id = event.id().as_ref();
            if let Some(id) = id.strip_prefix("account:") {
                desktop::activate(app, id, None);
                return;
            }
            match id {
                "open" => desktop::restore(app, false),
                "settings" => desktop::restore(app, true),
                "quit" => crate::probe::request_shutdown(app, 0),
                "lock" => {
                    let app = app.clone();
                    tauri::async_runtime::spawn(async move {
                        let _ = desktop::lock_now(&app).await;
                    });
                }
                "notifications" | "dnd" => {
                    let config = {
                        let s = app.state::<DesktopState>();
                        s.0.lock().ok().map(|d| {
                            let mut c = d.config.clone();
                            if id == "notifications" {
                                c.notifications = !c.notifications;
                            } else {
                                c.dnd = !c.dnd;
                            }
                            c
                        })
                    };
                    if let Some(config) = config {
                        let app = app.clone();
                        tauri::async_runtime::spawn(async move {
                            let _ = desktop::save_config(&app, config).await;
                        });
                    }
                }
                _ => {}
            }
        })
        .build(app)
        .map_err(|_| "System tray initialization failed")?;
    Ok(())
}
pub fn refresh(app: &tauri::AppHandle) {
    if accounts::synthetic_mode() {
        return;
    }
    let app = app.clone();
    let dispatcher = app.clone();
    let _ = dispatcher.run_on_main_thread(move || {
        if let Some(tray) = app.tray_by_id("appterisk") {
            if let Ok(menu) = menu(&app) {
                let _ = tray.set_menu(Some(menu));
            }
            let totals = {
                let s = app.state::<DesktopState>();
                s.0.lock()
                    .ok()
                    .map(|d| desktop_policy::aggregate(&d.unread))
            };
            if let Some((count, unknown)) = totals {
                let tooltip = if count > 0 || unknown {
                    format!(
                        "Appterisk Messenger — {count}{} unread (provider title)",
                        if unknown { "+" } else { "" }
                    )
                } else {
                    "Appterisk Messenger".into()
                };
                let _ = tray.set_tooltip(Some(tooltip));
                if let Some(icon) = app.default_window_icon() {
                    let mut pixels = icon.rgba().to_vec();
                    let (w, h) = (icon.width(), icon.height());
                    if count > 0 || unknown {
                        let radius = (w.min(h) / 5).max(2) as i64;
                        let cx = i64::from(w) - radius - 1;
                        let cy = radius + 1;
                        for y in 0..h {
                            for x in 0..w {
                                if (i64::from(x) - cx).pow(2) + (i64::from(y) - cy).pow(2)
                                    <= radius.pow(2)
                                {
                                    let n = ((y * w + x) * 4) as usize;
                                    pixels[n..n + 4].copy_from_slice(&[224, 91, 91, 255]);
                                }
                            }
                        }
                    }
                    let _ = tray.set_icon(Some(tauri::image::Image::new_owned(pixels, w, h)));
                }
            }
        }
    });
}
pub fn close_to_tray(app: &tauri::AppHandle) -> bool {
    app.try_state::<DesktopState>()
        .and_then(|s| {
            s.0.lock().ok().map(|d| {
                desktop_policy::close_action(&d.config, false, crate::health::closing())
                    == desktop_policy::CloseAction::Hide
            })
        })
        .unwrap_or(false)
}
pub fn minimized(app: &tauri::AppHandle) {
    let hide = app
        .try_state::<DesktopState>()
        .and_then(|s| s.0.lock().ok().map(|d| d.config.minimize_to_tray))
        .unwrap_or(false);
    if hide && let Some(w) = app.get_window("main") {
        let _ = w.hide();
    }
}
