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
        if version > 1 {
            return Err("Database was created by a newer application; downgrade refused.".into());
        }
        let tx = connection.transaction().map_err(db_error)?;
        if version == 0 {
            tx.execute_batch("CREATE TABLE schema_migrations(version INTEGER PRIMARY KEY, applied_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')));
CREATE TABLE accounts(id TEXT PRIMARY KEY, name TEXT NOT NULL, platform TEXT NOT NULL CHECK(platform='whatsapp'), profile_key TEXT NOT NULL UNIQUE, icon TEXT NOT NULL, display_order INTEGER NOT NULL, notifications_enabled INTEGER NOT NULL CHECK(notifications_enabled IN (0,1)), created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')), updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')), last_active_at TEXT);
CREATE TABLE application_settings(key TEXT PRIMARY KEY, value TEXT NOT NULL);
INSERT INTO schema_migrations(version) VALUES(1); PRAGMA user_version=1;").map_err(db_error)?;
        }
        tx.commit().map_err(db_error)?;
        Ok(Self { connection })
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
        self.connection.execute("INSERT INTO accounts(id,name,platform,profile_key,icon,display_order,notifications_enabled) VALUES(?1,?2,'whatsapp',?3,'jade',(SELECT COALESCE(MAX(display_order),-1)+1 FROM accounts),0)",params![id,name,key]).map_err(db_error)?;
        self.account(&id)
    }
    pub fn update(&mut self, id: &str, name: &str, icon: &str, notifications: bool) -> Result<()> {
        let name = valid_name(name)?;
        if !["jade", "blue", "violet", "rose", "amber", "slate"].contains(&icon) {
            return Err("Unknown account icon.".into());
        }
        if self.connection.execute("UPDATE accounts SET name=?2,icon=?3,notifications_enabled=?4,updated_at=strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE id=?1",params![id,name,icon,notifications]).map_err(db_error)? != 1 { return Err("Account not found.".into()); }
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
            || !["system", "en", "ms", "zh"].contains(&settings.language.as_str())
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
            1
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
}
