use std::path::Path;

use chrono::{DateTime, Utc};
use serde_json::Value;

use super::Credentials;
use crate::error::GatewayError;

/// Load credentials from a Kiro IDE style JSON file.
pub fn load(path: &Path, creds: &mut Credentials) -> Result<(), GatewayError> {
    let path = expand(path);
    if !path.exists() {
        return Err(GatewayError::Auth(format!(
            "credentials file not found: {}",
            path.display()
        )));
    }
    let text = std::fs::read_to_string(&path)
        .map_err(|e| GatewayError::Auth(format!("failed to read credentials file: {e}")))?;
    let data: Value = serde_json::from_str(&text)
        .map_err(|e| GatewayError::Auth(format!("failed to parse credentials file: {e}")))?;

    if let Some(v) = data.get("refreshToken").and_then(Value::as_str) {
        creds.refresh_token = Some(v.to_string());
    }
    if let Some(v) = data.get("accessToken").and_then(Value::as_str) {
        creds.access_token = Some(v.to_string());
    }
    if let Some(v) = data.get("profileArn").and_then(Value::as_str) {
        creds.profile_arn = Some(v.to_string());
    }
    if let Some(v) = data.get("region").and_then(Value::as_str) {
        creds.sso_region = Some(v.to_string());
        creds.detected_api_region = Some(v.to_string());
    }
    if let Some(v) = data.get("clientIdHash").and_then(Value::as_str) {
        load_enterprise_device_registration(v, creds);
    }
    if let Some(v) = data.get("clientId").and_then(Value::as_str) {
        creds.client_id = Some(v.to_string());
    }
    if let Some(v) = data.get("clientSecret").and_then(Value::as_str) {
        creds.client_secret = Some(v.to_string());
    }
    if let Some(v) = data.get("expiresAt").and_then(Value::as_str) {
        creds.expires_at = parse_datetime(v);
    }
    Ok(())
}

/// Enterprise Kiro IDE: device registration at ~/.aws/sso/cache/{hash}.json.
fn load_enterprise_device_registration(client_id_hash: &str, creds: &mut Credentials) {
    let Some(home) = home_dir() else { return };
    let path = home
        .join(".aws")
        .join("sso")
        .join("cache")
        .join(format!("{client_id_hash}.json"));
    if !path.exists() {
        tracing::warn!(
            "enterprise device registration not found: {}",
            path.display()
        );
        return;
    }
    let Ok(text) = std::fs::read_to_string(&path) else {
        return;
    };
    let Ok(data) = serde_json::from_str::<Value>(&text) else {
        return;
    };
    if let Some(v) = data.get("clientId").and_then(Value::as_str) {
        creds.client_id = Some(v.to_string());
    }
    if let Some(v) = data.get("clientSecret").and_then(Value::as_str) {
        creds.client_secret = Some(v.to_string());
    }
}

/// Save updated tokens back to the JSON file, preserving unknown fields.
pub fn save(path: &Path, creds: &Credentials) -> Result<(), GatewayError> {
    let path = expand(path);
    let mut existing: Value = if path.exists() {
        std::fs::read_to_string(&path)
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_else(|| Value::Object(Default::default()))
    } else {
        Value::Object(Default::default())
    };
    let obj = existing
        .as_object_mut()
        .ok_or_else(|| GatewayError::Internal("credentials file is not a JSON object".into()))?;

    if let Some(v) = &creds.access_token {
        obj.insert("accessToken".into(), Value::String(v.clone()));
    }
    if let Some(v) = &creds.refresh_token {
        obj.insert("refreshToken".into(), Value::String(v.clone()));
    }
    if let Some(v) = &creds.expires_at {
        obj.insert("expiresAt".into(), Value::String(v.to_rfc3339()));
    }
    if let Some(v) = &creds.profile_arn {
        obj.insert("profileArn".into(), Value::String(v.clone()));
    }

    let serialized = serde_json::to_string_pretty(&existing)
        .map_err(|e| GatewayError::Internal(format!("failed to serialize credentials: {e}")))?;
    std::fs::write(&path, serialized)
        .map_err(|e| GatewayError::Internal(format!("failed to write credentials file: {e}")))?;
    Ok(())
}

/// Parse an ISO-8601 / RFC3339 timestamp, tolerating trailing 'Z' and
/// nanosecond precision (kiro-cli writes 9 fractional digits).
pub fn parse_datetime(s: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(s)
        .ok()
        .map(|d| d.with_timezone(&Utc))
}

fn home_dir() -> Option<std::path::PathBuf> {
    std::env::var_os("HOME").map(std::path::PathBuf::from)
}

pub fn expand(path: &Path) -> std::path::PathBuf {
    let s = path.to_string_lossy();
    if let Some(rest) = s.strip_prefix("~/") {
        if let Some(home) = home_dir() {
            return home.join(rest);
        }
    }
    path.to_path_buf()
}
