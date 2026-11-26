//! Pin the official Telegram client per profile; never migrate existing browser storage.
use std::{fs, path::Path};

const MARKER: &str = "appterisk-telegram-client.txt";
const WEB_A: &str = "https://web.telegram.org/a/";
const WEB_K: &str = "https://web.telegram.org/k/";

pub fn entry_url(profile: &Path) -> Result<&'static str, String> {
    let marker = profile.join(MARKER);
    match fs::read_to_string(&marker) {
        Ok(client) => {
            return match client.as_str() {
                "a" => Ok(WEB_A),
                "k" => Ok(WEB_K),
                _ => Err("Telegram client preference is invalid; profile unchanged.".into()),
            };
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(_) => return Err("Cannot read Telegram client preference.".into()),
    }
    // Pre-fix profiles used Web K. Keep their client even if they are logged out.
    // Only check directory metadata; never open browser databases or credentials.
    let legacy = profile.join("EBWebView").exists()
        || profile.join("Default").exists()
        || profile.join("Local State").exists();
    let client = if legacy { "k" } else { "a" };
    use std::io::Write;
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(marker)
        .map_err(|_| "Cannot save Telegram client preference.")?;
    file.write_all(client.as_bytes())
        .and_then(|()| file.sync_all())
        .map_err(|_| "Cannot save Telegram client preference.")?;
    Ok(if legacy { WEB_K } else { WEB_A })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_is_pinned_without_migrating_existing_profiles() {
        let root =
            std::env::temp_dir().join(format!("appterisk-client-test-{}", uuid::Uuid::new_v4()));
        let fresh = root.join("fresh");
        let legacy = root.join("legacy");
        fs::create_dir_all(&fresh).unwrap();
        fs::create_dir_all(legacy.join("EBWebView")).unwrap();
        assert_eq!(entry_url(&fresh).unwrap(), WEB_A);
        fs::create_dir(fresh.join("EBWebView")).unwrap();
        assert_eq!(entry_url(&fresh).unwrap(), WEB_A);
        assert_eq!(entry_url(&legacy).unwrap(), WEB_K);
        assert_eq!(entry_url(&legacy).unwrap(), WEB_K);
        fs::write(fresh.join(MARKER), "https://unexpected.invalid/").unwrap();
        assert!(entry_url(&fresh).is_err());
        assert!(legacy.join("EBWebView").is_dir());
        fs::remove_dir_all(root).unwrap();
    }
}
