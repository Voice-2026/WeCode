use anyhow::Context;
use rusqlite::{Connection, params};
use serde::Serialize;
use std::{
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("not found")]
    NotFound,
    #[error(transparent)]
    Sqlite(#[from] rusqlite::Error),
}

pub type StoreResult<T> = Result<T, StoreError>;

#[derive(Debug)]
pub struct Store {
    conn: Connection,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Host {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing)]
    pub token: String,
    pub public_key: String,
    pub created_at: i64,
    pub last_seen: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Device {
    pub id: String,
    pub host_id: String,
    pub name: String,
    #[allow(dead_code)]
    #[serde(skip_serializing)]
    pub token: String,
    pub public_key: String,
    pub created_at: i64,
    pub last_seen: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revoked_at: Option<i64>,
    pub online: bool,
}

impl Store {
    pub fn open(path: &Path) -> anyhow::Result<Self> {
        if let Some(parent) = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("create db dir {}", parent.display()))?;
        }
        let conn = Connection::open(path).with_context(|| format!("open db {}", path.display()))?;
        conn.pragma_update(None, "busy_timeout", 5000)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        let store = Self { conn };
        store.migrate()?;
        Ok(store)
    }

    fn migrate(&self) -> StoreResult<()> {
        self.conn.execute_batch(
            r#"
CREATE TABLE IF NOT EXISTS hosts (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  token TEXT NOT NULL UNIQUE,
  public_key TEXT NOT NULL DEFAULT '',
  created_at INTEGER NOT NULL,
  last_seen INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS devices (
  id TEXT PRIMARY KEY,
  host_id TEXT NOT NULL,
  name TEXT NOT NULL,
  token TEXT NOT NULL UNIQUE,
  public_key TEXT NOT NULL DEFAULT '',
  created_at INTEGER NOT NULL,
  last_seen INTEGER NOT NULL,
  revoked_at INTEGER,
  FOREIGN KEY(host_id) REFERENCES hosts(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_devices_host ON devices(host_id);
"#,
        )?;
        Ok(())
    }

    pub fn upsert_host(
        &self,
        id: Option<String>,
        name: Option<String>,
        token: Option<String>,
        public_key: Option<String>,
    ) -> StoreResult<Host> {
        let now = now_millis();
        let host = Host {
            id: id.unwrap_or_else(|| Uuid::new_v4().to_string()),
            name: non_empty(name, "Codux Mac"),
            token: token.unwrap_or_else(|| token_url(32)),
            public_key: public_key.unwrap_or_default(),
            created_at: now,
            last_seen: now,
        };
        self.conn.execute(
            r#"
INSERT INTO hosts(id, name, token, public_key, created_at, last_seen)
VALUES(?1, ?2, ?3, ?4, ?5, ?6)
ON CONFLICT(id) DO UPDATE SET
  name=excluded.name,
  token=excluded.token,
  public_key=excluded.public_key,
  last_seen=excluded.last_seen
"#,
            params![
                host.id,
                host.name,
                host.token,
                host.public_key,
                host.created_at,
                host.last_seen
            ],
        )?;
        Ok(host)
    }

    pub fn host_by_token(&self, token: &str) -> StoreResult<Host> {
        self.conn
            .query_row(
                "SELECT id, name, token, public_key, created_at, last_seen FROM hosts WHERE token=?1",
                [token],
                scan_host,
            )
            .map_err(map_not_found)
    }

    pub fn devices_for_host(&self, host_id: &str) -> StoreResult<Vec<Device>> {
        let mut statement = self.conn.prepare(
            r#"
SELECT id, host_id, name, token, public_key, created_at, last_seen, revoked_at
FROM devices
WHERE host_id=?1 AND revoked_at IS NULL
ORDER BY created_at DESC
"#,
        )?;
        let rows = statement.query_map([host_id], scan_device)?;
        let mut devices = Vec::new();
        for row in rows {
            devices.push(row?);
        }
        Ok(devices)
    }

    pub fn revoke_device(&self, host_id: &str, device_id: &str) -> StoreResult<()> {
        let affected = self.conn.execute(
            "UPDATE devices SET revoked_at=?1 WHERE host_id=?2 AND id=?3 AND revoked_at IS NULL",
            params![now_millis(), host_id, device_id],
        )?;
        affected_or_not_found(affected)
    }
}

fn scan_host(row: &rusqlite::Row<'_>) -> rusqlite::Result<Host> {
    Ok(Host {
        id: row.get(0)?,
        name: row.get(1)?,
        token: row.get(2)?,
        public_key: row.get(3)?,
        created_at: row.get(4)?,
        last_seen: row.get(5)?,
    })
}

fn scan_device(row: &rusqlite::Row<'_>) -> rusqlite::Result<Device> {
    Ok(Device {
        id: row.get(0)?,
        host_id: row.get(1)?,
        name: row.get(2)?,
        token: row.get(3)?,
        public_key: row.get(4)?,
        created_at: row.get(5)?,
        last_seen: row.get(6)?,
        revoked_at: row.get(7)?,
        online: false,
    })
}

fn map_not_found(error: rusqlite::Error) -> StoreError {
    match error {
        rusqlite::Error::QueryReturnedNoRows => StoreError::NotFound,
        other => StoreError::Sqlite(other),
    }
}

fn affected_or_not_found(affected: usize) -> StoreResult<()> {
    if affected == 0 {
        Err(StoreError::NotFound)
    } else {
        Ok(())
    }
}

fn non_empty(value: Option<String>, fallback: &str) -> String {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| fallback.to_string())
}

pub fn token_url(bytes: usize) -> String {
    use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
    let mut data = vec![0_u8; bytes];
    getrandom::fill(&mut data).expect("system random");
    URL_SAFE_NO_PAD.encode(data)
}

pub fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}
