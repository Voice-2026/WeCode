use super::types::RemoteSettings;
use reqwest::header::CONTENT_TYPE;
use serde_json::Value;

pub(crate) fn default_remote_server_url() -> String {
    "http://127.0.0.1:8088".to_string()
}

pub(crate) fn remote_server_url(settings: &RemoteSettings) -> String {
    if settings.server_url.trim().is_empty() {
        default_remote_server_url()
    } else {
        settings.server_url.trim().to_string()
    }
}

pub(crate) fn remote_url(
    base: &str,
    path: &str,
    query: &[(&str, &str)],
    websocket: bool,
) -> Result<String, String> {
    let mut url = url::Url::parse(base.trim()).map_err(|error| error.to_string())?;
    url.set_path(path);
    url.set_query(None);
    if websocket {
        let scheme = match url.scheme() {
            "https" => "wss",
            "http" => "ws",
            other => other,
        }
        .to_string();
        let _ = url.set_scheme(&scheme);
    }
    {
        let mut pairs = url.query_pairs_mut();
        for (key, value) in query {
            pairs.append_pair(key, value);
        }
    }
    Ok(url.to_string())
}

pub(crate) fn remote_http_client() -> Result<reqwest::blocking::Client, String> {
    reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(12))
        .build()
        .map_err(remote_error_message)
}

pub(crate) fn remote_parse_response<T: serde::de::DeserializeOwned>(
    response: reqwest::blocking::Response,
) -> Result<T, String> {
    let status = response.status();
    let bytes = response.bytes().map_err(remote_error_message)?;
    if !status.is_success() {
        if let Ok(value) = serde_json::from_slice::<Value>(&bytes) {
            if let Some(error) = value.get("error").and_then(Value::as_str) {
                return Err(error.to_string());
            }
        }
        return Err(String::from_utf8_lossy(&bytes).to_string());
    }
    serde_json::from_slice(&bytes).map_err(|error| {
        format!(
            "Remote response decode failed: {error}. Body: {}",
            String::from_utf8_lossy(&bytes)
        )
    })
}

pub(crate) fn remote_post<T: serde::de::DeserializeOwned>(
    base: &str,
    path: &str,
    body: Value,
) -> Result<T, String> {
    let url = remote_url(base, path, &[], false)?;
    let response = remote_http_client()?
        .post(url)
        .header(CONTENT_TYPE, "application/json")
        .json(&body)
        .send()
        .map_err(remote_error_message)?;
    remote_parse_response(response)
}

pub(crate) fn remote_error_message(error: impl std::fmt::Display) -> String {
    error.to_string()
}
