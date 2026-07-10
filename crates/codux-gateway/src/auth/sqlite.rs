use std::path::{Path, PathBuf};

use regex::Regex;
use rusqlite::{Connection, OpenFlags};
use serde_json::Value;

use super::file::{expand, parse_datetime};
use super::Credentials;
use crate::error::GatewayError;

/// Token keys in priority order.
const TOKEN_KEYS: &[&str] = &[
    "kirocli:social:token",
    "kirocli:odic:token",
    "codewhisperer:odic:token",
];

/// Device-registration keys (client_id / client_secret) in priority order.
const REGISTRATION_KEYS: &[&str] = &[
    "kirocli:odic:device-registration",
    "codewhisperer:odic:device-registration",
];

pub fn resolve_db_path(path: Option<PathBuf>) -> PathBuf {
    match path {
        Some(p) => expand(&p),
        None => default_db_path(),
    }
}

fn default_db_path() -> PathBuf {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_default();
    #[cfg(target_os = "macos")]
    {
        return home.join("Library/Application Support/kiro-cli/data.sqlite3");
    }
    #[cfg(not(target_os = "macos"))]
    home.join(".local/share/kiro-cli/data.sqlite3")
}

/// Load credentials from the kiro-cli SQLite database.
pub fn load(db_path: &Path, creds: &mut Credentials) -> Result<(), GatewayError> {
    if !db_path.exists() {
        return Err(GatewayError::Auth(format!(
            "kiro-cli SQLite database not found: {}",
            db_path.display()
        )));
    }
    let conn = Connection::open_with_flags(db_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|e| GatewayError::Auth(format!("failed to open kiro-cli sqlite: {e}")))?;

    // Token.
    for key in TOKEN_KEYS {
        if let Some(raw) = read_kv(&conn, "auth_kv", key) {
            if let Ok(data) = serde_json::from_str::<Value>(&raw) {
                apply_token(&data, creds);
                creds.sqlite_token_key = Some((*key).to_string());
                break;
            }
        }
    }

    // Device registration.
    for key in REGISTRATION_KEYS {
        if let Some(raw) = read_kv(&conn, "auth_kv", key) {
            if let Ok(data) = serde_json::from_str::<Value>(&raw) {
                if let Some(v) = data.get("client_id").and_then(Value::as_str) {
                    creds.client_id = Some(v.to_string());
                }
                if let Some(v) = data.get("client_secret").and_then(Value::as_str) {
                    creds.client_secret = Some(v.to_string());
                }
                if creds.sso_region.is_none() {
                    if let Some(v) = data.get("region").and_then(Value::as_str) {
                        creds.sso_region = Some(v.to_string());
                    }
                }
                break;
            }
        }
    }

    // API region from state table profile ARN.
    if let Some(raw) = read_kv(&conn, "state", "api.codewhisperer.profile") {
        if let Ok(data) = serde_json::from_str::<Value>(&raw) {
            if let Some(arn) = data.get("arn").and_then(Value::as_str) {
                if creds.profile_arn.is_none() {
                    creds.profile_arn = Some(arn.to_string());
                }
                if let Some(region) = region_from_arn(arn) {
                    creds.detected_api_region = Some(region);
                }
            }
        }
    }

    Ok(())
}

fn apply_token(data: &Value, creds: &mut Credentials) {
    if let Some(v) = data.get("access_token").and_then(Value::as_str) {
        creds.access_token = Some(v.to_string());
    }
    if let Some(v) = data.get("refresh_token").and_then(Value::as_str) {
        creds.refresh_token = Some(v.to_string());
    }
    if let Some(v) = data.get("profile_arn").and_then(Value::as_str) {
        creds.profile_arn = Some(v.to_string());
    }
    if let Some(v) = data.get("region").and_then(Value::as_str) {
        creds.sso_region = Some(v.to_string());
    }
    if let Some(arr) = data.get("scopes").and_then(Value::as_array) {
        creds.scopes = Some(
            arr.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect(),
        );
    }
    if let Some(v) = data.get("expires_at").and_then(Value::as_str) {
        creds.expires_at = parse_datetime(v);
    }
}

/// arn:aws:codewhisperer:REGION:account:profile/id -> REGION (validated).
fn region_from_arn(arn: &str) -> Option<String> {
    let parts: Vec<&str> = arn.split(':').collect();
    if parts.len() < 4 {
        return None;
    }
    let region = parts[3];
    let re = Regex::new(r"^[a-z]+-[a-z]+-\d+$").ok()?;
    if re.is_match(region) {
        Some(region.to_string())
    } else {
        None
    }
}

fn read_kv(conn: &Connection, table: &str, key: &str) -> Option<String> {
    let sql = format!("SELECT value FROM {table} WHERE key = ?1");
    conn.query_row(&sql, [key], |row| row.get::<_, String>(0))
        .ok()
}

/// Write refreshed tokens back into the SQLite DB (read-merge-write).
pub fn save(db_path: &Path, creds: &Credentials, region: &str) -> Result<(), GatewayError> {
    if !db_path.exists() {
        return Err(GatewayError::Internal(format!(
            "kiro-cli SQLite database not found for writing: {}",
            db_path.display()
        )));
    }
    let conn = Connection::open(db_path)
        .map_err(|e| GatewayError::Internal(format!("failed to open sqlite for write: {e}")))?;

    // Try the known key first, then all keys.
    let mut keys: Vec<&str> = Vec::new();
    if let Some(k) = &creds.sqlite_token_key {
        keys.push(k.as_str());
    }
    keys.extend(TOKEN_KEYS.iter().copied());

    for key in keys {
        if try_save_to_key(&conn, key, creds, region)? {
            return Ok(());
        }
    }
    Err(GatewayError::Internal(
        "no matching kiro-cli sqlite key to write back".into(),
    ))
}

fn try_save_to_key(
    conn: &Connection,
    key: &str,
    creds: &Credentials,
    region: &str,
) -> Result<bool, GatewayError> {
    let Some(raw) = read_kv(conn, "auth_kv", key) else {
        return Ok(false);
    };
    let mut data: Value = match serde_json::from_str(&raw) {
        Ok(v) => v,
        Err(_) => return Ok(false),
    };
    let Some(obj) = data.as_object_mut() else {
        return Ok(false);
    };
    obj.insert(
        "access_token".into(),
        json_or_null(creds.access_token.as_deref()),
    );
    obj.insert(
        "refresh_token".into(),
        json_or_null(creds.refresh_token.as_deref()),
    );
    obj.insert(
        "expires_at".into(),
        creds
            .expires_at
            .map(|e| Value::String(e.to_rfc3339()))
            .unwrap_or(Value::Null),
    );
    obj.insert("region".into(), Value::String(region.to_string()));
    if let Some(scopes) = &creds.scopes {
        obj.insert(
            "scopes".into(),
            Value::Array(scopes.iter().cloned().map(Value::String).collect()),
        );
    }

    let serialized = serde_json::to_string(&data)
        .map_err(|e| GatewayError::Internal(format!("failed to serialize token: {e}")))?;
    let rows = conn
        .execute(
            "UPDATE auth_kv SET value = ?1 WHERE key = ?2",
            rusqlite::params![serialized, key],
        )
        .map_err(|e| GatewayError::Internal(format!("sqlite update failed: {e}")))?;
    Ok(rows > 0)
}

fn json_or_null(v: Option<&str>) -> Value {
    v.map(|s| Value::String(s.to_string()))
        .unwrap_or(Value::Null)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_db_path_matches_platform_storage() {
        let path = default_db_path();
        let path = path.to_string_lossy();
        #[cfg(target_os = "macos")]
        assert!(
            path.ends_with("Library/Application Support/kiro-cli/data.sqlite3"),
            "{path}"
        );
        #[cfg(not(target_os = "macos"))]
        assert!(
            path.ends_with(".local/share/kiro-cli/data.sqlite3"),
            "{path}"
        );
    }

    #[test]
    fn explicit_db_path_still_wins() {
        let path = resolve_db_path(Some(PathBuf::from("/tmp/custom-kiro.sqlite3")));
        assert_eq!(path, PathBuf::from("/tmp/custom-kiro.sqlite3"));
    }
}
