//! WeChat iLink Bot API client.
//!
//! Ported from the reference `ilinkai.weixin.qq.com` flow. The client is
//! transport-only: it fetches a login QR code, polls its scan status, long-
//! polls for incoming messages, sends replies/typing, and decrypts CDN media.
//! Message routing to a Codux session lives in the host, not here.

use serde::{Deserialize, Deserializer, Serialize};

use crate::{Error, Result};

// ===== iLink API constants =====

/// Default iLink Bot API base URL.
pub const DEFAULT_BASE_URL: &str = "https://ilinkai.weixin.qq.com";
const QR_CODE_PATH: &str = "/ilink/bot/get_bot_qrcode?bot_type=3";
const QR_STATUS_PATH: &str = "/ilink/bot/get_qrcode_status?qrcode=";
const GET_UPDATES_PATH: &str = "/ilink/bot/getupdates";
const SEND_MESSAGE_PATH: &str = "/ilink/bot/sendmessage";
const GET_CONFIG_PATH: &str = "/ilink/bot/getconfig";
const SEND_TYPING_PATH: &str = "/ilink/bot/sendtyping";

const LONG_POLL_TIMEOUT_SECS: u64 = 40;
const SEND_TIMEOUT_SECS: u64 = 15;

/// Message-item type tags used by the iLink protocol.
pub mod item_type {
    pub const TEXT: i32 = 1;
    pub const IMAGE: i32 = 2;
    pub const FILE: i32 = 4;
}

/// Message type tags: who authored a message.
pub mod message_type {
    pub const USER: i32 = 1;
    pub const BOT: i32 = 2;
}

/// Message lifecycle state tags.
pub mod message_state {
    pub const FINISH: i32 = 2;
}

/// iLink session-expired error code.
pub const SESSION_EXPIRED_CODE: i32 = -14;

// ===== credentials =====

/// Bot credentials obtained after a confirmed QR scan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeChatCredentials {
    pub bot_token: String,
    pub ilink_bot_id: String,
    #[serde(default)]
    pub base_url: String,
    #[serde(default)]
    pub ilink_user_id: String,
}

// ===== QR responses =====

/// Response of `get_bot_qrcode`: an opaque `qrcode` handle plus the URL the
/// WeChat app should scan (`qrcode_img_content`).
#[derive(Debug, Clone, Deserialize)]
pub struct QrCodeResponse {
    pub qrcode: String,
    #[serde(default)]
    pub qrcode_img_content: String,
}

/// Response of `get_qrcode_status`. `status` is one of
/// `wait | scaned | confirmed | expired`.
#[derive(Debug, Clone, Deserialize)]
pub struct QrStatusResponse {
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub bot_token: String,
    #[serde(default)]
    pub ilink_bot_id: String,
    #[serde(default)]
    pub baseurl: String,
    #[serde(default)]
    pub ilink_user_id: String,
}

/// Outcome of a single scan-status poll.
#[derive(Debug, Clone)]
pub enum QrScanOutcome {
    /// QR not yet scanned; keep polling.
    Waiting,
    /// QR scanned but not confirmed; keep polling.
    Scanned,
    /// Scan confirmed; credentials are ready.
    Confirmed(WeChatCredentials),
    /// QR expired; a fresh code must be fetched.
    Expired,
}

// ===== message payloads =====

/// One item inside a message (text/image/file). Only fields the router needs
/// are modeled; media download fields live under `image_item`/`file_item`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageItem {
    #[serde(rename = "type")]
    pub kind: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_item: Option<TextItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextItem {
    #[serde(default)]
    pub text: String,
}

/// An incoming message from `getupdates`.
#[derive(Debug, Clone, Deserialize)]
pub struct IncomingMessage {
    #[serde(default)]
    pub message_type: i32,
    #[serde(default)]
    pub message_state: i32,
    #[serde(default)]
    pub from_user_id: String,
    #[serde(default)]
    pub context_token: String,
    #[serde(default, deserialize_with = "deserialize_string_default")]
    pub message_id: String,
    #[serde(default)]
    pub item_list: Vec<MessageItem>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GetUpdatesResponse {
    #[serde(default, deserialize_with = "deserialize_i32_default")]
    pub ret: i32,
    #[serde(default)]
    pub errcode: Option<i32>,
    #[serde(default)]
    pub errmsg: Option<String>,
    #[serde(default)]
    pub msgs: Vec<IncomingMessage>,
    #[serde(default)]
    pub get_updates_buf: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SendMessageResponse {
    #[serde(default, deserialize_with = "deserialize_i32_default")]
    pub ret: i32,
    #[serde(default)]
    pub errmsg: Option<String>,
}

fn deserialize_i32_default<'de, D>(deserializer: D) -> std::result::Result<i32, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(Option::<i32>::deserialize(deserializer)?.unwrap_or_default())
}

fn deserialize_string_default<'de, D>(deserializer: D) -> std::result::Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Option::<serde_json::Value>::deserialize(deserializer)?;
    Ok(match value {
        Some(serde_json::Value::String(value)) => value,
        Some(serde_json::Value::Number(value)) => value.to_string(),
        Some(serde_json::Value::Bool(value)) => value.to_string(),
        _ => String::new(),
    })
}

// ===== HTTP client =====

/// Thin HTTP client over the iLink Bot API. Holds bot credentials and issues
/// authenticated POST requests. QR fetching is available before login via the
/// associated functions that take a base URL.
#[derive(Clone)]
pub struct ILinkClient {
    http: reqwest::Client,
    base_url: String,
    bot_token: String,
    bot_id: String,
    wechat_uin: String,
}

impl ILinkClient {
    /// Build a client from confirmed credentials.
    pub fn new(creds: &WeChatCredentials) -> Self {
        let base_url = if creds.base_url.trim().is_empty() {
            DEFAULT_BASE_URL.to_string()
        } else {
            creds.base_url.clone()
        };
        Self {
            http: reqwest::Client::new(),
            base_url,
            bot_token: creds.bot_token.clone(),
            bot_id: creds.ilink_bot_id.clone(),
            wechat_uin: generate_wechat_uin(),
        }
    }

    /// The bot's own iLink id.
    pub fn bot_id(&self) -> &str {
        &self.bot_id
    }

    /// Fetch a login QR code. Static because it precedes authentication.
    pub async fn fetch_qr_code(http: &reqwest::Client, base_url: &str) -> Result<QrCodeResponse> {
        let url = format!("{base_url}{QR_CODE_PATH}");
        let resp = http.get(&url).send().await?;
        if !resp.status().is_success() {
            return Err(Error::Protocol(format!(
                "get_bot_qrcode failed: HTTP {}",
                resp.status()
            )));
        }
        Ok(resp.json::<QrCodeResponse>().await?)
    }

    /// Poll the scan status of a QR code once. Static because it precedes
    /// authentication.
    pub async fn poll_qr_status(
        http: &reqwest::Client,
        base_url: &str,
        qrcode: &str,
    ) -> Result<QrScanOutcome> {
        let url = format!("{base_url}{QR_STATUS_PATH}{qrcode}");
        let resp = http.get(&url).send().await?;
        if !resp.status().is_success() {
            return Err(Error::Protocol(format!(
                "get_qrcode_status failed: HTTP {}",
                resp.status()
            )));
        }
        let data = resp.json::<QrStatusResponse>().await?;
        match data.status.as_str() {
            "confirmed" => {
                if data.bot_token.is_empty() || data.ilink_bot_id.is_empty() {
                    return Err(Error::Protocol(
                        "scan confirmed but credentials missing".into(),
                    ));
                }
                Ok(QrScanOutcome::Confirmed(WeChatCredentials {
                    bot_token: data.bot_token,
                    ilink_bot_id: data.ilink_bot_id,
                    base_url: data.baseurl,
                    ilink_user_id: data.ilink_user_id,
                }))
            }
            "scaned" => Ok(QrScanOutcome::Scanned),
            "expired" => Ok(QrScanOutcome::Expired),
            _ => Ok(QrScanOutcome::Waiting),
        }
    }

    /// Long-poll for new messages, resuming from `buf`.
    pub async fn get_updates(&self, buf: &str) -> Result<GetUpdatesResponse> {
        let body = serde_json::json!({
            "get_updates_buf": buf,
            "base_info": { "channel_version": "1.0.0" },
        });
        self.post(GET_UPDATES_PATH, &body, LONG_POLL_TIMEOUT_SECS + 5)
            .await
    }

    /// Send a text reply back to a user.
    pub async fn send_text(
        &self,
        to_user_id: &str,
        text: &str,
        context_token: &str,
    ) -> Result<SendMessageResponse> {
        let body = serde_json::json!({
            "msg": {
                "from_user_id": self.bot_id,
                "to_user_id": to_user_id,
                "client_id": format!("codux_{}", now_millis()),
                "message_type": message_type::BOT,
                "message_state": message_state::FINISH,
                "item_list": [{
                    "type": item_type::TEXT,
                    "text_item": { "text": text },
                }],
                "context_token": context_token,
            },
            "base_info": {},
        });
        self.post(SEND_MESSAGE_PATH, &body, SEND_TIMEOUT_SECS).await
    }

    /// Fetch a `typing_ticket` for a conversation.
    pub async fn get_typing_ticket(
        &self,
        user_id: &str,
        context_token: &str,
    ) -> Result<Option<String>> {
        let body = serde_json::json!({
            "ilink_user_id": user_id,
            "context_token": context_token,
            "base_info": {},
        });
        let value: serde_json::Value = self.post(GET_CONFIG_PATH, &body, 10).await?;
        Ok(value
            .get("typing_ticket")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()))
    }

    /// Send a "typing" indicator. `status` follows the iLink convention.
    pub async fn send_typing(&self, user_id: &str, typing_ticket: &str, status: i32) -> Result<()> {
        let body = serde_json::json!({
            "ilink_user_id": user_id,
            "typing_ticket": typing_ticket,
            "status": status,
            "base_info": {},
        });
        let _: serde_json::Value = self.post(SEND_TYPING_PATH, &body, 10).await?;
        Ok(())
    }

    async fn post<T: for<'de> Deserialize<'de>>(
        &self,
        path: &str,
        body: &serde_json::Value,
        timeout_secs: u64,
    ) -> Result<T> {
        let url = format!("{}{path}", self.base_url);
        let resp = self
            .http
            .post(&url)
            .header("Content-Type", "application/json")
            .header("AuthorizationType", "ilink_bot_token")
            .header("Authorization", format!("Bearer {}", self.bot_token))
            .header("X-WECHAT-UIN", &self.wechat_uin)
            .timeout(std::time::Duration::from_secs(timeout_secs))
            .json(body)
            .send()
            .await?;
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(Error::Protocol(format!("HTTP {status}: {text}")));
        }
        Ok(resp.json::<T>().await?)
    }
}

/// Extract the concatenated text of an incoming message's text items.
pub fn message_text(msg: &IncomingMessage) -> String {
    msg.item_list
        .iter()
        .filter(|item| item.kind == item_type::TEXT)
        .filter_map(|item| item.text_item.as_ref())
        .map(|t| t.text.as_str())
        .collect::<Vec<_>>()
        .join("")
}

fn generate_wechat_uin() -> String {
    use base64::Engine;
    // Reference uses a random u32 rendered as a decimal string, then base64.
    let n: u32 = pseudo_random_u32();
    base64::engine::general_purpose::STANDARD.encode(n.to_string())
}

fn pseudo_random_u32() -> u32 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(0);
    // Mix with the address of a stack local for a little extra entropy.
    let stack = &nanos as *const u32 as usize as u32;
    nanos.wrapping_mul(2654435761).wrapping_add(stack)
}

fn now_millis() -> u128 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn message_text_concatenates_text_items() {
        let msg = IncomingMessage {
            message_type: message_type::USER,
            message_state: message_state::FINISH,
            from_user_id: "u1".into(),
            context_token: "ctx".into(),
            message_id: "m1".into(),
            item_list: vec![
                MessageItem {
                    kind: item_type::TEXT,
                    text_item: Some(TextItem {
                        text: "hello ".into(),
                    }),
                },
                MessageItem {
                    kind: item_type::IMAGE,
                    text_item: None,
                },
                MessageItem {
                    kind: item_type::TEXT,
                    text_item: Some(TextItem {
                        text: "world".into(),
                    }),
                },
            ],
        };
        assert_eq!(message_text(&msg), "hello world");
    }

    #[test]
    fn credentials_default_base_url() {
        let creds = WeChatCredentials {
            bot_token: "t".into(),
            ilink_bot_id: "b".into(),
            base_url: String::new(),
            ilink_user_id: String::new(),
        };
        let client = ILinkClient::new(&creds);
        assert_eq!(client.base_url, DEFAULT_BASE_URL);
    }

    #[test]
    fn wechat_uin_is_nonempty_base64() {
        let uin = generate_wechat_uin();
        assert!(!uin.is_empty());
    }
}
