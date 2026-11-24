//! Keep canonical paths for validation, but pass ordinary absolute paths to Chromium.
use std::path::{Path, PathBuf};

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
