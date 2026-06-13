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
    #[serde(skip_serializing)]
    pub token: String,
    pub public_key: String,
    pub created_at: i64,
    pub last_seen: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revoked_at: Option<i64>,
    pub online: bool,
}

#[derive(Debug, Clone)]
pub struct Pairing {
    pub id: String,
    pub host_id: String,
    pub code: String,
    pub secret: String,
    pub device_name: String,
    pub device_public_key: String,
    pub status: String,
    pub expires_at: i64,
    pub device_id: Option<String>,
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

    #[cfg(test)]
    pub fn in_memory() -> anyhow::Result<Self> {
        let store = Self {
            conn: Connection::open_in_memory()?,
        };
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
CREATE TABLE IF NOT EXISTS pairings (
  id TEXT PRIMARY KEY,
  host_id TEXT NOT NULL,
  code TEXT NOT NULL UNIQUE,
  secret TEXT NOT NULL,
  device_name TEXT NOT NULL DEFAULT '',
  device_public_key TEXT NOT NULL DEFAULT '',
  status TEXT NOT NULL,
  created_at INTEGER NOT NULL,
  expires_at INTEGER NOT NULL,
  claimed_at INTEGER,
  confirmed_at INTEGER,
  device_id TEXT,
  FOREIGN KEY(host_id) REFERENCES hosts(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_pairings_host ON pairings(host_id);
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

    pub fn host_by_id(&self, id: &str) -> StoreResult<Host> {
        self.conn
            .query_row(
                "SELECT id, name, token, public_key, created_at, last_seen FROM hosts WHERE id=?1",
                [id],
                scan_host,
            )
            .map_err(map_not_found)
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

    pub fn touch_host(&self, id: &str) -> StoreResult<()> {
        self.conn
            .execute(
                "UPDATE hosts SET last_seen=?1 WHERE id=?2",
                params![now_millis(), id],
            )
            .map(|_| ())
            .map_err(Into::into)
    }

    pub fn create_pairing(
        &self,
        host_id: String,
        code: String,
        secret: String,
        ttl_ms: i64,
    ) -> StoreResult<Pairing> {
        let now = now_millis();
        let pairing = Pairing {
            id: Uuid::new_v4().to_string(),
            host_id,
            code,
            secret,
            device_name: String::new(),
            device_public_key: String::new(),
            status: "pending".into(),
            expires_at: now.saturating_add(ttl_ms),
            device_id: None,
        };
        self.conn.execute(
            r#"
INSERT INTO pairings(id, host_id, code, secret, status, created_at, expires_at)
VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7)
"#,
            params![
                pairing.id,
                pairing.host_id,
                pairing.code,
                pairing.secret,
                pairing.status,
                now,
                pairing.expires_at
            ],
        )?;
        Ok(pairing)
    }

    pub fn pairing_by_code(&self, code: &str) -> StoreResult<Pairing> {
        self.conn
            .query_row(PAIRING_SELECT_BY_CODE, [code], scan_pairing)
            .map_err(map_not_found)
    }

    pub fn pairing_by_id(&self, id: &str) -> StoreResult<Pairing> {
        self.conn
            .query_row(PAIRING_SELECT_BY_ID, [id], scan_pairing)
            .map_err(map_not_found)
    }

    pub fn claim_pairing(
        &self,
        id: &str,
        device_name: &str,
        device_public_key: &str,
    ) -> StoreResult<()> {
        let affected = self.conn.execute(
            r#"
UPDATE pairings
SET status='claimed', device_name=?1, device_public_key=?2, claimed_at=?3
WHERE id=?4 AND status='pending' AND expires_at>?3
"#,
            params![device_name, device_public_key, now_millis(), id],
        )?;
        affected_or_not_found(affected)
    }

    pub fn confirm_pairing(&mut self, pairing: &Pairing) -> StoreResult<Device> {
        let now = now_millis();
        let device = Device {
            id: Uuid::new_v4().to_string(),
            host_id: pairing.host_id.clone(),
            name: non_empty(Some(pairing.device_name.clone()), "Mobile Device"),
            token: token_url(32),
            public_key: pairing.device_public_key.clone(),
            created_at: now,
            last_seen: now,
            revoked_at: None,
            online: false,
        };
        let tx = self.conn.transaction()?;
        tx.execute(
            r#"
INSERT INTO devices(id, host_id, name, token, public_key, created_at, last_seen)
VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7)
"#,
            params![
                device.id,
                device.host_id,
                device.name,
                device.token,
                device.public_key,
                device.created_at,
                device.last_seen
            ],
        )?;
        let affected = tx.execute(
            "UPDATE pairings SET status='confirmed', confirmed_at=?1, device_id=?2 WHERE id=?3 AND status='claimed'",
            params![now, device.id, pairing.id],
        )?;
        affected_or_not_found(affected)?;
        tx.commit()?;
        Ok(device)
    }

    pub fn reject_pairing(&self, host_id: &str, pairing_id: &str) -> StoreResult<()> {
        let affected = self.conn.execute(
            "UPDATE pairings SET status='rejected', confirmed_at=?1 WHERE host_id=?2 AND id=?3 AND status IN ('pending', 'claimed')",
            params![now_millis(), host_id, pairing_id],
        )?;
        affected_or_not_found(affected)
    }

    pub fn device_by_id(&self, id: &str) -> StoreResult<Device> {
        self.conn
            .query_row(DEVICE_SELECT_BY_ID, [id], scan_device)
            .map_err(map_not_found)
    }

    pub fn device_by_token(&self, token: &str) -> StoreResult<Device> {
        self.conn
            .query_row(DEVICE_SELECT_BY_TOKEN, [token], scan_device)
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

    pub fn touch_device(&self, id: &str) -> StoreResult<()> {
        self.conn
            .execute(
                "UPDATE devices SET last_seen=?1 WHERE id=?2",
                params![now_millis(), id],
            )
            .map(|_| ())
            .map_err(Into::into)
    }

    pub fn update_device_name(&self, id: &str, name: &str) -> StoreResult<()> {
        self.conn
            .execute(
                "UPDATE devices SET name=?1, last_seen=?2 WHERE id=?3 AND revoked_at IS NULL",
                params![name, now_millis(), id],
            )
            .map(|_| ())
            .map_err(Into::into)
    }

    pub fn revoke_device(&self, host_id: &str, device_id: &str) -> StoreResult<()> {
        let affected = self.conn.execute(
            "UPDATE devices SET revoked_at=?1 WHERE host_id=?2 AND id=?3 AND revoked_at IS NULL",
            params![now_millis(), host_id, device_id],
        )?;
        affected_or_not_found(affected)
    }
}

const PAIRING_SELECT_BY_CODE: &str = r#"
SELECT id, host_id, code, secret, device_name, device_public_key, status, expires_at, device_id
FROM pairings
WHERE code=?1
"#;

const PAIRING_SELECT_BY_ID: &str = r#"
SELECT id, host_id, code, secret, device_name, device_public_key, status, expires_at, device_id
FROM pairings
WHERE id=?1
"#;

const DEVICE_SELECT_BY_ID: &str = r#"
SELECT id, host_id, name, token, public_key, created_at, last_seen, revoked_at
FROM devices
WHERE id=?1
"#;

const DEVICE_SELECT_BY_TOKEN: &str = r#"
SELECT id, host_id, name, token, public_key, created_at, last_seen, revoked_at
FROM devices
WHERE token=?1
"#;

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

fn scan_pairing(row: &rusqlite::Row<'_>) -> rusqlite::Result<Pairing> {
    Ok(Pairing {
        id: row.get(0)?,
        host_id: row.get(1)?,
        code: row.get(2)?,
        secret: row.get(3)?,
        device_name: row.get(4)?,
        device_public_key: row.get(5)?,
        status: row.get(6)?,
        expires_at: row.get(7)?,
        device_id: row.get(8)?,
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

pub fn pairing_code() -> String {
    let mut data = [0_u8; 4];
    getrandom::fill(&mut data).expect("system random");
    let number = u32::from_le_bytes(data) % 1_000_000;
    format!("{number:06}")
}

pub fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn store_runs_pairing_lifecycle() {
        let mut store = Store::in_memory().unwrap();
        let host = store
            .upsert_host(Some("host-1".into()), Some("Host".into()), None, None)
            .unwrap();
        let pairing = store
            .create_pairing(host.id.clone(), "123456".into(), "secret".into(), 60_000)
            .unwrap();

        store
            .claim_pairing(&pairing.id, "Phone", "device-key")
            .unwrap();
        let pairing = store.pairing_by_id(&pairing.id).unwrap();
        assert_eq!(pairing.status, "claimed");
        assert_eq!(pairing.device_name, "Phone");

        let device = store.confirm_pairing(&pairing).unwrap();
        assert_eq!(device.host_id, host.id);
        assert_eq!(device.name, "Phone");
        assert!(store.device_by_token(&device.token).is_ok());
    }
}
