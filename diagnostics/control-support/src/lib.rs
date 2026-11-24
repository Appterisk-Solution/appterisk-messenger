//! Independently authored, metadata-only support. No Tauri/Wry dependency and no JavaScript.
#![cfg(windows)]
use serde_json::{Value, json};
use std::{
    fs::{File, OpenOptions},
    io::{self, Write},
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{SystemTime, UNIX_EPOCH},
};
use webview2_com::{
    BrowserProcessExitedEventHandler, Microsoft::Web::WebView2::Win32::*, ProcessFailedEventHandler,
};
use windows::{
    Win32::System::Com::CoTaskMemFree,
    core::{BOOL, Interface, PWSTR},
};

pub struct Profile {
    pub directory: PathBuf,
    pub log: PathBuf,
    _exclusive: File,
}
impl Profile {
    pub fn acquire(tool: &str) -> io::Result<Self> {
        if !matches!(tool, "native-webview2" | "minimal-tauri") {
            return Err(io::Error::other("Unknown control"));
        }
        for name in [
            "WEBVIEW2_USER_DATA_FOLDER",
            "WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS",
            "WEBVIEW2_BROWSER_EXECUTABLE_FOLDER",
        ] {
            if std::env::var_os(name).is_some() {
                return Err(io::Error::other(format!(
                    "Remove {name} from the launching shell before the baseline test"
                )));
            }
        }
        let local = std::env::var_os("LOCALAPPDATA")
            .ok_or_else(|| io::Error::other("LOCALAPPDATA is unavailable"))?;
        let root = PathBuf::from(local)
            .join("com.appterisk.messenger")
            .join("controls")
            .join(tool);
        std::fs::create_dir_all(&root)?;
        let file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(root.join("instance.lock"))?;
        file.try_lock().map_err(|_| {
            io::Error::other("This control/profile is already open; close it normally first")
        })?;
        let directory = root.join("UserData");
        let existed = directory.is_dir();
        std::fs::create_dir_all(&directory)?;
        let directory = dunce::simplified(&directory.canonicalize()?).to_path_buf();
        let log = root.join("metadata.jsonl");
        append(
            &log,
            "startup",
            json!({"tool":tool,"version":env!("CARGO_PKG_VERSION"),"executable":std::env::current_exe().ok(),"user_data_directory":directory,"existed":existed,"application_javascript_injected":false,"tauri_internal_scripts":tool=="minimal-tauri","identity_override":false,"custom_browser_arguments":false,"profile_cleanup":false}),
        );
        Ok(Self {
            directory,
            log,
            _exclusive: file,
        })
    }
}

pub fn append(path: &std::path::Path, event: &str, fields: Value) {
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(
            file,
            "{}",
            json!({"unix_ms":SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis(),"pid":std::process::id(),"event":event,"fields":fields})
        );
    }
}

// Only non-sensitive engine metadata getters may use this helper.
unsafe fn text(
    read: impl FnOnce(*mut PWSTR) -> windows::core::Result<()>,
) -> windows::core::Result<String> {
    let mut raw = PWSTR::null();
    read(&mut raw)?;
    if raw.is_null() {
        return Err(windows::core::Error::from_hresult(windows::core::HRESULT(
            0x80004005u32 as i32,
        )));
    }
    let value = unsafe { raw.to_string() };
    unsafe { CoTaskMemFree(Some(raw.0.cast())) };
    value.map_err(|_| {
        windows::core::Error::from_hresult(windows::core::HRESULT(0x80004005u32 as i32))
    })
}

pub struct BrowserWatch {
    pub exited: Arc<AtomicBool>,
    pub normal: Arc<AtomicBool>,
    environment: ICoreWebView2Environment5,
    token: i64,
}
impl BrowserWatch {
    /// Call and retain on the COM/UI thread until the browser exits.
    pub fn attach(
        environment: &ICoreWebView2Environment,
        core: &ICoreWebView2,
        profile: &Profile,
        on_exit: Box<dyn Fn()>,
    ) -> windows::core::Result<Self> {
        unsafe {
            let folder = text(|out| {
                environment
                    .cast::<ICoreWebView2Environment7>()?
                    .UserDataFolder(out)
            })?;
            let runtime = text(|out| environment.BrowserVersionString(out))?;
            let ua = text(|out| {
                core.Settings()?
                    .cast::<ICoreWebView2Settings2>()?
                    .UserAgent(out)
            })?;
            let native_profile = core.cast::<ICoreWebView2_13>()?.Profile()?;
            let mut private = BOOL(0);
            native_profile.IsInPrivateModeEnabled(&mut private)?;
            let native_path = text(|out| native_profile.ProfilePath(out))?;
            let matches =
                PathBuf::from(&folder).canonicalize().ok() == profile.directory.canonicalize().ok();
            append(
                &profile.log,
                "engine_metadata",
                json!({"runtime":runtime,"user_agent":ua,"effective_directory":folder,"profile_path":native_path,"directory_matches":matches,"in_private":private.as_bool()}),
            );
            if private.as_bool() || !matches {
                return Err(windows::core::Error::from_hresult(windows::core::HRESULT(
                    0x80004005u32 as i32,
                )));
            }
            let exited = Arc::new(AtomicBool::new(false));
            let normal = Arc::new(AtomicBool::new(false));
            let exited_event = exited.clone();
            let normal_event = normal.clone();
            let log = profile.log.clone();
            let handler = BrowserProcessExitedEventHandler::create(Box::new(move |_, args| {
                let mut kind = COREWEBVIEW2_BROWSER_PROCESS_EXIT_KIND_FAILED;
                if let Some(args) = args {
                    args.BrowserProcessExitKind(&mut kind)?;
                }
                normal_event.store(
                    kind == COREWEBVIEW2_BROWSER_PROCESS_EXIT_KIND_NORMAL,
                    Ordering::SeqCst,
                );
                exited_event.store(true, Ordering::SeqCst);
                append(&log, "browser_exited", json!({"kind":kind.0}));
                on_exit();
                Ok(())
            }));
            let environment = environment.cast::<ICoreWebView2Environment5>()?;
            let mut token = 0;
            environment.add_BrowserProcessExited(&handler, &mut token)?;
            let log = profile.log.clone();
            let mut process_token = 0;
            core.add_ProcessFailed(
                &ProcessFailedEventHandler::create(Box::new(move |_, args| {
                    let mut kind = COREWEBVIEW2_PROCESS_FAILED_KIND_BROWSER_PROCESS_EXITED;
                    if let Some(args) = args {
                        args.ProcessFailedKind(&mut kind)?;
                    }
                    append(&log, "process_failed", json!({"kind":kind.0}));
                    Ok(())
                })),
                &mut process_token,
            )?;
            Ok(Self {
                exited,
                normal,
                environment,
                token,
            })
        }
    }
}
impl Drop for BrowserWatch {
    fn drop(&mut self) {
        let _ = unsafe { self.environment.remove_BrowserProcessExited(self.token) };
    }
}
