//! Original native HWND + COM baseline. No Tauri, Wry, scripts, IPC bridge or identity changes.
use appterisk_control_support::{BrowserWatch, Profile, append};
use serde_json::json;
use std::{cell::RefCell, sync::atomic::Ordering};
use webview2_com::{
    CreateCoreWebView2ControllerCompletedHandler, CreateCoreWebView2EnvironmentCompletedHandler,
    Microsoft::Web::WebView2::Win32::*, NavigationCompletedEventHandler,
};
use windows::{
    Win32::{
        Foundation::*,
        Graphics::Gdi::*,
        System::{Com::*, LibraryLoader::GetModuleHandleW},
        UI::WindowsAndMessaging::*,
    },
    core::{BOOL, HSTRING, PCWSTR, w},
};

const BROWSER_EXIT: u32 = WM_APP + 1;
const CLOSE_TIMEOUT: usize = 1;
const SMOKE_CLOSE: usize = 2;
struct State {
    profile: Profile,
    controller: Option<ICoreWebView2Controller>,
    watch: Option<BrowserWatch>,
    closing: bool,
}
thread_local! {static STATE:RefCell<Option<State>>=const {RefCell::new(None)};}

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let profile = Profile::acquire("native-webview2")?;
    unsafe {
        CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok()?;
        let instance = HINSTANCE(GetModuleHandleW(None)?.0);
        let class = w!("AppteriskNativeWebView2Control");
        let wc = WNDCLASSW {
            hInstance: instance,
            lpszClassName: class,
            lpfnWndProc: Some(window_proc),
            hCursor: LoadCursorW(None, IDC_ARROW)?,
            hbrBackground: GetSysColorBrush(COLOR_WINDOW),
            ..Default::default()
        };
        if RegisterClassW(&wc) == 0 {
            return Err(windows::core::Error::from_thread().into());
        }
        let hwnd = CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            class,
            w!("Appterisk Native WebView2 — baseline 0.1.2"),
            WS_OVERLAPPEDWINDOW,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            1100,
            800,
            None,
            None,
            Some(instance),
            None,
        )?;
        STATE.with(|state| {
            *state.borrow_mut() = Some(State {
                profile,
                controller: None,
                watch: None,
                closing: false,
            })
        });
        let _ = ShowWindow(hwnd, SW_SHOW);
        initialize(hwnd)?;
        let mut message = MSG::default();
        loop {
            let status = GetMessageW(&mut message, None, 0, 0).0;
            if status == 0 {
                break;
            }
            if status < 0 {
                return Err(windows::core::Error::from_thread().into());
            }
            let _ = TranslateMessage(&message);
            DispatchMessageW(&message);
        }
        STATE.with(|state| {
            state.borrow_mut().take();
        });
        CoUninitialize();
    }
    Ok(())
}

fn initialize(hwnd: HWND) -> windows::core::Result<()> {
    let path = STATE.with(|state| state.borrow().as_ref().unwrap().profile.directory.clone());
    let window_id = hwnd.0 as isize;
    let callback = CreateCoreWebView2EnvironmentCompletedHandler::create(Box::new(
        move |status, environment| {
            status?;
            let environment = environment.ok_or_else(windows::core::Error::from_thread)?;
            let native_environment = environment.clone();
            let controller_callback = CreateCoreWebView2ControllerCompletedHandler::create(
                Box::new(move |status, controller| {
                    status?;
                    let controller = controller.ok_or_else(windows::core::Error::from_thread)?;
                    let hwnd = HWND(window_id as *mut _);
                    unsafe {
                        let core = controller.CoreWebView2()?;
                        let watch = STATE.with(|state| {
                            let state = state.borrow();
                            let profile = &state.as_ref().unwrap().profile;
                            BrowserWatch::attach(
                                &native_environment,
                                &core,
                                profile,
                                Box::new(move || {
                                    let _ = PostMessageW(
                                        Some(HWND(window_id as *mut _)),
                                        BROWSER_EXIT,
                                        WPARAM(0),
                                        LPARAM(0),
                                    );
                                }),
                            )
                        })?;
                        STATE.with(|state| {
                            let mut state = state.borrow_mut();
                            let state = state.as_mut().unwrap();
                            state.controller = Some(controller.clone());
                            state.watch = Some(watch);
                        });
                        resize(hwnd);
                        controller.SetIsVisible(true)?;
                        let smoke = std::env::args().any(|arg| arg == "--smoke");
                        let mut token = 0;
                        core.add_NavigationCompleted(&NavigationCompletedEventHandler::create(Box::new(move|_,args|{
                    let mut success=BOOL(0);
                    if let Some(args)=args {args.IsSuccess(&mut success)?;}
                    STATE.with(|state|{if let Some(state)=state.borrow().as_ref(){append(&state.profile.log,"navigation_completed",json!({"success":success.as_bool(),"authentication_verified":false}));}});
                    if smoke {let _=SetTimer(Some(HWND(window_id as *mut _)),SMOKE_CLOSE,1000,None);}
                    Ok(())
                })),&mut token)?;
                        core.Navigate(&HSTRING::from("https://web.whatsapp.com/"))?;
                    }
                    Ok(())
                }),
            );
            unsafe {
                environment
                    .CreateCoreWebView2Controller(HWND(window_id as *mut _), &controller_callback)
            }
        },
    ));
    // Null environment options = browser defaults, including no extra Chromium arguments.
    unsafe {
        CreateCoreWebView2EnvironmentWithOptions(
            PCWSTR::null(),
            &HSTRING::from(path.to_string_lossy().as_ref()),
            None::<&ICoreWebView2EnvironmentOptions>,
            &callback,
        )
    }
}

fn resize(hwnd: HWND) {
    let controller = STATE.with(|state| {
        state
            .borrow()
            .as_ref()
            .and_then(|state| state.controller.clone())
    });
    if let Some(controller) = controller {
        unsafe {
            let mut rect = RECT::default();
            if GetClientRect(hwnd, &mut rect).is_ok() {
                let _ = controller.SetBounds(rect);
            }
        }
    }
}
fn begin_close(hwnd: HWND) {
    let controller = STATE.with(|state| {
        let mut state = state.borrow_mut();
        let state = state.as_mut()?;
        if state.closing {
            return None;
        }
        state.closing = true;
        append(
            &state.profile.log,
            "close_requested",
            json!({"forced":false}),
        );
        state.controller.take()
    });
    if let Some(controller) = controller {
        let _ = unsafe { controller.Close() };
    }
    let done = STATE.with(|state| {
        state
            .borrow()
            .as_ref()
            .map(|state| {
                state
                    .watch
                    .as_ref()
                    .is_none_or(|watch| watch.exited.load(Ordering::SeqCst))
            })
            .unwrap_or(true)
    });
    unsafe {
        if done {
            let _ = DestroyWindow(hwnd);
        } else {
            let _ = SetTimer(Some(hwnd), CLOSE_TIMEOUT, 15000, None);
        }
    }
}
unsafe extern "system" fn window_proc(hwnd: HWND, message: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    match message {
        WM_SIZE => {
            resize(hwnd);
            LRESULT(0)
        }
        WM_CLOSE => {
            begin_close(hwnd);
            LRESULT(0)
        }
        BROWSER_EXIT => {
            let closing =
                STATE.with(|state| state.borrow().as_ref().is_some_and(|state| state.closing));
            if closing {
                let _ = unsafe { DestroyWindow(hwnd) };
            }
            LRESULT(0)
        }
        WM_TIMER if w.0 == SMOKE_CLOSE => {
            let _ = unsafe { KillTimer(Some(hwnd), SMOKE_CLOSE) };
            begin_close(hwnd);
            LRESULT(0)
        }
        WM_TIMER if w.0 == CLOSE_TIMEOUT => {
            let _ = unsafe { KillTimer(Some(hwnd), CLOSE_TIMEOUT) };
            STATE.with(|state| {
                if let Some(state) = state.borrow_mut().as_mut() {
                    state.closing = false;
                    append(
                        &state.profile.log,
                        "shutdown_timeout",
                        json!({"forced":false}),
                    );
                }
            });
            let _ = unsafe {
                SetWindowTextW(
                    hwnd,
                    w!("Native control: browser is still closing; retry X"),
                )
            };
            LRESULT(0)
        }
        WM_DESTROY => {
            unsafe { PostQuitMessage(0) };
            LRESULT(0)
        }
        _ => unsafe { DefWindowProcW(hwnd, message, w, l) },
    }
}
