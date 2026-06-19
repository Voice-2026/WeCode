//! The headless host's persisted configuration (`codux.toml`).
//!
//! `host_id` + `host_token` seed the iroh node identity (see
//! `host_secret_key`), so they must stay stable across restarts — otherwise the
//! node id (and every saved desktop's reconnect target) changes. They are
//! generated once on first `config` and preserved thereafter.

use serde::{Deserialize, Serialize};

use crate::paths;

pub const RELAY_PRESET_CUSTOM: &str = "custom";

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, rename_all = "snake_case")]
pub struct CoduxConfig {
    /// The name shown for this host on paired desktops.
    pub device_name: String,
    /// Stable logical host id (part of the node-identity seed).
    pub host_id: String,
    /// Stable secret seeding the iroh node identity. Treat as sensitive.
    pub host_token: String,
    /// Relay preset key (e.g. "global", "china", or "custom").
    pub relay_preset: String,
    /// Relay URL when `relay_preset` is "custom".
    pub relay_url: String,
    /// Optional bearer token for a custom relay.
    pub relay_authentication: String,
}

impl Default for CoduxConfig {
    fn default() -> Self {
        Self {
            device_name: default_device_name(),
            host_id: String::new(),
            host_token: String::new(),
            relay_preset: "global".to_string(),
            relay_url: String::new(),
            relay_authentication: String::new(),
        }
    }
}

impl CoduxConfig {
    /// Load the config, or a default if none exists yet.
    pub fn load() -> Self {
        std::fs::read_to_string(paths::config_path())
            .ok()
            .and_then(|text| toml::from_str(&text).ok())
            .unwrap_or_default()
    }

    /// True if a config file exists on disk.
    pub fn exists() -> bool {
        paths::config_path().exists()
    }

    /// Fill in a stable identity (host id + token) if absent. Returns whether
    /// anything was generated, so the caller can persist.
    pub fn ensure_identity(&mut self) -> bool {
        let mut generated = false;
        if self.host_id.trim().is_empty() {
            self.host_id = format!("codux-{}", random_hex(6));
            generated = true;
        }
        if self.host_token.trim().is_empty() {
            self.host_token = random_hex(32);
            generated = true;
        }
        generated
    }

    pub fn save(&self) -> Result<(), String> {
        paths::ensure_data_dir();
        let text = toml::to_string_pretty(self).map_err(|error| error.to_string())?;
        std::fs::write(paths::config_path(), text).map_err(|error| error.to_string())
    }
}

/// A lowercase hex string of `bytes` random bytes (2 chars per byte).
pub fn random_hex(bytes: usize) -> String {
    let mut buf = vec![0u8; bytes];
    if getrandom::getrandom(&mut buf).is_err() {
        // Fallback: time-seeded, only reached if the OS RNG is unavailable.
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        for (index, slot) in buf.iter_mut().enumerate() {
            *slot = ((nanos >> (index % 16 * 8)) as u8) ^ (index as u8).wrapping_mul(31);
        }
    }
    buf.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// A sensible default device name (the machine's hostname).
pub fn default_device_name() -> String {
    std::env::var("HOSTNAME")
        .ok()
        .or_else(|| std::env::var("COMPUTERNAME").ok())
        .map(|name| name.trim().to_string())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| "codux-agent".to_string())
}
