//! Keep canonical paths for validation, but pass ordinary absolute paths to Chromium.
use std::path::{Path, PathBuf};

/// Keep the same metadata/profile root when Windows package identity is added.
/// The manifest separately disables AppData write virtualization.
pub fn app_data_root(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    #[cfg(all(windows, feature = "store-distribution"))]
    {
        use windows::Win32::{
            System::Com::CoTaskMemFree,
            UI::Shell::{
                FOLDERID_LocalAppData, KF_FLAG_NO_PACKAGE_REDIRECTION, SHGetKnownFolderPath,
            },
        };
        let raw = unsafe {
            SHGetKnownFolderPath(&FOLDERID_LocalAppData, KF_FLAG_NO_PACKAGE_REDIRECTION, None)
        }
        .map_err(|_| "Cannot resolve existing application data location")?;
        let value = unsafe { raw.to_string() };
        unsafe { CoTaskMemFree(Some(raw.0.cast())) };
        let root = PathBuf::from(value.map_err(|_| "Invalid application data location")?);
        if !root.is_absolute() {
            return Err("Application data location must be absolute".into());
        }
        Ok(root.join(&app.config().identifier))
    }
    #[cfg(not(all(windows, feature = "store-distribution")))]
    {
        use tauri::Manager;
        app.path()
            .app_local_data_dir()
            .map_err(|_| "Cannot resolve application data location".into())
    }
}

pub fn browser_directory(canonical: &Path) -> Result<PathBuf, &'static str> {
    if !canonical.is_absolute() {
        return Err("Browser profile path must be absolute");
    }
    let normal = dunce::simplified(canonical);
    #[cfg(windows)]
    if normal
        .as_os_str()
        .as_encoded_bytes()
        .starts_with(b"\\\\?\\")
    {
        return Err(
            "Profile path cannot be represented as an ordinary Windows path; existing data is unchanged",
        );
    }
    Ok(normal.to_path_buf())
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    #[test]
    fn normalizes_verbatim_drive_without_changing_components() {
        assert_eq!(
            browser_directory(Path::new(r"\\?\C:\Users\Fixture\UserData")).unwrap(),
            PathBuf::from(r"C:\Users\Fixture\UserData")
        );
        assert_eq!(
            browser_directory(Path::new(r"C:\Users\Fixture\UserData")).unwrap(),
            PathBuf::from(r"C:\Users\Fixture\UserData")
        );
    }
    #[test]
    fn rejects_unrepresentable_verbatim_paths_and_relative_paths() {
        assert!(browser_directory(Path::new(r"\\?\UNC\server\share\UserData")).is_err());
        assert!(browser_directory(Path::new("UserData")).is_err());
    }
}
