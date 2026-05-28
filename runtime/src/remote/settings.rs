use super::RemoteService;
use super::types::RemoteSettings;
use serde_json::{Map, Value};
use std::fs;

impl RemoteService {
    pub(super) fn raw_settings(&self) -> Map<String, Value> {
        fs::read_to_string(&self.settings_path)
            .ok()
            .and_then(|content| serde_json::from_str::<Value>(&content).ok())
            .and_then(|value| value.as_object().cloned())
            .unwrap_or_default()
    }

    pub(super) fn save_raw_settings(&self, settings: &Map<String, Value>) -> Result<(), String> {
        if let Some(parent) = self.settings_path.parent() {
            fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        let content = serde_json::to_string_pretty(settings).map_err(|error| error.to_string())?;
        fs::write(&self.settings_path, format!("{content}\n")).map_err(|error| error.to_string())
    }
}

pub(crate) fn remote_settings_mut(
    raw: &mut Map<String, Value>,
) -> Result<&mut Map<String, Value>, String> {
    raw.entry("remote".to_string())
        .or_insert_with(|| Value::Object(Map::new()))
        .as_object_mut()
        .ok_or_else(|| "Remote settings are invalid.".to_string())
}

pub(crate) fn remote_settings_from_raw(raw: &Map<String, Value>) -> RemoteSettings {
    raw.get("remote")
        .cloned()
        .and_then(|remote| serde_json::from_value::<RemoteSettings>(remote).ok())
        .unwrap_or_default()
}
