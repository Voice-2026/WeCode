//! Chat-platform bridges for WeCode.
//!
//! A bridge connects an external messaging platform (WeChat via the official
//! iLink Bot API, DingTalk, Feishu, ...) to a WeCode host so a user can drive a
//! terminal/agent session from a chat app. This crate owns only the
//! platform-protocol layer and message routing; it never depends on GPUI or
//! Flutter and holds no UI state.
//!
//! WeChat is implemented first. Its wire protocol is a faithful Rust port of
//! the reference `ilinkai.weixin.qq.com` flow: QR login, long-poll
//! `getupdates`, `sendmessage`, `sendtyping`, and AES-128-ECB media decryption.

pub mod binding;
pub mod runtime;
pub mod wechat;

/// Result alias for bridge operations.
pub type Result<T> = std::result::Result<T, Error>;

/// Errors surfaced by chat-platform bridges.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("protocol error: {0}")]
    Protocol(String),
    #[error("crypto error: {0}")]
    Crypto(String),
    #[error("login cancelled")]
    Cancelled,
}
