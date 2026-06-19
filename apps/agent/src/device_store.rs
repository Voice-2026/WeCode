//! Persisted record of devices that have paired with this host (`devices.json`).
//! The running host upserts a device on pairing; the `device` CLI commands and
//! `status` read it.

use serde::{Deserialize, Serialize};

use crate::paths;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PairedDevice {
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub platform: String,
    #[serde(default)]
    pub paired_at: String,
    #[serde(default)]
    pub last_seen: String,
}

fn load() -> Vec<PairedDevice> {
    std::fs::read_to_string(paths::devices_path())
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

fn save(devices: &[PairedDevice]) -> Result<(), String> {
    paths::ensure_data_dir();
    let text = serde_json::to_string_pretty(devices).map_err(|error| error.to_string())?;
    std::fs::write(paths::devices_path(), text).map_err(|error| error.to_string())
}

pub fn list() -> Vec<PairedDevice> {
    load()
}

fn now() -> String {
    chrono::Utc::now().to_rfc3339()
}

/// Upsert a device on (re)pairing: refresh name/platform/last_seen, preserving
/// the original pairing time.
pub fn record(id: &str, name: &str, platform: &str) {
    let mut devices = load();
    let timestamp = now();
    if let Some(existing) = devices.iter_mut().find(|device| device.id == id) {
        if !name.trim().is_empty() {
            existing.name = name.to_string();
        }
        if !platform.trim().is_empty() {
            existing.platform = platform.to_string();
        }
        existing.last_seen = timestamp;
    } else {
        devices.push(PairedDevice {
            id: id.to_string(),
            name: name.to_string(),
            platform: platform.to_string(),
            paired_at: timestamp.clone(),
            last_seen: timestamp,
        });
    }
    let _ = save(&devices);
}

/// Remove a device by id. Returns whether it existed.
pub fn remove(id: &str) -> Result<bool, String> {
    let mut devices = load();
    let before = devices.len();
    devices.retain(|device| device.id != id);
    let removed = devices.len() != before;
    save(&devices)?;
    Ok(removed)
}

/// Rename a device by id. Returns whether it existed.
pub fn rename(id: &str, name: &str) -> Result<bool, String> {
    let mut devices = load();
    let found = if let Some(device) = devices.iter_mut().find(|device| device.id == id) {
        device.name = name.to_string();
        true
    } else {
        false
    };
    save(&devices)?;
    Ok(found)
}

pub fn clear() -> Result<usize, String> {
    let count = load().len();
    save(&[])?;
    Ok(count)
}
