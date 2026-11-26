//! Supported Windows per-user registration. Startup is changed only by explicit settings.
use winreg::{RegKey, enums::HKEY_CURRENT_USER};
const APP_ID: &str = "com.appterisk.messenger";
pub fn register(_: &tauri::AppHandle) -> Result<(), String> {
    // MSIX supplies the protocol and notification identity through its manifest.
    // Never rewrite the standalone installation's per-user registration.
    if cfg!(feature = "store-distribution") {
        return Ok(());
    }
    let exe = std::env::current_exe().map_err(|_| "Cannot locate application")?;
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let (key, _) = hkcu
        .create_subkey(format!("Software\\Classes\\AppUserModelId\\{APP_ID}"))
        .map_err(|_| "Notification registration failed")?;
    key.set_value("DisplayName", &"Appterisk Messenger")
        .map_err(|_| "Notification display name registration failed")?;
    let (protocol, _) = hkcu
        .create_subkey("Software\\Classes\\appterisk")
        .map_err(|_| "Notification activation registration failed")?;
    protocol
        .set_value("", &"URL:Appterisk notification activation")
        .map_err(|_| "Protocol registration failed")?;
    protocol
        .set_value("URL Protocol", &"")
        .map_err(|_| "Protocol registration failed")?;
    let (command, _) = protocol
        .create_subkey("shell\\open\\command")
        .map_err(|_| "Protocol registration failed")?;
    command
        .set_value("", &format!("\"{}\" \"%1\"", exe.display()))
        .map_err(|_| "Protocol registration failed")?;
    Ok(())
}
pub fn startup(_: &tauri::AppHandle, enabled: bool) -> Result<(), String> {
    if cfg!(feature = "store-distribution") {
        return Err("Startup changes are unavailable in this Store build.".into());
    }
    let key = RegKey::predef(HKEY_CURRENT_USER)
        .create_subkey("Software\\Microsoft\\Windows\\CurrentVersion\\Run")
        .map_err(|_| "Cannot update Windows startup preference")?
        .0;
    if enabled {
        let exe = std::env::current_exe().map_err(|_| "Cannot locate application")?;
        key.set_value(
            "Appterisk Messenger",
            &format!("\"{}\" --autostart", exe.display()),
        )
        .map_err(|_| "Cannot enable Windows startup")?;
    } else if let Err(e) = key.delete_value("Appterisk Messenger")
        && e.kind() != std::io::ErrorKind::NotFound
    {
        return Err("Cannot disable Windows startup".into());
    }
    Ok(())
}
pub fn activity() -> Option<u32> {
    unsafe {
        use windows::Win32::{
            System::Threading::GetCurrentProcessId,
            UI::{
                Input::KeyboardAndMouse::{GetLastInputInfo, LASTINPUTINFO},
                WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId},
            },
        };
        let mut pid = 0;
        GetWindowThreadProcessId(GetForegroundWindow(), Some(&mut pid));
        if pid != GetCurrentProcessId() {
            return None;
        }
        let mut info = LASTINPUTINFO {
            cbSize: std::mem::size_of::<LASTINPUTINFO>() as u32,
            dwTime: 0,
        };
        if !GetLastInputInfo(&mut info).as_bool() {
            return None;
        }
        Some(info.dwTime)
    }
}
pub fn notifier() -> windows::core::Result<windows::UI::Notifications::ToastNotifier> {
    if cfg!(feature = "store-distribution") {
        return windows::UI::Notifications::ToastNotificationManager::CreateToastNotifier();
    }
    windows::UI::Notifications::ToastNotificationManager::CreateToastNotifierWithId(
        &windows::core::HSTRING::from(APP_ID),
    )
}
