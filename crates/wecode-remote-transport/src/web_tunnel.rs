use serde::{Deserialize, Serialize};
use tokio::io::{AsyncRead, AsyncWrite};

pub const WECODE_WEB_TUNNEL_ALPN: &[u8] = b"/wecode/web-tunnel/1";
pub(crate) const WEB_TUNNEL_KIND_TCP_CONNECT: &str = "tcpConnect";

pub trait WebTunnelIoStream: AsyncRead + AsyncWrite + Unpin + Send {}

impl<T> WebTunnelIoStream for T where T: AsyncRead + AsyncWrite + Unpin + Send {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WebTunnelResponse {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
    pub error: Option<String>,
}

impl WebTunnelResponse {
    pub fn error(status: u16, message: impl Into<String>) -> Self {
        Self {
            status,
            headers: vec![(
                "content-type".to_string(),
                "text/plain; charset=utf-8".to_string(),
            )],
            body: Vec::new(),
            error: Some(message.into()),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WebTunnelTcpConnectRequest {
    pub device_id: String,
    pub device_token: String,
    pub host: String,
    pub port: u16,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WebTunnelRequestEnvelope {
    pub kind: String,
    pub device_id: String,
    pub device_token: String,
    pub target_host: String,
    pub target_port: u16,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WebTunnelResponseEnvelope {
    pub status: u16,
    #[serde(default)]
    pub headers: Vec<WebTunnelHeader>,
    #[serde(default)]
    pub body_len: u64,
    #[serde(default)]
    pub error: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WebTunnelHeader {
    pub name: String,
    pub value: String,
}

impl From<(String, String)> for WebTunnelHeader {
    fn from((name, value): (String, String)) -> Self {
        Self { name, value }
    }
}

impl From<WebTunnelHeader> for (String, String) {
    fn from(header: WebTunnelHeader) -> Self {
        (header.name, header.value)
    }
}
