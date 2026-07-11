//! Filesystem layout for the headless host. Everything lives under the data dir
//! (`~/.wecode-agent`, or `WECODE_AGENT_DATA_DIR`): the TOML config, the device
//! store, the daemon's published pairing ticket + status, and the single-instance
//! lock. CLI commands read these files; the running daemon writes them.

use std::path::PathBuf;

pub use crate::projects::agent_data_dir as data_dir;

fn data_file(name: &str) -> PathBuf {
    data_dir().join(name)
}

/// The TOML configuration written by `wecode config`.
pub fn config_path() -> PathBuf {
    data_file("wecode.toml")
}

/// Devices that have paired with this host (a JSON array).
pub fn devices_path() -> PathBuf {
    data_file("devices.json")
}

/// The pasteable `wecode://pair` ticket the running daemon publishes for `link`
/// and `qrcode` to read.
pub fn ticket_path() -> PathBuf {
    data_file("pair-ticket.json")
}

/// The running daemon's status (pid, start time, node id) for `status`/`stop`.
pub fn status_path() -> PathBuf {
    data_file("status.json")
}

/// Single-instance advisory lock held for the daemon's lifetime.
pub fn lock_path() -> PathBuf {
    data_file("wecode.lock")
}

/// Rolling log file the daemon appends to when running detached.
pub fn log_path() -> PathBuf {
    data_file("wecode.log")
}

/// Create the data dir if missing (best effort).
pub fn ensure_data_dir() {
    let _ = std::fs::create_dir_all(data_dir());
}
