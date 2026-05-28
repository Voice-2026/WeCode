use crate::runtime_paths::runtime_temp_dir;
use std::path::PathBuf;

pub fn runtime_root_dir() -> PathBuf {
    runtime_temp_dir().join("runtime-root")
}

pub fn runtime_event_dir() -> PathBuf {
    runtime_temp_dir().join("runtime-events")
}

pub fn runtime_live_log_path() -> PathBuf {
    runtime_temp_dir().join("live.log")
}
