//! Pure desktop policy. No provider messages or credentials are persisted here.
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
};

pub const ORIGIN: &str = "https://web.whatsapp.com";
pub const PERMISSIONS: [&str; 5] = [
    "microphone",
    "camera",
    "notifications",
    "clipboard",
    "autoplay",
];
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct DesktopConfig {
    pub notifications: bool,
    pub dnd: bool,
    pub sound: bool,
    pub hide_previews: bool,
    pub launch_on_startup: bool,
    pub start_minimized: bool,
    pub minimize_to_tray: bool,
    pub close_to_tray: bool,
    pub download_directory: Option<String>,
    pub ask_before_saving: bool,
    pub lock_timeout_seconds: u32,
}
impl Default for DesktopConfig {
    fn default() -> Self {
        Self {
            notifications: true,
            dnd: false,
            sound: true,
            hide_previews: true,
            launch_on_startup: false,
            start_minimized: false,
            minimize_to_tray: false,
            close_to_tray: false,
            download_directory: None,
            ask_before_saving: true,
            lock_timeout_seconds: 300,
        }
    }
}
impl DesktopConfig {
    pub fn validate(&self) -> Result<(), String> {
        if self.lock_timeout_seconds > 86400 {
            return Err("Invalid automatic lock timeout.".into());
        }
        if let Some(path) = &self.download_directory {
            let p = Path::new(path);
            if path.len() > 4096 || !p.is_absolute() || !p.is_dir() {
                return Err("Choose an existing absolute download directory.".into());
            }
        }
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Permission {
    pub account_id: String,
    pub origin: String,
    pub kind: String,
    pub decision: String,
}
pub fn valid_permission(origin: &str, kind: &str, decision: &str) -> bool {
    origin == ORIGIN && PERMISSIONS.contains(&kind) && ["ask", "allow", "deny"].contains(&decision)
}
#[derive(Debug, PartialEq)]
pub enum CloseAction {
    Hide,
    Exit,
}
pub fn close_action(
    config: &DesktopConfig,
    explicit_quit: bool,
    shutting_down: bool,
) -> CloseAction {
    if config.close_to_tray && !explicit_quit && !shutting_down {
        CloseAction::Hide
    } else {
        CloseAction::Exit
    }
}
#[derive(Clone, Copy, Default, Debug, PartialEq, Serialize, Deserialize)]
pub struct Unread {
    pub count: Option<u32>,
    pub unread: bool,
}
pub fn unread_from_title(title: &str) -> Option<Unread> {
    let title = title.trim();
    if matches!(title, "WhatsApp" | "WhatsApp Web") {
        return Some(Unread {
            count: Some(0),
            unread: false,
        });
    }
    let remaining = title.strip_prefix('(')?;
    let (number, suffix) = remaining.split_once(')')?;
    if !matches!(suffix.trim(), "WhatsApp" | "WhatsApp Web")
        || number.is_empty()
        || number.len() > 6
        || !number.bytes().all(|c| c.is_ascii_digit())
    {
        return None;
    }
    let count = number.parse::<u32>().ok()?;
    if count > 999999 {
        return None;
    }
    Some(Unread {
        count: Some(count),
        unread: count > 0,
    })
}
pub fn aggregate(unread: &HashMap<String, Unread>) -> (u64, bool) {
    (
        unread.values().filter_map(|s| s.count).map(u64::from).sum(),
        unread.values().any(|s| s.unread && s.count.is_none()),
    )
}
#[derive(Default)]
pub struct NotificationGate {
    active: HashSet<String>,
}
impl NotificationGate {
    pub fn accept(&mut self, event: &str, config: &DesktopConfig, account_enabled: bool) -> bool {
        config.notifications
            && !config.dnd
            && account_enabled
            && self.active.len() < 256
            && self.active.insert(event.into())
    }
    pub fn release(&mut self, event: &str) {
        self.active.remove(event);
    }
}
pub fn clean_text(value: &str, limit: usize) -> String {
    value
        .chars()
        .filter(|c| {
            !c.is_control() && !matches!(*c,'\u{202a}'..='\u{202e}'|'\u{2066}'..='\u{2069}')
        })
        .take(limit)
        .collect()
}
pub fn xml(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
pub fn notification_xml(
    account: &str,
    title: &str,
    body: &str,
    id: &str,
    ticket: &str,
    private: bool,
    silent: bool,
) -> String {
    // The activation URI contains only host-generated UUIDs, never a provider URL or payload.
    let title = if private {
        clean_text(account, 64)
    } else {
        format!("{} — {}", clean_text(account, 64), clean_text(title, 128))
    };
    let body = if private {
        "New WhatsApp notification".into()
    } else {
        clean_text(body, 512)
    };
    format!(
        "<toast activationType=\"protocol\" launch=\"appterisk://notification/{id}/{ticket}\"><visual><binding template=\"ToastGeneric\"><text>{}</text><text>{}</text></binding></visual><audio silent=\"{}\"/></toast>",
        xml(&title),
        xml(&body),
        if silent { "true" } else { "false" }
    )
}
pub fn activation(value: &str) -> Option<(String, String)> {
    let url = tauri::Url::parse(value).ok()?;
    if url.scheme() != "appterisk"
        || url.host_str() != Some("notification")
        || url.query().is_some()
        || url.fragment().is_some()
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return None;
    }
    let parts: Vec<_> = url.path_segments()?.collect();
    if parts.len() != 2 {
        return None;
    }
    let id = uuid::Uuid::parse_str(parts[0]).ok()?.to_string();
    let ticket = uuid::Uuid::parse_str(parts[1]).ok()?.to_string();
    if id != parts[0] || ticket != parts[1] {
        return None;
    }
    Some((id, ticket))
}
pub fn safe_filename(suggested: &str) -> String {
    // Treat both path separators as hostile even when running tests on Unix.
    let leaf = suggested.rsplit(['\\', '/']).next().unwrap_or("");
    let mut name: String = leaf
        .chars()
        .filter(|c| !c.is_control() && !['<', '>', ':', '"', '/', '\\', '|', '?', '*'].contains(c))
        .take(140)
        .collect();
    name = name.trim_matches([' ', '.']).to_owned();
    if name.is_empty() {
        return "attachment".into();
    }
    let stem = name.split('.').next().unwrap_or("").to_uppercase();
    if [
        "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
        "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
    ]
    .contains(&stem.as_str())
    {
        name.insert(0, '_');
    }
    name
}
pub fn collision_path(directory: &Path, name: &str, index: u32) -> PathBuf {
    let name = safe_filename(name);
    if index == 0 {
        return directory.join(name);
    }
    let p = Path::new(&name);
    let stem = p
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("attachment");
    let ext = p
        .extension()
        .and_then(|s| s.to_str())
        .map(|e| format!(".{e}"))
        .unwrap_or_default();
    directory.join(format!("{stem} ({index}){ext}"))
}
pub fn persist_download(
    file: tempfile::TempPath,
    directory: &Path,
    name: &str,
) -> Result<PathBuf, String> {
    let directory = directory
        .canonicalize()
        .map_err(|_| "Download folder unavailable.")?;
    let mut file = file;
    for index in 0..10000 {
        let target = collision_path(&directory, name, index);
        match file.persist_noclobber(&target) {
            Ok(_) => return Ok(target),
            Err(error) => {
                let collision = error.error.kind() == std::io::ErrorKind::AlreadyExists;
                file = error.path;
                if !collision {
                    return Err("Could not finalize download without overwriting files.".into());
                }
            }
        }
    }
    Err("Too many filename collisions. Choose a different folder.".into())
}
pub fn shortcut(code: u32, ctrl: bool, shift: bool) -> Option<String> {
    if !ctrl {
        return None;
    }
    match code {
        9 => Some(if shift { "previous" } else { "next" }.into()),
        49..=57 => char::from_u32(code).map(|c| c.to_string()),
        82 => Some("r".into()),
        188 => Some(",".into()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn routing_and_payload_safety() {
        let a = uuid::Uuid::new_v4().to_string();
        let t = uuid::Uuid::new_v4().to_string();
        assert_eq!(
            activation(&format!("appterisk://notification/{a}/{t}")),
            Some((a.clone(), t.clone()))
        );
        for u in [
            "appterisk://notification/../../evil",
            "https://notification/a/b",
            "appterisk://settings/quit",
        ] {
            assert!(activation(u).is_none());
        }
        assert!(activation(&format!("appterisk://notification/{a}/{t}?command=quit")).is_none());
        let text = notification_xml("Work", "<script>", "private message", &a, &t, true, true);
        assert!(!text.contains("private message"));
        assert!(text.contains("silent=\"true\""));
        assert!(!notification_xml("Work", "<bad>", "x&y", &a, &t, false, false).contains("<bad>"));
    }
    #[test]
    fn preferences_and_dedup() {
        let mut gate = NotificationGate::default();
        let mut c = DesktopConfig::default();
        assert!(!gate.accept("a", &c, false));
        assert!(gate.accept("a", &c, true));
        assert!(!gate.accept("a", &c, true));
        assert!(gate.accept("b", &c, true));
        c.dnd = true;
        assert!(!gate.accept("c", &c, true));
        c.dnd = false;
        c.notifications = false;
        assert!(!gate.accept("d", &c, true));
        c.notifications = true;
        gate.release("a");
        assert!(gate.accept("a", &c, true));
    }
    #[test]
    fn unread_is_provider_scoped_and_unknown_does_not_clear() {
        assert_eq!(unread_from_title("(12) WhatsApp").unwrap().count, Some(12));
        assert!(!unread_from_title("WhatsApp").unwrap().unread);
        assert!(unread_from_title("Customer name (12)").is_none());
        assert!(unread_from_title("(12) WhatsApp evil").is_none());
        let mut map = HashMap::new();
        map.insert(
            "a".into(),
            Unread {
                count: Some(3),
                unread: true,
            },
        );
        map.insert(
            "b".into(),
            Unread {
                count: None,
                unread: true,
            },
        );
        assert_eq!(aggregate(&map), (3, true));
    }
    #[test]
    fn downloads_never_escape_or_overwrite() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(safe_filename("../../CON.txt"), "_CON.txt");
        assert_eq!(safe_filename("..\\..\\x?.txt"), "x.txt");
        assert_eq!(safe_filename("..."), "attachment");
        let file = tempfile::NamedTempFile::new_in(dir.path()).unwrap();
        std::fs::write(dir.path().join("image.png"), b"existing").unwrap();
        let saved = persist_download(file.into_temp_path(), dir.path(), "../image.png").unwrap();
        assert_eq!(saved.file_name().unwrap(), "image (1).png");
        assert_eq!(
            std::fs::read(dir.path().join("image.png")).unwrap(),
            b"existing"
        );
        assert!(saved.starts_with(dir.path().canonicalize().unwrap()));
    }
    #[test]
    fn permission_and_shortcut_policy() {
        assert!(valid_permission(ORIGIN, "microphone", "ask"));
        assert!(!valid_permission("https://evil.test", "camera", "allow"));
        assert!(!valid_permission(ORIGIN, "filesystem", "allow"));
        assert_eq!(shortcut(9, true, true).as_deref(), Some("previous"));
        assert_eq!(shortcut(9, true, false).as_deref(), Some("next"));
        assert!(shortcut(65, true, false).is_none());
        assert!(shortcut(82, false, false).is_none());
        assert!(!DesktopConfig::default().close_to_tray);
        let mut config = DesktopConfig::default();
        assert_eq!(close_action(&config, false, false), CloseAction::Exit);
        config.close_to_tray = true;
        assert_eq!(close_action(&config, false, false), CloseAction::Hide);
        assert_eq!(close_action(&config, true, false), CloseAction::Exit);
        assert_eq!(close_action(&config, false, true), CloseAction::Exit);
    }
}
