pub use wecode_remote_transport::GLOBAL_RELAY_SERVER_URL;

pub fn remote_relay_url_for_preset(preset: &str, custom_url: &str) -> String {
    wecode_remote_transport::remote_relay_url_for_preset(preset, custom_url)
}

pub fn remote_relay_preset_for_url(url: &str) -> String {
    wecode_remote_transport::remote_relay_preset_for_url(url)
}

pub fn normalize_remote_relay_preset(preset: &str, relay_url: &str) -> String {
    wecode_remote_transport::normalize_remote_relay_preset(preset, relay_url)
}

pub fn remote_relay_presets() -> &'static [wecode_remote_transport::RemoteRelayPreset] {
    wecode_remote_transport::remote_relay_presets()
}

pub(crate) fn remote_relay_url(value: &str) -> String {
    wecode_remote_transport::remote_relay_url(value)
}

pub(crate) fn remote_pairing_payload_url(payload: &serde_json::Value) -> Result<String, String> {
    let bytes = serde_json::to_vec(payload).map_err(|error| error.to_string())?;
    let encoded = crate::remote::crypto::remote_base64_url_encode(&bytes);
    let mut url = url::Url::parse("wecode://pair").map_err(|error| error.to_string())?;
    url.query_pairs_mut().append_pair("payload", &encoded);
    Ok(url.to_string())
}
