use reqwest::header::CONTENT_TYPE;
use serde_json::Value;
use std::time::Duration;

pub(crate) const DEFAULT_RELAY_SERVER_URL: &str = "https://codux-service.dux.plus";

pub(crate) fn remote_server_url(value: &str) -> String {
    let value = value.trim();
    if value.is_empty() {
        DEFAULT_RELAY_SERVER_URL.to_string()
    } else {
        value.to_string()
    }
}

pub(crate) fn remote_stun_urls() -> Vec<String> {
    vec![
        "stun:stun.miwifi.com:3478".to_string(),
        "stun:stun.l.google.com:19302".to_string(),
    ]
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

pub(crate) async fn remote_post<T: serde::de::DeserializeOwned>(
    base: &str,
    path: &str,
    body: Value,
) -> Result<T, String> {
    let url = remote_url(base, path, &[], false)?;
    let client = remote_http_client()?;
    let response = client
        .post(url)
        .header(CONTENT_TYPE, "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|error| error.to_string())?;
    remote_parse_response(response).await
}

pub(crate) async fn remote_get<T: serde::de::DeserializeOwned>(
    base: &str,
    path: &str,
    query: &[(&str, &str)],
) -> Result<T, String> {
    let url = remote_url(base, path, query, false)?;
    let client = remote_http_client()?;
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|error| error.to_string())?;
    remote_parse_response(response).await
}

async fn remote_parse_response<T: serde::de::DeserializeOwned>(
    response: reqwest::Response,
) -> Result<T, String> {
    let status = response.status();
    let bytes = response.bytes().await.map_err(|error| error.to_string())?;
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

fn remote_http_client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(12))
        .build()
        .map_err(|error| error.to_string())
}
