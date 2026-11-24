//! Use documented WebView2 metadata APIs; never access cookie values or provider storage contents.
use crate::health;
use serde_json::json;
use std::{cell::RefCell, collections::HashMap};
use std::{
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};
use tauri::{Emitter, Webview};
use webview2_com::{
    AcceleratorKeyPressedEventHandler, BrowserProcessExitedEventHandler,
    GetCookiesCompletedHandler, Microsoft::Web::WebView2::Win32::*,
    NavigationCompletedEventHandler, ProcessFailedEventHandler,
};
use windows::{
    Win32::System::Com::CoTaskMemFree,
    core::{BOOL, HSTRING, Interface, PWSTR},
};

#[derive(Clone)]
pub struct BrowserLifetime {
    pub exited: Arc<AtomicBool>,
}

pub fn runtime_version() -> Option<String> {
    unsafe {
        string_property(|out| {
            GetAvailableCoreWebView2BrowserVersionString(windows::core::PCWSTR::null(), out)
        })
        .ok()
    }
}

/// Application shortcuts continue working while the native provider owns keyboard focus.
pub fn account_events(app: &tauri::AppHandle, view: &Webview, id: &str) -> Result<(), String> {
    let app = app.clone();
    let id = id.to_owned();
    let (tx, rx) = std::sync::mpsc::channel();
    view.with_webview(move|native|{
        let result=(||->windows::core::Result<()>{unsafe{
            let controller=native.controller();let mut token=0;let shortcut_app=app.clone();
            controller.add_AcceleratorKeyPressed(&AcceleratorKeyPressedEventHandler::create(Box::new(move|_,args|{
                if let Some(args)=args {
                    let mut kind=COREWEBVIEW2_KEY_EVENT_KIND_KEY_UP;args.KeyEventKind(&mut kind)?;
                    if kind==COREWEBVIEW2_KEY_EVENT_KIND_KEY_DOWN && windows::Win32::UI::Input::KeyboardAndMouse::GetKeyState(windows::Win32::UI::Input::KeyboardAndMouse::VK_CONTROL.0 as i32)<0{
                        let mut code=0;args.VirtualKey(&mut code)?;
                        let key=match code{49..=57=>char::from_u32(code).map(|c|c.to_string()),82=>Some("r".into()),188=>Some(",".into()),_=>None};
                        if let Some(key)=key{args.SetHandled(true)?;let _=shortcut_app.emit_to("main","account-shortcut",json!({"key":key}));}
                    }
                }Ok(())
            })),&mut token)?;
            controller.CoreWebView2()?.add_NavigationCompleted(&NavigationCompletedEventHandler::create(Box::new(move|_,args|{
                if let Some(args)=args{let mut success=BOOL(0);args.IsSuccess(&mut success)?;if !success.as_bool(){let _=app.emit_to("main","account-error",json!({"id":id,"message":"WhatsApp could not finish loading. Check your connection and reload this account."}));}}
                Ok(())
            })),&mut token)?;Ok(())
        }})();let _=tx.send(result.map_err(|_|"Cannot register native account events".to_string()));
    }).map_err(|_|"Cannot schedule native account events")?;
    rx.recv_timeout(std::time::Duration::from_secs(5))
        .map_err(|_| "Native event registration timed out")?
}

// Keep environment COM objects alive on their UI thread until BrowserProcessExited.
thread_local! {
    static EXIT_WATCHERS:RefCell<HashMap<String,(ICoreWebView2Environment5,i64)>>=RefCell::new(HashMap::new());
}

pub fn close_controller(view: &Webview) -> Result<(), String> {
    let (sender, receiver) = std::sync::mpsc::channel();
    view.with_webview(move |native| {
        let result = unsafe { native.controller().Close() }
            .map_err(|_| "Native controller close failed".to_string());
        let _ = sender.send(result);
    })
    .map_err(|_| "Could not schedule native controller close")?;
    receiver
        .recv_timeout(std::time::Duration::from_secs(5))
        .map_err(|_| "Native controller close timed out")?
}

// COM getters allocate with CoTaskMemAlloc. Only call this for non-sensitive metadata getters.
unsafe fn string_property(
    read: impl FnOnce(*mut PWSTR) -> windows::core::Result<()>,
) -> windows::core::Result<String> {
    let mut raw = PWSTR::null();
    read(&mut raw)?;
    let result = unsafe { raw.to_string() };
    unsafe { CoTaskMemFree(Some(raw.0.cast())) };
    result.map_err(|_| {
        windows::core::Error::from_hresult(windows::core::HRESULT(0x80004005u32 as i32))
    })
}

pub fn attach(
    app: &tauri::AppHandle,
    view: &Webview,
    id: &str,
    expected: &Path,
) -> Result<BrowserLifetime, String> {
    let exited = Arc::new(AtomicBool::new(false));
    let observed = exited.clone();
    let (sender, receiver) = std::sync::mpsc::channel();
    let app = app.clone();
    let id = id.to_string();
    let expected = expected.to_path_buf();
    view.with_webview(move |native| {
        // All COM calls run on the WebView thread, and only documented metadata is read.
        let result = (|| -> windows::core::Result<()> { unsafe {
            let environment = native.environment();
            let core = native.controller().CoreWebView2()?;
            let effective = string_property(|out| environment.cast::<ICoreWebView2Environment7>()?.UserDataFolder(out))?;
            let version = string_property(|out| environment.BrowserVersionString(out))?;
            let profile = core.cast::<ICoreWebView2_13>()?.Profile()?;
            let profile_path = string_property(|out| profile.ProfilePath(out))?;
            let mut private = BOOL(0);
            profile.IsInPrivateModeEnabled(&mut private)?;
            let user_agent = string_property(|out| core.Settings()?.cast::<ICoreWebView2Settings2>()?.UserAgent(out))?;
            let actual = PathBuf::from(&effective).canonicalize().ok();
            let matches = actual.as_deref() == Some(expected.as_path());
            health::log(&app,"effective_profile",json!({"profile_id":id,"expected_directory":expected,"effective_directory":effective,"profile_path":profile_path,"directory_matches":matches,"in_private":private.as_bool(),"runtime_version":version,"user_agent":user_agent}));
            if !matches || private.as_bool() { return Err(windows::core::Error::from_hresult(windows::core::HRESULT(0x80004005u32 as i32))); }
            let app_exit = app.clone();
            let id_exit = id.clone();
            let handler = BrowserProcessExitedEventHandler::create(Box::new(move |_, args| {
                let mut kind = COREWEBVIEW2_BROWSER_PROCESS_EXIT_KIND_NORMAL;
                if let Some(args) = args { let _ = args.BrowserProcessExitKind(&mut kind); }
                observed.store(true, Ordering::SeqCst);
                health::log(&app_exit,"browser_process_exited",json!({"profile_id":id_exit,"exit_kind":kind.0}));
                EXIT_WATCHERS.with(|watchers| {
                    if let Some((environment,token))=watchers.borrow_mut().remove(&id_exit) {
                        let _=environment.remove_BrowserProcessExited(token);
                    }
                });
                Ok(())
            }));
            let mut token = 0;
            let environment5=environment.cast::<ICoreWebView2Environment5>()?;
            environment5.add_BrowserProcessExited(&handler,&mut token)?;
            EXIT_WATCHERS.with(|watchers| {watchers.borrow_mut().insert(id.clone(),(environment5,token));});
            let app_failed = app.clone();
            let id_failed = id.clone();
            core.add_ProcessFailed(&ProcessFailedEventHandler::create(Box::new(move |_,args| {
                let mut kind = COREWEBVIEW2_PROCESS_FAILED_KIND_BROWSER_PROCESS_EXITED;
                if let Some(args) = args { let _ = args.ProcessFailedKind(&mut kind); }
                health::log(&app_failed,"browser_process_failed",json!({"profile_id":id_failed,"failure_kind":kind.0}));
                Ok(())
            })),&mut token)?;
            Ok(())
        }})();
        let _ = sender.send(result.map_err(|_| "Effective profile verification or browser event subscription failed".to_string()));
    }).map_err(|_| "Cannot inspect native profile")?;
    receiver
        .recv_timeout(std::time::Duration::from_secs(10))
        .map_err(|_| "Native profile verification timed out")??;
    Ok(BrowserLifetime { exited })
}

pub fn cookie_count(app: &tauri::AppHandle, view: &Webview, id: &str) {
    let app = app.clone();
    let id = id.to_string();
    let _ = view.with_webview(move |native| unsafe {
        let result = (|| -> windows::core::Result<()> {
            let manager = native.controller().CoreWebView2()?.cast::<ICoreWebView2_2>()?.CookieManager()?;
            manager.GetCookies(&HSTRING::from("https://web.whatsapp.com/"), &GetCookiesCompletedHandler::create(Box::new(move |status, list| {
                let count = status.and_then(|()| { let mut count=0; if let Some(list)=list { list.Count(&mut count)?; } Ok(count) });
                health::log(&app,"origin_cookie_metadata",json!({"profile_id":id,"origin":"https://web.whatsapp.com","cookie_count":count.ok()}));
                // Deliberately never call GetValueAtIndex, Name, Value, or other cookie getters.
                Ok(())
            })))
        })();
        let _ = result;
    });
}
