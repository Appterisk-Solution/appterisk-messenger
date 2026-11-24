//! Application metadata only. Browser authentication databases are never accessed.
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};
use uuid::Uuid;

pub const LEGACY_IDS: [&str; 3] = [
    "c4b9dc51-80ce-4e57-a738-4c182a216b01",
    "c4b9dc51-80ce-4e57-a738-4c182a216b02",
    "c4b9dc51-80ce-4e57-a738-4c182a216b03",
];
pub type Result<T> = std::result::Result<T, String>;
fn db_error(_: rusqlite::Error) -> String {
    "Account database operation failed; existing profiles are unchanged.".into()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    pub id: String,
    pub name: String,
    pub platform: String,
    pub profile_key: String,
    pub icon: String,
    pub display_order: i64,
    pub notifications_enabled: bool,
    pub created_at: String,
    pub updated_at: String,
    pub last_active_at: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Settings {
    pub theme: String,
    pub language: String,
    pub default_account: Option<String>,
    pub last_active_account: Option<String>,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: "system".into(),
            language: "system".into(),
            default_account: None,
            last_active_account: None,
        }
    }
}
pub struct Database {
    connection: Connection,
}
impl Database {
    pub fn open(path: &Path) -> Result<Self> {
        let mut connection = Connection::open(path).map_err(db_error)?;
        connection
            .busy_timeout(std::time::Duration::from_secs(5))
            .map_err(db_error)?;
        connection
            .execute_batch("PRAGMA foreign_keys=ON; PRAGMA journal_mode=WAL;")
            .map_err(db_error)?;
        let version: i64 = connection
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .map_err(db_error)?;
        if version > 3 {
            return Err("Database was created by a newer application; downgrade refused.".into());
        }
        if version > 0 && version < 3 {
            let folder = path
                .parent()
                .ok_or("Metadata directory unavailable")?
                .join("metadata-backups");
            std::fs::create_dir_all(&folder).map_err(|_| "Cannot create local metadata backup")?;
            let backup = folder.join(format!("schema-{version}-{}.sqlite3", Uuid::new_v4()));
            connection
                .backup("main", &backup, None::<fn(rusqlite::backup::Progress)>)
                .map_err(db_error)?;
        }
        let tx = connection.transaction().map_err(db_error)?;
        if version == 0 {
            tx.execute_batch("CREATE TABLE schema_migrations(version INTEGER PRIMARY KEY, applied_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')));
CREATE TABLE accounts(id TEXT PRIMARY KEY, name TEXT NOT NULL, platform TEXT NOT NULL CHECK(platform='whatsapp'), profile_key TEXT NOT NULL UNIQUE, icon TEXT NOT NULL, display_order INTEGER NOT NULL, notifications_enabled INTEGER NOT NULL CHECK(notifications_enabled IN (0,1)), created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')), updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')), last_active_at TEXT);
CREATE TABLE application_settings(key TEXT PRIMARY KEY, value TEXT NOT NULL);
INSERT INTO schema_migrations(version) VALUES(1); PRAGMA user_version=1;").map_err(db_error)?;
        }
        if version < 2 {
            tx.execute_batch("CREATE TABLE origin_permissions(account_id TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,origin TEXT NOT NULL,kind TEXT NOT NULL,decision TEXT NOT NULL CHECK(decision IN ('ask','allow','deny')),PRIMARY KEY(account_id,origin,kind)); INSERT INTO schema_migrations(version) VALUES(2); PRAGMA user_version=2;").map_err(db_error)?;
        }
        if version < 3 {
            tx.execute_batch("CREATE TABLE accounts_next(id TEXT PRIMARY KEY, name TEXT NOT NULL, platform TEXT NOT NULL, profile_key TEXT NOT NULL UNIQUE, icon TEXT NOT NULL, display_order INTEGER NOT NULL, notifications_enabled INTEGER NOT NULL CHECK(notifications_enabled IN (0,1)), created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')), updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')), last_active_at TEXT, custom_url TEXT, color TEXT NOT NULL DEFAULT 'jade', group_id TEXT, enabled INTEGER NOT NULL DEFAULT 1 CHECK(enabled IN (0,1)));
INSERT INTO accounts_next(id,name,platform,profile_key,icon,display_order,notifications_enabled,created_at,updated_at,last_active_at,color) SELECT id,name,platform,profile_key,icon,display_order,notifications_enabled,created_at,updated_at,last_active_at,icon FROM accounts;
CREATE TABLE permissions_next(account_id TEXT NOT NULL REFERENCES accounts_next(id) ON DELETE CASCADE,origin TEXT NOT NULL,kind TEXT NOT NULL,decision TEXT NOT NULL CHECK(decision IN ('ask','allow','deny')),PRIMARY KEY(account_id,origin,kind));
INSERT INTO permissions_next SELECT account_id,origin,kind,decision FROM origin_permissions;
DROP TABLE origin_permissions; DROP TABLE accounts; ALTER TABLE accounts_next RENAME TO accounts; ALTER TABLE permissions_next RENAME TO origin_permissions;
INSERT INTO schema_migrations(version) VALUES(3); PRAGMA user_version=3;").map_err(db_error)?;
        }
        tx.commit().map_err(db_error)?;
        Ok(Self { connection })
    }
    pub fn close_for_update(&mut self) -> Result<()> {
        let busy: i64 = self
            .connection
            .query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |row| row.get(0))
            .map_err(db_error)?;
        if busy != 0 {
            return Err("Metadata is busy; retry installation later.".into());
        }
        let replacement = Connection::open_in_memory().map_err(db_error)?;
        let original = std::mem::replace(&mut self.connection, replacement);
        if let Err((connection, error)) = original.close() {
            self.connection = connection;
            return Err(db_error(error));
        }
        Ok(())
    }
    pub fn accounts(&self) -> Result<Vec<Account>> {
        let mut statement=self.connection.prepare("SELECT id,name,platform,profile_key,icon,display_order,notifications_enabled,created_at,updated_at,last_active_at FROM accounts ORDER BY display_order,id").map_err(db_error)?;
        statement
            .query_map([], |r| {
                Ok(Account {
                    id: r.get(0)?,
                    name: r.get(1)?,
                    platform: r.get(2)?,
                    profile_key: r.get(3)?,
                    icon: r.get(4)?,
                    display_order: r.get(5)?,
                    notifications_enabled: r.get(6)?,
                    created_at: r.get(7)?,
                    updated_at: r.get(8)?,
                    last_active_at: r.get(9)?,
                })
            })
            .map_err(db_error)?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(db_error)
    }
    pub fn account(&self, id: &str) -> Result<Account> {
        self.accounts()?
            .into_iter()
            .find(|a| a.id == id)
            .ok_or_else(|| "Account not found.".into())
    }
    pub fn add(&mut self, name: &str, legacy: Option<&str>) -> Result<Account> {
        self.add_platform(name, legacy, "whatsapp")
    }
    pub fn add_platform(
        &mut self,
        name: &str,
        legacy: Option<&str>,
        platform: &str,
    ) -> Result<Account> {
        crate::platforms::available(platform)?;
        if legacy.is_some() && platform != "whatsapp" {
            return Err("Only WhatsApp profiles can be adopted.".into());
        }

        let name = valid_name(name)?;
        if legacy.is_some_and(|id| !LEGACY_IDS.contains(&id)) {
            return Err("Unknown compatibility profile.".into());
        }
        let id = legacy
            .map(str::to_owned)
            .unwrap_or_else(|| Uuid::new_v4().to_string());
        let key = format!(
            "{}:{id}",
            if legacy.is_some() {
                "compatibility"
            } else {
                "account"
            }
        );
        self.connection.execute("INSERT INTO accounts(id,name,platform,profile_key,icon,display_order,notifications_enabled) VALUES(?1,?2,?4,?3,'jade',(SELECT COALESCE(MAX(display_order),-1)+1 FROM accounts),0)",params![id,name,key,platform]).map_err(db_error)?;
        self.account(&id)
    }
    pub fn update(&mut self, id: &str, name: &str, icon: &str, notifications: bool) -> Result<()> {
        let name = valid_name(name)?;
        if !["jade", "blue", "violet", "rose", "amber", "slate"].contains(&icon) {
            return Err("Unknown account icon.".into());
        }
        if self.connection.execute("UPDATE accounts SET name=?2,icon=?3,color=?3,notifications_enabled=?4,updated_at=strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE id=?1",params![id,name,icon,notifications]).map_err(db_error)? != 1 { return Err("Account not found.".into()); }
        Ok(())
    }
    pub fn reorder(&mut self, ids: &[String]) -> Result<()> {
        let existing = self.accounts()?;
        let unique: HashSet<_> = ids.iter().collect();
        if ids.len() != existing.len()
            || unique.len() != ids.len()
            || existing.iter().any(|a| !unique.contains(&a.id))
        {
            return Err("Ordering must contain every account exactly once.".into());
        }
        let tx = self.connection.transaction().map_err(db_error)?;
        for (index, id) in ids.iter().enumerate() {
            tx.execute("UPDATE accounts SET display_order=?2,updated_at=strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE id=?1",params![id,index as i64]).map_err(db_error)?;
        }
        tx.commit().map_err(db_error)
    }
    pub fn settings(&self) -> Result<Settings> {
        let raw: Option<String> = self
            .connection
            .query_row(
                "SELECT value FROM application_settings WHERE key='preferences'",
                [],
                |r| r.get(0),
            )
            .optional()
            .map_err(db_error)?;
        raw.map(|s| serde_json::from_str(&s).map_err(|_| "Settings could not be read.".into()))
            .unwrap_or_else(|| Ok(Settings::default()))
    }
    pub fn save_settings(&mut self, settings: &Settings) -> Result<()> {
        if !["system", "dark", "light"].contains(&settings.theme.as_str())
            || !["system", "en", "ms", "zh", "zh-CN", "zh-TW"].contains(&settings.language.as_str())
        {
            return Err("Unsupported preference.".into());
        }
        for id in [&settings.default_account, &settings.last_active_account]
            .into_iter()
            .flatten()
        {
            self.account(id)?;
        }
        self.connection.execute("INSERT INTO application_settings(key,value) VALUES('preferences',?1) ON CONFLICT(key) DO UPDATE SET value=excluded.value",[serde_json::to_string(settings).map_err(|_|"Cannot encode settings.")?]).map_err(db_error)?;
        Ok(())
    }
    pub fn activate(&mut self, id: &str) -> Result<()> {
        self.account(id)?;
        let mut settings = self.settings()?;
        settings.last_active_account = Some(id.into());
        let tx = self.connection.transaction().map_err(db_error)?;
        tx.execute(
            "UPDATE accounts SET last_active_at=strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE id=?1",
            [id],
        )
        .map_err(db_error)?;
        tx.execute("INSERT INTO application_settings(key,value) VALUES('preferences',?1) ON CONFLICT(key) DO UPDATE SET value=excluded.value",[serde_json::to_string(&settings).map_err(|_|"Cannot encode settings.")?]).map_err(db_error)?;
        tx.commit().map_err(db_error)
    }
    pub fn remove(&mut self, id: &str, confirmed: bool) -> Result<()> {
        if !confirmed {
            return Err("Removal requires confirmation.".into());
        }
        self.account(id)?;
        let mut settings = self.settings()?;
        if settings.last_active_account.as_deref() == Some(id) {
            settings.last_active_account = None;
        }
        if settings.default_account.as_deref() == Some(id) {
            settings.default_account = None;
        }
        let tx = self.connection.transaction().map_err(db_error)?;
        tx.execute("DELETE FROM accounts WHERE id=?1", [id])
            .map_err(db_error)?;
        tx.execute("INSERT INTO application_settings(key,value) VALUES('preferences',?1) ON CONFLICT(key) DO UPDATE SET value=excluded.value",[serde_json::to_string(&settings).map_err(|_|"Cannot encode settings.")?]).map_err(db_error)?;
        tx.commit().map_err(db_error)
    }
    pub fn desktop_config(&self) -> Result<crate::desktop_policy::DesktopConfig> {
        self.read_value("desktop")?
            .map(|v| serde_json::from_str(&v).map_err(|_| "Desktop settings are invalid.".into()))
            .unwrap_or_else(|| Ok(Default::default()))
    }
    pub fn save_desktop_config(
        &mut self,
        config: &crate::desktop_policy::DesktopConfig,
    ) -> Result<()> {
        config.validate()?;
        self.write_value(
            "desktop",
            Some(&serde_json::to_string(config).map_err(|_| "Cannot encode desktop settings")?),
        )
    }
    pub fn lock_hash(&self) -> Result<Option<String>> {
        self.read_value("lock_hash")
    }
    pub fn save_lock_hash(&mut self, hash: Option<&str>) -> Result<()> {
        self.write_value("lock_hash", hash)
    }
    pub fn read_value(&self, key: &str) -> Result<Option<String>> {
        self.connection
            .query_row(
                "SELECT value FROM application_settings WHERE key=?1",
                [key],
                |r| r.get(0),
            )
            .optional()
            .map_err(db_error)
    }
    pub fn write_value(&mut self, key: &str, value: Option<&str>) -> Result<()> {
        if let Some(value) = value {
            self.connection.execute("INSERT INTO application_settings(key,value) VALUES(?1,?2) ON CONFLICT(key) DO UPDATE SET value=excluded.value",params![key,value]).map_err(db_error)?;
        } else {
            self.connection
                .execute("DELETE FROM application_settings WHERE key=?1", [key])
                .map_err(db_error)?;
        }
        Ok(())
    }
    pub fn permissions(&self) -> Result<Vec<crate::desktop_policy::Permission>> {
        let mut s=self.connection.prepare("SELECT account_id,origin,kind,decision FROM origin_permissions ORDER BY account_id,kind").map_err(db_error)?;
        s.query_map([], |r| {
            Ok(crate::desktop_policy::Permission {
                account_id: r.get(0)?,
                origin: r.get(1)?,
                kind: r.get(2)?,
                decision: r.get(3)?,
            })
        })
        .map_err(db_error)?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(db_error)
    }
    pub fn permission_save(&mut self, account: &str, kind: &str, decision: &str) -> Result<()> {
        let account_metadata = self.account(account)?;
        let platform = crate::platforms::available(&account_metadata.platform)?;
        if !platform.permission(platform.origin, kind)
            || !["ask", "allow", "deny"].contains(&decision)
        {
            return Err("Unsupported origin permission.".into());
        }
        self.connection.execute("INSERT INTO origin_permissions(account_id,origin,kind,decision) VALUES(?1,?2,?3,?4) ON CONFLICT(account_id,origin,kind) DO UPDATE SET decision=excluded.decision",params![account,platform.origin,kind,decision]).map_err(db_error)?;
        Ok(())
    }
}
fn valid_name(name: &str) -> Result<&str> {
    let name = name.trim();
    if name.is_empty() || name.chars().count() > 64 || name.chars().any(char::is_control) {
        Err("Use an account name of 1–64 characters.".into())
    } else {
        Ok(name)
    }
}
pub fn profile_path(root: &Path, account: &Account) -> Result<PathBuf> {
    let (kind, id) = account
        .profile_key
        .split_once(':')
        .ok_or("Invalid profile key.")?;
    if Uuid::parse_str(id).map(|v| v.to_string()).ok().as_deref() != Some(id) || id != account.id {
        return Err("Invalid permanent profile identifier.".into());
    }
    let directory = match kind {
        "compatibility" if LEGACY_IDS.contains(&id) => "compatibility-profiles",
        "account" => "accounts",
        _ => return Err("Unknown profile namespace.".into()),
    };
    Ok(root.join(directory).join(id))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn v2_migration_preserves_metadata_permissions_and_profile_keys() {
        let fixture = tempfile::tempdir().unwrap();
        let path = fixture.path().join("metadata.sqlite3");
        let c = Connection::open(&path).unwrap();
        c.execute_batch("CREATE TABLE schema_migrations(version INTEGER PRIMARY KEY, applied_at TEXT NOT NULL DEFAULT 'fixture'); INSERT INTO schema_migrations(version) VALUES(1),(2);
CREATE TABLE accounts(id TEXT PRIMARY KEY,name TEXT NOT NULL,platform TEXT NOT NULL CHECK(platform='whatsapp'),profile_key TEXT NOT NULL UNIQUE,icon TEXT NOT NULL,display_order INTEGER NOT NULL,notifications_enabled INTEGER NOT NULL,created_at TEXT NOT NULL,updated_at TEXT NOT NULL,last_active_at TEXT);
CREATE TABLE application_settings(key TEXT PRIMARY KEY,value TEXT NOT NULL);
CREATE TABLE origin_permissions(account_id TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,origin TEXT NOT NULL,kind TEXT NOT NULL,decision TEXT NOT NULL,PRIMARY KEY(account_id,origin,kind)); PRAGMA user_version=2;").unwrap();
        let id = LEGACY_IDS[0];
        c.execute("INSERT INTO accounts VALUES(?1,'Existing','whatsapp',?2,'blue',7,1,'created','updated','active')", params![id, format!("compatibility:{id}")]).unwrap();
        let settings = Settings {
            last_active_account: Some(id.into()),
            ..Default::default()
        };
        let raw = serde_json::to_string(&settings).unwrap();
        c.execute(
            "INSERT INTO application_settings VALUES('preferences',?1)",
            [&raw],
        )
        .unwrap();
        c.execute(
            "INSERT INTO application_settings VALUES('lock_hash','opaque-host-fixture')",
            [],
        )
        .unwrap();
        c.execute("INSERT INTO origin_permissions VALUES(?1,'https://web.whatsapp.com','notifications','allow')", [id]).unwrap();
        drop(c);
        let mut db = Database::open(&path).unwrap();
        let a = db.account(id).unwrap();
        assert_eq!(a.platform, "whatsapp");
        assert_eq!(a.profile_key, format!("compatibility:{id}"));
        assert_eq!(a.display_order, 7);
        assert_eq!(a.icon, "blue");
        assert!(a.notifications_enabled);
        assert_eq!(a.created_at, "created");
        assert_eq!(a.updated_at, "updated");
        assert_eq!(a.last_active_at.as_deref(), Some("active"));
        assert_eq!(db.settings().unwrap(), settings);
        assert_eq!(
            db.lock_hash().unwrap().as_deref(),
            Some("opaque-host-fixture")
        );
        assert_eq!(db.permissions().unwrap()[0].decision, "allow");
        let tg = db.add_platform("Telegram", None, "telegram").unwrap();
        let dc = db.add_platform("Discord", None, "discord").unwrap();
        assert_ne!(dc.id, tg.id);
        assert_ne!(dc.profile_key, tg.profile_key);
        assert_eq!(dc.platform, "discord");
        assert!(
            db.add_platform("Invalid Discord adoption", Some(id), "discord")
                .is_err()
        );
        let second = db.add_platform("Telegram 2", None, "telegram").unwrap();
        assert_ne!(tg.id, second.id);
        assert_ne!(
            profile_path(fixture.path(), &tg).unwrap(),
            profile_path(fixture.path(), &second).unwrap()
        );
        assert!(
            db.add_platform("Illegal adoption", Some(id), "telegram")
                .is_err()
        );
        assert!(db.add_platform("Unavailable", None, "gmail").is_err());
        db.permission_save(&tg.id, "notifications", "deny").unwrap();
        let permissions = db.permissions().unwrap();
        assert!(permissions.iter().any(|p| p.account_id == tg.id
            && p.origin == "https://web.telegram.org"
            && p.decision == "deny"));
        db.remove(&tg.id, true).unwrap();
        assert_eq!(db.permissions().unwrap().len(), 1);
        assert_eq!(db.account(&dc.id).unwrap().platform, "discord");
        let violations: i64 = db
            .connection
            .query_row("SELECT count(*) FROM pragma_foreign_key_check", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(violations, 0);
        drop(db);
        assert!(Database::open(&path).is_ok());
    }
    #[test]
    fn updater_preferences_and_closed_database_reopen_preserve_accounts() {
        let folder = tempfile::tempdir().unwrap();
        let path = folder.path().join("metadata.sqlite3");
        let mut db = Database::open(&path).unwrap();
        let account = db.add("Fixture", None).unwrap();
        db.write_value("updater", Some(r#"{"channel":"beta","automatic":false}"#))
            .unwrap();
        db.close_for_update().unwrap();
        drop(db);
        let db = Database::open(&path).unwrap();
        assert_eq!(
            db.account(&account.id).unwrap().profile_key,
            account.profile_key
        );
        assert_eq!(
            db.read_value("updater").unwrap().as_deref(),
            Some(r#"{"channel":"beta","automatic":false}"#)
        );
    }
    fn temporary() -> (PathBuf, Database) {
        let path = std::env::temp_dir().join(format!("appterisk-db-{}.sqlite", Uuid::new_v4()));
        let db = Database::open(&path).unwrap();
        (path, db)
    }
    #[test]
    fn migrations_crud_order_and_restart() {
        let (path, mut db) = temporary();
        let a = db.add(" Personal ", None).unwrap();
        let b = db.add("Work", None).unwrap();
        assert_ne!(a.id, b.id);
        assert_eq!(a.name, "Personal");
        db.update(&a.id, "Family", "blue", true).unwrap();
        db.reorder(&[b.id.clone(), a.id.clone()]).unwrap();
        assert!(db.reorder(&[a.id.clone(), a.id.clone()]).is_err());
        assert_eq!(db.accounts().unwrap()[0].id, b.id);
        db.activate(&a.id).unwrap();
        let mut s = db.settings().unwrap();
        s.default_account = Some(b.id.clone());
        s.theme = "light".into();
        db.save_settings(&s).unwrap();
        drop(db);
        let mut db = Database::open(&path).unwrap();
        assert_eq!(db.accounts().unwrap().len(), 2);
        assert_eq!(db.settings().unwrap(), s);
        assert_eq!(db.account(&a.id).unwrap().profile_key, a.profile_key);
        assert!(
            !db.account(&a.id)
                .unwrap()
                .last_active_at
                .unwrap()
                .is_empty()
        );
        assert!(db.remove(&a.id, false).is_err());
        db.remove(&a.id, true).unwrap();
        assert!(db.settings().unwrap().last_active_account.is_none());
        assert!(db.account(&b.id).is_ok());
        assert_eq!(
            db.connection
                .query_row("SELECT COUNT(*) FROM schema_migrations", [], |r| r
                    .get::<_, i64>(0))
                .unwrap(),
            3
        );
        drop(db);
        std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn adoption_keeps_uuid_and_path_without_copying() {
        let (path, mut db) = temporary();
        let a = db.add("Existing", Some(LEGACY_IDS[0])).unwrap();
        assert_eq!(a.id, LEGACY_IDS[0]);
        assert_eq!(
            profile_path(Path::new("root"), &a).unwrap(),
            Path::new("root")
                .join("compatibility-profiles")
                .join(LEGACY_IDS[0])
        );
        assert!(db.add("Duplicate", Some(LEGACY_IDS[0])).is_err());
        assert!(db.add("Bad", Some("../bad")).is_err());
        let mut bad = a.clone();
        bad.profile_key = "account:../../bad".into();
        assert!(profile_path(Path::new("root"), &bad).is_err());
        db.remove(&a.id, true).unwrap();
        assert_eq!(db.add("Readopt", Some(LEGACY_IDS[0])).unwrap().id, a.id);
        drop(db);
        std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn validates_names_preferences_and_future_schema() {
        let (path, mut db) = temporary();
        assert!(db.add("\n", None).is_err());
        let s = Settings {
            theme: "other".into(),
            ..Default::default()
        };
        assert!(db.save_settings(&s).is_err());
        db.connection
            .pragma_update(None, "user_version", 99)
            .unwrap();
        drop(db);
        assert!(Database::open(&path).is_err());
        std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn desktop_migration_preserves_existing_v1_accounts_and_settings() {
        let (path, mut db) = temporary();
        let a = db.add("Existing account", Some(LEGACY_IDS[0])).unwrap();
        db.activate(&a.id).unwrap();
        db.connection.execute_batch("DROP TABLE origin_permissions;DELETE FROM schema_migrations WHERE version>=2;PRAGMA user_version=1;").unwrap();
        drop(db);
        let mut db = Database::open(&path).unwrap();
        assert_eq!(db.account(&a.id).unwrap().profile_key, a.profile_key);
        assert_eq!(
            db.settings().unwrap().last_active_account,
            Some(a.id.clone())
        );
        let mut config = db.desktop_config().unwrap();
        assert!(!config.close_to_tray);
        assert!(config.hide_previews);
        config.dnd = true;
        db.save_desktop_config(&config).unwrap();
        db.permission_save(&a.id, "microphone", "deny").unwrap();
        assert!(db.permission_save(&a.id, "filesystem", "allow").is_err());
        drop(db);
        let db = Database::open(&path).unwrap();
        assert_eq!(db.desktop_config().unwrap(), config);
        assert_eq!(db.permissions().unwrap()[0].account_id, a.id);
        assert_eq!(db.permissions().unwrap()[0].decision, "deny");
        assert!(db.lock_hash().unwrap().is_none());
        drop(db);
        std::fs::remove_file(path).unwrap();
    }
}
