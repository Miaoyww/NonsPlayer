use crate::model::AppResult;
use rusqlite::{params, Connection, OptionalExtension};
use serde_json::{Map, Value};
use std::{path::Path, sync::Mutex};

pub struct Database(Mutex<Connection>);
impl Database {
    pub fn open(path: &Path) -> AppResult<Self> {
        let db = Connection::open(path).map_err(|e| e.to_string())?;
        db.execute_batch("PRAGMA journal_mode=WAL; CREATE TABLE IF NOT EXISTS plugins (id TEXT PRIMARY KEY, enabled INTEGER NOT NULL DEFAULT 0); CREATE TABLE IF NOT EXISTS storage (plugin TEXT NOT NULL, key TEXT NOT NULL, value TEXT NOT NULL, PRIMARY KEY(plugin,key)); CREATE TABLE IF NOT EXISTS settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);").map_err(|e| e.to_string())?;
        db.execute_batch("CREATE TABLE IF NOT EXISTS configuration (plugin TEXT PRIMARY KEY, revision INTEGER NOT NULL, value TEXT NOT NULL); CREATE TABLE IF NOT EXISTS directory_grants (plugin TEXT NOT NULL, id TEXT NOT NULL, path TEXT NOT NULL, writable INTEGER NOT NULL, PRIMARY KEY(plugin,id));").map_err(|e| e.to_string())?;
        db.execute_batch("CREATE TABLE IF NOT EXISTS plugin_authorizations (plugin TEXT PRIMARY KEY, scope TEXT NOT NULL);").map_err(|e| e.to_string())?;
        db.execute_batch("CREATE TABLE IF NOT EXISTS plugin_secret_keys (plugin TEXT NOT NULL, key TEXT NOT NULL, PRIMARY KEY(plugin,key));").map_err(|e| e.to_string())?;
        Ok(Self(Mutex::new(db)))
    }
    fn lock(&self) -> AppResult<std::sync::MutexGuard<'_, Connection>> {
        self.0
            .lock()
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("plugins.database.lock", error);
            })
            .map_err(|_| "插件存储不可用".into())
    }
    pub fn secret_keys(&self, plugin: &str) -> AppResult<Vec<String>> {
        let db = self.lock()?;
        let mut query = db
            .prepare("SELECT key FROM plugin_secret_keys WHERE plugin=?1")
            .map_err(|e| e.to_string())?;
        let keys = query
            .query_map([plugin], |row| row.get(0))
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<String>, _>>()
            .map_err(|e| e.to_string())?;
        Ok(keys)
    }
    pub fn secret_key(&self, plugin: &str, key: &str, add: bool) -> AppResult<()> {
        self.lock()?
            .execute(
                if add {
                    "INSERT OR IGNORE INTO plugin_secret_keys VALUES(?1,?2)"
                } else {
                    "DELETE FROM plugin_secret_keys WHERE plugin=?1 AND key=?2"
                },
                params![plugin, key],
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }
    pub fn enabled(&self, id: &str, scope: &str) -> AppResult<bool> {
        Ok(self
            .lock()?
            .query_row("SELECT enabled FROM plugins JOIN plugin_authorizations ON plugin=id WHERE id=?1 AND scope=?2", params![id,scope], |r| {
                r.get::<_, bool>(0)
            })
            .optional()
            .map_err(|e| e.to_string())?
            .unwrap_or(false))
    }
    pub fn authorize(&self, id: &str, scope: &str) -> AppResult<()> {
        let mut db = self.lock()?;
        let tx = db.transaction().map_err(|e| e.to_string())?;
        tx.execute("INSERT INTO plugin_authorizations VALUES(?1,?2) ON CONFLICT(plugin) DO UPDATE SET scope=excluded.scope", params![id,scope]).map_err(|e| e.to_string())?;
        tx.execute(
            "INSERT INTO plugins(id,enabled) VALUES(?1,1) ON CONFLICT(id) DO UPDATE SET enabled=1",
            [id],
        )
        .map_err(|e| e.to_string())?;
        tx.commit().map_err(|e| e.to_string())
    }
    pub fn enable(&self, id: &str, value: bool) -> AppResult<()> {
        self.lock()?.execute("INSERT INTO plugins(id,enabled) VALUES(?1,?2) ON CONFLICT(id) DO UPDATE SET enabled=excluded.enabled", params![id,value]).map_err(|e| e.to_string())?;
        Ok(())
    }
    pub fn initialized(&self) -> AppResult<bool> {
        Ok(self
            .lock()?
            .query_row("SELECT value FROM settings WHERE key='bundled'", [], |r| {
                r.get::<_, String>(0)
            })
            .optional()
            .map_err(|e| e.to_string())?
            .is_some())
    }
    pub fn mark_initialized(&self) -> AppResult<()> {
        self.lock()?
            .execute("INSERT OR REPLACE INTO settings VALUES('bundled','1')", [])
            .map_err(|e| e.to_string())?;
        Ok(())
    }
    pub fn remove(&self, id: &str) -> AppResult<()> {
        let mut db = self.lock()?;
        let tx = db.transaction().map_err(|e| e.to_string())?;
        tx.execute("DELETE FROM plugins WHERE id=?1", [id])
            .map_err(|e| e.to_string())?;
        tx.execute("DELETE FROM plugin_authorizations WHERE plugin=?1", [id])
            .map_err(|e| e.to_string())?;
        tx.execute("DELETE FROM storage WHERE plugin=?1", [id])
            .map_err(|e| e.to_string())?;
        tx.execute("DELETE FROM configuration WHERE plugin=?1", [id])
            .map_err(|e| e.to_string())?;
        tx.commit().map_err(|e| e.to_string())
    }
    pub fn config_read(&self, id: &str) -> AppResult<(u64, Map<String, Value>)> {
        let row: Option<(u64, String)> = self
            .lock()?
            .query_row(
                "SELECT revision,value FROM configuration WHERE plugin=?1",
                [id],
                |r| Ok((r.get::<_, i64>(0)? as u64, r.get(1)?)),
            )
            .optional()
            .map_err(|e| e.to_string())?;
        match row {
            Some((revision, value)) => Ok((
                revision,
                serde_json::from_str(&value).map_err(|e| e.to_string())?,
            )),
            None => Ok((0, Map::new())),
        }
    }
    pub fn config_change(
        &self,
        id: &str,
        expected: u64,
        change: impl FnOnce(&mut Map<String, Value>) -> AppResult<()>,
    ) -> AppResult<()> {
        let mut db = self.lock()?;
        let tx = db.transaction().map_err(|e| e.to_string())?;
        let row: Option<(u64, String)> = tx
            .query_row(
                "SELECT revision,value FROM configuration WHERE plugin=?1",
                [id],
                |r| Ok((r.get::<_, i64>(0)? as u64, r.get(1)?)),
            )
            .optional()
            .map_err(|e| e.to_string())?;
        let (revision, mut values) = match row {
            Some((revision, text)) => (
                revision,
                serde_json::from_str(&text).map_err(|e| e.to_string())?,
            ),
            None => (0, Map::new()),
        };
        if expected != revision {
            return Err("配置已被其他页面修改，请刷新后重试".into());
        }
        if revision >= 9_007_199_254_740_991 {
            return Err("配置修订号已耗尽".into());
        }
        change(&mut values)?;
        let text = serde_json::to_string(&values).map_err(|e| e.to_string())?;
        if text.len() > 65536 {
            return Err("配置超过 64KiB".into());
        }
        tx.execute("INSERT INTO configuration VALUES(?1,?2,?3) ON CONFLICT(plugin) DO UPDATE SET revision=excluded.revision,value=excluded.value", params![id,(revision+1) as i64,text]).map_err(|e| e.to_string())?;
        tx.commit().map_err(|e| e.to_string())
    }
    pub fn grants(&self, id: &str) -> AppResult<Vec<(String, String, bool)>> {
        let db = self.lock()?;
        let mut stmt = db
            .prepare("SELECT id,path,writable FROM directory_grants WHERE plugin=?1 ORDER BY id")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())
    }
    pub fn grant(&self, plugin: &str, id: &str, path: &str, writable: bool) -> AppResult<()> {
        self.lock()?
            .execute(
                "INSERT INTO directory_grants VALUES(?1,?2,?3,?4)",
                params![plugin, id, path, writable],
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }
    pub fn revoke(&self, plugin: &str, id: Option<&str>) -> AppResult<()> {
        self.lock()?
            .execute(
                "DELETE FROM directory_grants WHERE plugin=?1 AND (?2 IS NULL OR id=?2)",
                params![plugin, id],
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }
    pub fn get(&self, id: &str, key: &str) -> AppResult<Option<String>> {
        check_key(key)?;
        self.lock()?
            .query_row(
                "SELECT value FROM storage WHERE plugin=?1 AND key=?2",
                params![id, key],
                |r| r.get(0),
            )
            .optional()
            .map_err(|e| e.to_string())
    }
    pub fn set(&self, id: &str, key: &str, value: &str) -> AppResult<()> {
        check_key(key)?;
        if value.len() > 65536 {
            return Err("插件存储单值超过 64KiB".into());
        }
        serde_json::from_str::<serde_json::Value>(value)
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("plugins.storage.set.parse_value", error);
            })
            .map_err(|_| "存储值必须为 JSON")?;
        let mut db = self.lock()?;
        let tx = db.transaction().map_err(|e| e.to_string())?;
        let total: i64 = tx.query_row("SELECT coalesce(sum(length(CAST(key AS BLOB))+length(CAST(value AS BLOB))),0) FROM storage WHERE plugin=?1 AND key!=?2", params![id,key], |r| r.get(0)).map_err(|e| e.to_string())?;
        if total + (key.len() + value.len()) as i64 > 1024 * 1024 {
            return Err("插件存储超过 1MiB".into());
        }
        tx.execute("INSERT INTO storage VALUES(?1,?2,?3) ON CONFLICT(plugin,key) DO UPDATE SET value=excluded.value", params![id,key,value]).map_err(|e| e.to_string())?;
        tx.commit().map_err(|e| e.to_string())
    }
    pub fn delete(&self, id: &str, key: &str) -> AppResult<()> {
        check_key(key)?;
        self.lock()?
            .execute(
                "DELETE FROM storage WHERE plugin=?1 AND key=?2",
                params![id, key],
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }
}
fn check_key(key: &str) -> AppResult<()> {
    if key.is_empty() || key.len() > 128 {
        Err("存储键无效".into())
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn wildcard_file_roots_require_new_authorization() {
        let db = Database::open(Path::new(":memory:")).unwrap();
        let mut manifest: super::super::manifest::Manifest = serde_json::from_str(include_str!(
            "../../../../plugins/ncm-converter/manifest.json"
        ))
        .unwrap();
        manifest.file_roots.clear();
        db.authorize(&manifest.id, &manifest.authorization())
            .unwrap();
        manifest.file_roots = vec!["*".into()];
        assert!(!db.enabled(&manifest.id, &manifest.authorization()).unwrap());
        db.authorize(&manifest.id, &manifest.authorization())
            .unwrap();
        assert!(db.enabled(&manifest.id, &manifest.authorization()).unwrap());
    }
    #[test]
    fn authorization_is_bound_to_scope_and_cannot_upgrade_legacy_enablement() {
        let db = Database::open(Path::new(":memory:")).unwrap();
        db.enable("a", true).unwrap();
        assert!(!db.enabled("a", "old").unwrap());
        db.authorize("a", "old").unwrap();
        assert!(db.enabled("a", "old").unwrap());
        assert!(!db.enabled("a", "new").unwrap());
        assert!(!db.enabled("b", "old").unwrap());
        db.enable("a", false).unwrap();
        assert!(!db.enabled("a", "old").unwrap());
        db.remove("a").unwrap();
        db.enable("a", true).unwrap();
        assert!(!db.enabled("a", "old").unwrap());
    }
    #[test]
    fn account_credentials_permission_requires_new_authorization() {
        let db = Database::open(Path::new(":memory:")).unwrap();
        let new: super::super::manifest::Manifest = serde_json::from_str(include_str!(
            "../../../../plugins/netease-download/manifest.json"
        ))
        .unwrap();
        let mut old = new.clone();
        old.permissions.retain(|p| p != "account:credentials");
        db.authorize(&new.id, &old.authorization()).unwrap();
        assert!(!db.enabled(&new.id, &new.authorization()).unwrap());
        db.authorize(&new.id, &new.authorization()).unwrap();
        assert!(db.enabled(&new.id, &new.authorization()).unwrap());
    }
    #[test]
    fn wildcard_http_hosts_require_new_authorization() {
        let db = Database::open(Path::new(":memory:")).unwrap();
        let mut manifest: super::super::manifest::Manifest = serde_json::from_str(include_str!(
            "../../../../plugins/netease-download/manifest.json"
        ))
        .unwrap();
        manifest.http_hosts = vec!["interface.music.163.com".into()];
        db.authorize(&manifest.id, &manifest.authorization())
            .unwrap();
        manifest.http_hosts = vec!["*".into()];
        assert!(!db.enabled(&manifest.id, &manifest.authorization()).unwrap());
        db.authorize(&manifest.id, &manifest.authorization())
            .unwrap();
        assert!(db.enabled(&manifest.id, &manifest.authorization()).unwrap());
    }
    #[test]
    fn storage_isolation_and_quota() {
        let db = Database::open(Path::new(":memory:")).unwrap();
        db.set("a", "key", "123").unwrap();
        assert_eq!(db.get("b", "key").unwrap(), None);
        db.delete("b", "key").unwrap();
        assert_eq!(db.get("a", "key").unwrap().as_deref(), Some("123"));
        assert!(db
            .set("a", "large", &format!("\"{}\"", "a".repeat(65536)))
            .is_err());
        db.remove("a").unwrap();
        assert_eq!(db.get("a", "key").unwrap(), None);
    }
}
