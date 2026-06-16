use serde::Deserialize;
use std::sync::OnceLock;

const RELAY_PRESETS_JSON: &str = include_str!("relay_presets.json");

pub const GLOBAL_RELAY_SERVER_URL: &str = GLOBAL_IROH_RELAY_SERVER_URL;
pub const CHINA_RELAY_SERVER_URL: &str = CHINA_IROH_RELAY_SERVER_URL;
pub const DEFAULT_RELAY_SERVER_URL: &str = GLOBAL_RELAY_SERVER_URL;
pub const GLOBAL_IROH_RELAY_SERVER_URL: &str = "";
pub const CHINA_IROH_RELAY_SERVER_URL: &str = "https://iroh-service.dux.plus";

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct RemoteRelayPreset {
    pub key: String,
    pub name: String,
    pub url: String,
}

static RELAY_PRESETS: OnceLock<Vec<RemoteRelayPreset>> = OnceLock::new();

pub fn remote_relay_presets() -> &'static [RemoteRelayPreset] {
    RELAY_PRESETS
        .get_or_init(|| serde_json::from_str(RELAY_PRESETS_JSON).expect("valid relay presets"))
        .as_slice()
}

pub fn remote_relay_presets_json() -> String {
    RELAY_PRESETS_JSON.to_string()
}

pub fn remote_relay_url_for_preset(preset: &str, custom_url: &str) -> String {
    iroh_relay_url_for_preset(preset, custom_url)
}

pub fn remote_relay_preset_for_url(url: &str) -> String {
    iroh_relay_preset_for_url(url)
}

pub fn iroh_relay_url_for_preset(preset: &str, custom_url: &str) -> String {
    let preset = normalize_remote_relay_preset(preset, custom_url);
    if preset == "custom" {
        return custom_url.trim().to_string();
    }
    remote_relay_presets()
        .iter()
        .find(|item| item.key == preset)
        .map(|item| item.url.trim().to_string())
        .unwrap_or_default()
}

pub fn iroh_relay_preset_for_url(url: &str) -> String {
    let url = url.trim();
    if url.is_empty() {
        return "global".to_string();
    }
    let normalized_url = url.trim_end_matches('/');
    remote_relay_presets()
        .iter()
        .find(|item| item.key != "custom" && item.url.trim_end_matches('/') == normalized_url)
        .map(|item| item.key.clone())
        .unwrap_or_else(|| "custom".to_string())
}

pub fn normalize_remote_relay_preset(preset: &str, relay_url: &str) -> String {
    let preset = preset.trim();
    if preset == "china" {
        return "china-tencent".to_string();
    }
    if let Some(item) = remote_relay_presets()
        .iter()
        .find(|item| item.key == preset)
    {
        return item.key.clone();
    }
    iroh_relay_preset_for_url(relay_url)
}

pub fn remote_relay_url(value: &str) -> String {
    value.trim().to_string()
}

pub fn remote_url(base: &str, path: &str, query: &[(&str, &str)]) -> Result<String, String> {
    let mut url = url::Url::parse(base.trim()).map_err(|error| error.to_string())?;
    url.set_path(&join_url_path(url.path(), path));
    url.set_query(None);
    if !query.is_empty() {
        let mut pairs = url.query_pairs_mut();
        for (key, value) in query {
            pairs.append_pair(key, value);
        }
    }
    Ok(url.to_string())
}

pub fn preferred_controller_transport_kind<'a>(
    candidates: impl IntoIterator<Item = (&'a str, &'a str)>,
) -> &'static str {
    for (kind, _) in candidates {
        if kind == "iroh" {
            return "iroh";
        }
    }
    ""
}

pub fn preferred_pairing_transport_kind<'a>(
    candidates: impl IntoIterator<Item = (&'a str, &'a str)>,
) -> &'static str {
    for (kind, _) in candidates {
        if kind == "iroh" {
            return "iroh";
        }
    }
    ""
}

fn join_url_path(base_path: &str, path: &str) -> String {
    let base_path = base_path.trim_end_matches('/');
    let path = path.trim_start_matches('/');
    if base_path.is_empty() {
        format!("/{path}")
    } else if path.is_empty() {
        base_path.to_string()
    } else {
        format!("{base_path}/{path}")
    }
}
