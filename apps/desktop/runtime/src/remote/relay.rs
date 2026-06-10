pub use codux_remote_transport::{CHINA_RELAY_SERVER_URL, GLOBAL_RELAY_SERVER_URL};

pub fn remote_relay_url_for_preset(preset: &str, custom_url: &str) -> String {
    codux_remote_transport::remote_relay_url_for_preset(preset, custom_url)
}

pub fn remote_relay_preset_for_url(url: &str) -> String {
    codux_remote_transport::remote_relay_preset_for_url(url)
}

pub(crate) fn remote_server_url(value: &str) -> String {
    codux_remote_transport::remote_server_url(value)
}

pub(crate) fn remote_stun_urls() -> Vec<String> {
    codux_remote_transport::remote_stun_urls()
}

pub(crate) fn remote_url(
    base: &str,
    path: &str,
    query: &[(&str, &str)],
    websocket: bool,
) -> Result<String, String> {
    codux_remote_transport::remote_url(base, path, query, websocket)
}

pub(crate) fn remote_pairing_ticket_payload(base: &str, ticket: &str) -> Result<String, String> {
    let mut url = url::Url::parse("codux://pair").map_err(|error| error.to_string())?;
    url.query_pairs_mut()
        .append_pair("server", base.trim())
        .append_pair("ticket", ticket.trim());
    Ok(url.to_string())
}
