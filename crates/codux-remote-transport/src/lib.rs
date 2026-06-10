use async_trait::async_trait;
pub use codux_protocol::RemoteTransportKind;
use codux_protocol::{RemoteEnvelope, RemoteOutgoingEnvelope, RemoteTransportPairingRequest};
use futures_util::{SinkExt, StreamExt};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message as WebSocketMessage;
use webrtc::api::APIBuilder;
use webrtc::api::media_engine::MediaEngine;
use webrtc::data_channel::RTCDataChannel;
use webrtc::data_channel::data_channel_message::DataChannelMessage;
use webrtc::ice_transport::ice_candidate::RTCIceCandidateInit;
use webrtc::ice_transport::ice_server::RTCIceServer;
use webrtc::peer_connection::RTCPeerConnection;
use webrtc::peer_connection::configuration::RTCConfiguration;
use webrtc::peer_connection::peer_connection_state::RTCPeerConnectionState;
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;

pub const GLOBAL_RELAY_SERVER_URL: &str = "https://codux-node.dux.plus";
pub const CHINA_RELAY_SERVER_URL: &str = "https://codux-service.dux.plus";
pub const DEFAULT_RELAY_SERVER_URL: &str = GLOBAL_RELAY_SERVER_URL;

pub type RemoteTransportMessageHandler = Arc<dyn Fn(String, Vec<u8>) + Send + Sync + 'static>;
pub type RemoteTransportStateHandler = Arc<dyn Fn(String, String) + Send + Sync + 'static>;
pub type RemoteTransportPairingHandler =
    Arc<dyn Fn(RemoteTransportPairingRequest) + Send + Sync + 'static>;
pub type RemoteTransportControlHandler =
    Arc<dyn Fn(String, RemoteEnvelope) -> bool + Send + Sync + 'static>;
pub type RemoteTransportLogHandler = Arc<dyn Fn(String) + Send + Sync + 'static>;

#[async_trait]
pub trait RemoteTransport: Send + Sync {
    fn kind(&self) -> RemoteTransportKind;
    fn send(&self, data: Vec<u8>, device_id: Option<&str>) -> bool;
    async fn shutdown(&self);
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RemoteControllerTransportConfig {
    pub server_url: String,
    pub host_id: String,
    pub device_id: String,
    pub device_token: String,
    pub transports: Vec<RemoteTransportCandidate>,
    pub stun_urls: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RemoteTransportCandidate {
    pub kind: String,
    pub url: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RemoteHostTransportConfig {
    pub server_url: String,
    pub host_id: String,
    pub host_token: String,
    pub stun_urls: Vec<String>,
}

pub struct RemoteTransportFactory;

impl RemoteTransportFactory {
    pub async fn connect_host(
        config: &RemoteHostTransportConfig,
        on_message: RemoteTransportMessageHandler,
        on_state: RemoteTransportStateHandler,
        on_pairing: RemoteTransportPairingHandler,
        on_log: Option<RemoteTransportLogHandler>,
    ) -> Result<Arc<dyn RemoteTransport>, String> {
        let relay = remote_server_url(&config.server_url);
        let ws_url = remote_url(
            &relay,
            "/ws/host",
            &[
                ("hostId", config.host_id.as_str()),
                ("token", config.host_token.as_str()),
            ],
            true,
        )?;
        let transport = RemoteWebRtcHostTransport::connect(
            config, ws_url, on_message, on_state, on_pairing, on_log,
        )
        .await?;
        Ok(transport)
    }

    pub async fn connect_controller(
        config: &RemoteControllerTransportConfig,
        on_message: RemoteTransportMessageHandler,
        on_state: RemoteTransportStateHandler,
        on_log: Option<RemoteTransportLogHandler>,
    ) -> Result<Arc<dyn RemoteTransport>, String> {
        let kind = preferred_controller_transport_kind(
            config
                .transports
                .iter()
                .map(|candidate| (candidate.kind.as_str(), candidate.url.as_str())),
        );
        let relay = config
            .transports
            .iter()
            .find(|candidate| {
                candidate.kind == "websocketRelay" && !candidate.url.trim().is_empty()
            })
            .map(|candidate| candidate.url.as_str())
            .unwrap_or(config.server_url.as_str());
        let ws_url = remote_client_websocket_url(
            relay,
            &config.host_id,
            &config.device_id,
            Some(&config.device_token),
        )?;
        let relay_transport =
            RemoteWebSocketControllerTransport::connect(ws_url, on_message, on_state, on_log)
                .await?;
        match kind {
            "webRtc" => Ok(Arc::new(RemoteControllerCompositeTransport {
                relay: relay_transport,
                kind: RemoteTransportKind::WebRtc,
            })),
            "websocketRelay" => Ok(relay_transport),
            _ => Err("missing supported controller transport candidate".to_string()),
        }
    }
}

pub fn remote_relay_url_for_preset(preset: &str, custom_url: &str) -> String {
    match preset.trim() {
        "global" => GLOBAL_RELAY_SERVER_URL.to_string(),
        "china" => CHINA_RELAY_SERVER_URL.to_string(),
        "" => GLOBAL_RELAY_SERVER_URL.to_string(),
        "custom" => custom_url.trim().to_string(),
        _ => custom_url.trim().to_string(),
    }
}

pub fn remote_relay_preset_for_url(url: &str) -> String {
    let normalized = remote_server_url(url);
    if normalized == remote_server_url(GLOBAL_RELAY_SERVER_URL) || url.trim().is_empty() {
        "global".to_string()
    } else if normalized == remote_server_url(CHINA_RELAY_SERVER_URL) {
        "china".to_string()
    } else {
        "custom".to_string()
    }
}

pub fn remote_server_url(value: &str) -> String {
    let value = value.trim();
    let value = if value.is_empty() {
        DEFAULT_RELAY_SERVER_URL
    } else {
        value
    };
    with_protocol_path(value)
}

pub fn remote_stun_urls() -> Vec<String> {
    vec![
        "stun:stun.miwifi.com:3478".to_string(),
        "stun:stun.l.google.com:19302".to_string(),
    ]
}

pub fn remote_url(
    base: &str,
    path: &str,
    query: &[(&str, &str)],
    websocket: bool,
) -> Result<String, String> {
    let mut url = url::Url::parse(base.trim()).map_err(|error| error.to_string())?;
    url.set_path(&join_url_path(url.path(), path));
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
    if !query.is_empty() {
        let mut pairs = url.query_pairs_mut();
        for (key, value) in query {
            pairs.append_pair(key, value);
        }
    }
    Ok(url.to_string())
}

pub fn remote_pairing_ticket_url(base: &str, ticket: &str) -> Result<String, String> {
    let base = remote_server_url(base);
    remote_url(
        &base,
        &format!("/api/tickets/{}", ticket.trim()),
        &[],
        false,
    )
}

pub fn remote_client_websocket_url(
    base: &str,
    host_id: &str,
    device_id: &str,
    token: Option<&str>,
) -> Result<String, String> {
    let base = remote_server_url(base);
    let mut query = vec![("hostId", host_id), ("deviceId", device_id)];
    if let Some(token) = token.filter(|value| !value.trim().is_empty()) {
        query.push(("token", token));
    }
    remote_url(&base, "/ws/client", &query, true)
}

pub fn remote_pairing_websocket_url(
    base: &str,
    host_id: &str,
    device_public_key: &str,
) -> Result<String, String> {
    let base = remote_server_url(base);
    remote_url(
        &base,
        "/ws/client",
        &[("hostId", host_id), ("deviceId", device_public_key)],
        true,
    )
}

pub fn preferred_controller_transport_kind<'a>(
    candidates: impl IntoIterator<Item = (&'a str, &'a str)>,
) -> &'static str {
    let mut has_relay = false;
    let mut has_webrtc = false;
    for (kind, url) in candidates {
        if kind == "webRtc" && !url.trim().is_empty() {
            has_webrtc = true;
        }
        if kind == "websocketRelay" && !url.trim().is_empty() {
            has_relay = true;
        }
    }
    if has_relay && has_webrtc {
        "webRtc"
    } else if has_relay {
        "websocketRelay"
    } else {
        ""
    }
}

pub fn preferred_pairing_transport_kind<'a>(
    candidates: impl IntoIterator<Item = (&'a str, &'a str)>,
) -> &'static str {
    let mut has_webrtc = false;
    for (kind, url) in candidates {
        if kind == "websocketRelay" && !url.trim().is_empty() {
            return "websocketRelay";
        }
        if kind == "webRtc" && !url.trim().is_empty() {
            has_webrtc = true;
        }
    }
    if has_webrtc { "webRtc" } else { "" }
}

pub struct RemoteWebSocketHostTransport {
    tx: Mutex<Option<mpsc::UnboundedSender<String>>>,
    on_message: RemoteTransportMessageHandler,
    on_state: RemoteTransportStateHandler,
    on_pairing: RemoteTransportPairingHandler,
    on_control: Option<RemoteTransportControlHandler>,
    on_log: Option<RemoteTransportLogHandler>,
}

impl RemoteWebSocketHostTransport {
    pub async fn connect(
        ws_url: String,
        on_message: RemoteTransportMessageHandler,
        on_state: RemoteTransportStateHandler,
        on_pairing: RemoteTransportPairingHandler,
        on_control: Option<RemoteTransportControlHandler>,
        on_log: Option<RemoteTransportLogHandler>,
    ) -> Result<Arc<Self>, String> {
        let (socket, _) = tokio_tungstenite::connect_async(&ws_url)
            .await
            .map_err(|error| error.to_string())?;
        let (mut write, mut read) = socket.split();
        let (tx, mut rx) = mpsc::unbounded_channel::<String>();
        let transport = Arc::new(Self {
            tx: Mutex::new(Some(tx)),
            on_message,
            on_state,
            on_pairing,
            on_control,
            on_log,
        });

        let writer = Arc::clone(&transport);
        tokio::spawn(async move {
            while let Some(message) = rx.recv().await {
                if write
                    .send(WebSocketMessage::Text(message.into()))
                    .await
                    .is_err()
                {
                    break;
                }
            }
            writer.close_sender();
        });

        let reader = Arc::clone(&transport);
        tokio::spawn(async move {
            while let Some(message) = read.next().await {
                match message {
                    Ok(WebSocketMessage::Text(text)) => {
                        reader.handle_text(text.to_string());
                    }
                    Ok(WebSocketMessage::Close(_)) => break,
                    Ok(_) => {}
                    Err(error) => {
                        reader.log(format!("websocket_recv failed error={error}"));
                        break;
                    }
                }
            }
            reader.close_sender();
            (reader.on_state)(String::new(), "closed".to_string());
        });

        Ok(transport)
    }

    fn handle_text(&self, text: String) {
        let Ok(raw) = serde_json::from_str::<RemoteEnvelope>(&text) else {
            self.log("websocket_recv drop reason=decode".to_string());
            return;
        };
        if raw.kind == "pairing.request" {
            if let Some(handshake) = pairing_handshake_from_envelope(&raw) {
                (self.on_pairing)(handshake);
            }
        }
        let device_id = raw
            .device_id
            .clone()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_default();
        if !device_id.is_empty() {
            (self.on_state)(device_id.clone(), "connected".to_string());
        }
        if self
            .on_control
            .as_ref()
            .map(|handler| handler(device_id.clone(), raw))
            .unwrap_or(false)
        {
            return;
        }
        (self.on_message)(device_id, text.into_bytes());
    }

    fn close_sender(&self) {
        if let Ok(mut tx) = self.tx.lock() {
            *tx = None;
        }
    }

    fn log(&self, message: String) {
        if let Some(on_log) = self.on_log.as_ref() {
            on_log(message);
        }
    }
}

#[async_trait]
impl RemoteTransport for RemoteWebSocketHostTransport {
    fn kind(&self) -> RemoteTransportKind {
        RemoteTransportKind::WebSocketRelay
    }

    fn send(&self, data: Vec<u8>, _device_id: Option<&str>) -> bool {
        let Ok(text) = String::from_utf8(data) else {
            return false;
        };
        self.tx
            .lock()
            .ok()
            .and_then(|tx| tx.clone())
            .map(|tx| tx.send(text).is_ok())
            .unwrap_or(false)
    }

    async fn shutdown(&self) {
        self.close_sender();
    }
}

pub struct RemoteWebSocketControllerTransport {
    tx: Mutex<Option<mpsc::UnboundedSender<String>>>,
    on_message: RemoteTransportMessageHandler,
    on_state: RemoteTransportStateHandler,
    on_log: Option<RemoteTransportLogHandler>,
}

impl RemoteWebSocketControllerTransport {
    pub async fn connect(
        ws_url: String,
        on_message: RemoteTransportMessageHandler,
        on_state: RemoteTransportStateHandler,
        on_log: Option<RemoteTransportLogHandler>,
    ) -> Result<Arc<Self>, String> {
        let (socket, _) = tokio_tungstenite::connect_async(&ws_url)
            .await
            .map_err(|error| error.to_string())?;
        let (mut write, mut read) = socket.split();
        let (tx, mut rx) = mpsc::unbounded_channel::<String>();
        let transport = Arc::new(Self {
            tx: Mutex::new(Some(tx)),
            on_message,
            on_state,
            on_log,
        });

        let writer = Arc::clone(&transport);
        tokio::spawn(async move {
            while let Some(message) = rx.recv().await {
                if write
                    .send(WebSocketMessage::Text(message.into()))
                    .await
                    .is_err()
                {
                    break;
                }
            }
            writer.close_sender();
        });

        let reader = Arc::clone(&transport);
        tokio::spawn(async move {
            (reader.on_state)(String::new(), "connected".to_string());
            while let Some(message) = read.next().await {
                match message {
                    Ok(WebSocketMessage::Text(text)) => {
                        (reader.on_message)(String::new(), text.to_string().into_bytes());
                    }
                    Ok(WebSocketMessage::Close(_)) => break,
                    Ok(_) => {}
                    Err(error) => {
                        reader.log(format!("controller_websocket_recv failed error={error}"));
                        break;
                    }
                }
            }
            reader.close_sender();
            (reader.on_state)(String::new(), "closed".to_string());
        });

        Ok(transport)
    }

    fn close_sender(&self) {
        if let Ok(mut tx) = self.tx.lock() {
            *tx = None;
        }
    }

    fn log(&self, message: String) {
        if let Some(on_log) = self.on_log.as_ref() {
            on_log(message);
        }
    }
}

#[async_trait]
impl RemoteTransport for RemoteWebSocketControllerTransport {
    fn kind(&self) -> RemoteTransportKind {
        RemoteTransportKind::WebSocketRelay
    }

    fn send(&self, data: Vec<u8>, _device_id: Option<&str>) -> bool {
        let Ok(text) = String::from_utf8(data) else {
            return false;
        };
        self.tx
            .lock()
            .ok()
            .and_then(|tx| tx.clone())
            .map(|tx| tx.send(text).is_ok())
            .unwrap_or(false)
    }

    async fn shutdown(&self) {
        self.close_sender();
    }
}

struct RemoteControllerCompositeTransport {
    relay: Arc<RemoteWebSocketControllerTransport>,
    kind: RemoteTransportKind,
}

#[async_trait]
impl RemoteTransport for RemoteControllerCompositeTransport {
    fn kind(&self) -> RemoteTransportKind {
        self.kind
    }

    fn send(&self, data: Vec<u8>, device_id: Option<&str>) -> bool {
        self.relay.send(data, device_id)
    }

    async fn shutdown(&self) {
        self.relay.shutdown().await;
    }
}

pub struct RemoteWebRtcHostTransport {
    relay: Mutex<Option<Arc<RemoteWebSocketHostTransport>>>,
    peers: Mutex<HashMap<String, Arc<WebRtcPeer>>>,
    ice_servers: Vec<String>,
    on_message: RemoteTransportMessageHandler,
    on_state: RemoteTransportStateHandler,
    on_log: Option<RemoteTransportLogHandler>,
}

#[derive(Clone, Default)]
pub struct LocalMemoryTransportHub {
    peers: Arc<Mutex<HashMap<String, LocalMemoryPeer>>>,
}

#[derive(Clone)]
struct LocalMemoryPeer {
    id: String,
    on_message: RemoteTransportMessageHandler,
    on_state: RemoteTransportStateHandler,
}

impl LocalMemoryTransportHub {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn connect(
        &self,
        id: impl Into<String>,
        kind: RemoteTransportKind,
        on_message: RemoteTransportMessageHandler,
        on_state: RemoteTransportStateHandler,
    ) -> Arc<LocalMemoryTransport> {
        let id = id.into();
        let peer = LocalMemoryPeer {
            id: id.clone(),
            on_message,
            on_state,
        };
        if let Ok(mut peers) = self.peers.lock() {
            peers.insert(id.clone(), peer.clone());
        }
        (peer.on_state)(id.clone(), "connected".to_string());
        Arc::new(LocalMemoryTransport {
            id,
            kind,
            hub: self.clone(),
        })
    }

    fn send_from(&self, source_id: &str, target_id: Option<&str>, data: Vec<u8>) -> bool {
        let targets = self
            .peers
            .lock()
            .map(|peers| {
                peers
                    .values()
                    .filter(|peer| {
                        if peer.id == source_id {
                            return false;
                        }
                        target_id.map(|target| peer.id == target).unwrap_or(true)
                    })
                    .cloned()
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        if targets.is_empty() {
            return false;
        }
        for peer in targets {
            (peer.on_message)(source_id.to_string(), data.clone());
        }
        true
    }

    fn disconnect(&self, id: &str) {
        let peer = self
            .peers
            .lock()
            .ok()
            .and_then(|mut peers| peers.remove(id));
        if let Some(peer) = peer {
            (peer.on_state)(peer.id, "closed".to_string());
        }
    }
}

pub struct LocalMemoryTransport {
    id: String,
    kind: RemoteTransportKind,
    hub: LocalMemoryTransportHub,
}

impl LocalMemoryTransport {
    pub fn id(&self) -> &str {
        &self.id
    }
}

#[async_trait]
impl RemoteTransport for LocalMemoryTransport {
    fn kind(&self) -> RemoteTransportKind {
        self.kind
    }

    fn send(&self, data: Vec<u8>, device_id: Option<&str>) -> bool {
        self.hub.send_from(&self.id, device_id, data)
    }

    async fn shutdown(&self) {
        self.hub.disconnect(&self.id);
    }
}

struct WebRtcPeer {
    pc: Arc<RTCPeerConnection>,
    dc: Mutex<Option<Arc<RTCDataChannel>>>,
}

impl RemoteWebRtcHostTransport {
    pub async fn connect(
        config: &RemoteHostTransportConfig,
        ws_url: String,
        on_message: RemoteTransportMessageHandler,
        on_state: RemoteTransportStateHandler,
        on_pairing: RemoteTransportPairingHandler,
        on_log: Option<RemoteTransportLogHandler>,
    ) -> Result<Arc<Self>, String> {
        let transport = Arc::new(Self {
            relay: Mutex::new(None),
            peers: Mutex::new(HashMap::new()),
            ice_servers: if config.stun_urls.is_empty() {
                remote_stun_urls()
            } else {
                config.stun_urls.clone()
            },
            on_message: Arc::clone(&on_message),
            on_state: Arc::clone(&on_state),
            on_log: on_log.clone(),
        });
        let weak = Arc::downgrade(&transport);
        let relay = RemoteWebSocketHostTransport::connect(
            ws_url,
            on_message,
            on_state,
            on_pairing,
            Some(Arc::new(move |device_id, envelope| {
                if !envelope.kind.starts_with("webrtc.") {
                    return false;
                }
                if let Some(transport) = weak.upgrade() {
                    tokio::spawn(async move {
                        transport.handle_signal(device_id, envelope).await;
                    });
                }
                true
            })),
            on_log,
        )
        .await?;
        if let Ok(mut current) = transport.relay.lock() {
            *current = Some(relay);
        }
        transport.log(format!("webrtc_transport ready host={}", config.host_id));
        Ok(transport)
    }

    async fn handle_signal(self: Arc<Self>, device_id: String, envelope: RemoteEnvelope) {
        if device_id.trim().is_empty() {
            return;
        }
        match envelope.kind.as_str() {
            "webrtc.offer" => {
                if let Err(error) = self.handle_offer(&device_id, envelope.payload).await {
                    self.log(format!(
                        "webrtc_offer failed device={device_id} error={error}"
                    ));
                    (self.on_state)(device_id, "path=relay".to_string());
                }
            }
            "webrtc.ice" => {
                if let Err(error) = self.handle_ice(&device_id, envelope.payload).await {
                    self.log(format!(
                        "webrtc_ice failed device={device_id} error={error}"
                    ));
                }
            }
            _ => {}
        }
    }

    async fn handle_offer(&self, device_id: &str, payload: Value) -> Result<(), String> {
        let description = payload
            .get("description")
            .cloned()
            .ok_or_else(|| "Missing WebRTC offer description.".to_string())
            .and_then(session_description_from_value)?;
        let peer = self.create_peer(device_id.to_string()).await?;
        peer.pc
            .set_remote_description(description)
            .await
            .map_err(|error| error.to_string())?;
        let answer = peer
            .pc
            .create_answer(None)
            .await
            .map_err(|error| error.to_string())?;
        let mut gathering_complete = peer.pc.gathering_complete_promise().await;
        peer.pc
            .set_local_description(answer)
            .await
            .map_err(|error| error.to_string())?;
        let _ = gathering_complete.recv().await;
        let description = peer
            .pc
            .local_description()
            .await
            .ok_or_else(|| "Missing WebRTC local answer.".to_string())?;
        self.send_signal(
            "webrtc.answer",
            Some(device_id),
            json!({ "description": description }),
        );
        Ok(())
    }

    async fn handle_ice(&self, device_id: &str, payload: Value) -> Result<(), String> {
        let candidate = payload
            .get("candidate")
            .cloned()
            .ok_or_else(|| "Missing WebRTC ICE candidate.".to_string())
            .and_then(|value| {
                serde_json::from_value::<RTCIceCandidateInit>(value)
                    .map_err(|error| error.to_string())
            })?;
        let peer = self
            .peers
            .lock()
            .ok()
            .and_then(|peers| peers.get(device_id).cloned())
            .ok_or_else(|| "Missing WebRTC peer.".to_string())?;
        peer.pc
            .add_ice_candidate(candidate)
            .await
            .map_err(|error| error.to_string())
    }

    async fn create_peer(&self, device_id: String) -> Result<Arc<WebRtcPeer>, String> {
        if let Some(peer) = self
            .peers
            .lock()
            .ok()
            .and_then(|peers| peers.get(&device_id).cloned())
        {
            let _ = peer.pc.close().await;
        }

        let mut media_engine = MediaEngine::default();
        media_engine
            .register_default_codecs()
            .map_err(|error| error.to_string())?;
        let api = APIBuilder::new().with_media_engine(media_engine).build();
        let pc = Arc::new(
            api.new_peer_connection(RTCConfiguration {
                ice_servers: vec![RTCIceServer {
                    urls: self.ice_servers.clone(),
                    ..Default::default()
                }],
                ..Default::default()
            })
            .await
            .map_err(|error| error.to_string())?,
        );
        let peer = Arc::new(WebRtcPeer {
            pc: Arc::clone(&pc),
            dc: Mutex::new(None),
        });
        let weak_peer = Arc::downgrade(&peer);
        let message_handler = Arc::clone(&self.on_message);
        let state_handler = Arc::clone(&self.on_state);
        let channel_device_id = device_id.clone();
        pc.on_data_channel(Box::new(move |dc: Arc<RTCDataChannel>| {
            let weak_peer = weak_peer.clone();
            let message_handler = Arc::clone(&message_handler);
            let state_handler = Arc::clone(&state_handler);
            let channel_device_id = channel_device_id.clone();
            Box::pin(async move {
                install_data_channel(
                    weak_peer,
                    dc,
                    channel_device_id,
                    message_handler,
                    state_handler,
                );
            })
        }));

        let state_device_id = device_id.clone();
        let state_handler = Arc::clone(&self.on_state);
        pc.on_peer_connection_state_change(Box::new(move |state| {
            let state_handler = Arc::clone(&state_handler);
            let state_device_id = state_device_id.clone();
            Box::pin(async move {
                if matches!(
                    state,
                    RTCPeerConnectionState::Failed
                        | RTCPeerConnectionState::Disconnected
                        | RTCPeerConnectionState::Closed
                ) {
                    state_handler(state_device_id, "path=relay".to_string());
                }
            })
        }));

        if let Ok(mut peers) = self.peers.lock() {
            peers.insert(device_id, Arc::clone(&peer));
        }
        Ok(peer)
    }

    fn send_signal(&self, kind: &str, device_id: Option<&str>, payload: Value) -> bool {
        let envelope = RemoteOutgoingEnvelope {
            kind: kind.to_string(),
            device_id: device_id.map(str::to_string),
            session_id: None,
            seq: None,
            payload,
        };
        let Ok(data) = serde_json::to_vec(&envelope) else {
            return false;
        };
        self.send_relay(data)
    }

    fn send_relay(&self, data: Vec<u8>) -> bool {
        let relay = self.relay.lock().ok().and_then(|value| value.clone());
        relay.map(|relay| relay.send(data, None)).unwrap_or(false)
    }

    fn log(&self, message: String) {
        if let Some(on_log) = self.on_log.as_ref() {
            on_log(message);
        }
    }
}

#[async_trait]
impl RemoteTransport for RemoteWebRtcHostTransport {
    fn kind(&self) -> RemoteTransportKind {
        RemoteTransportKind::WebRtc
    }

    fn send(&self, data: Vec<u8>, device_id: Option<&str>) -> bool {
        if let Some(device_id) = device_id {
            let channel = self
                .peers
                .lock()
                .ok()
                .and_then(|peers| peers.get(device_id).cloned())
                .and_then(|peer| peer.dc.lock().ok().and_then(|dc| dc.clone()));
            if let Some(channel) = channel {
                if let Ok(text) = String::from_utf8(data.clone()) {
                    let channel = Arc::clone(&channel);
                    tokio::spawn(async move {
                        let _ = channel.send_text(text).await;
                    });
                    return true;
                }
            }
        }
        self.send_relay(data)
    }

    async fn shutdown(&self) {
        let relay = self.relay.lock().ok().and_then(|mut value| value.take());
        if let Some(relay) = relay {
            relay.shutdown().await;
        }
        let peers = self
            .peers
            .lock()
            .map(|mut peers| peers.drain().map(|(_, peer)| peer).collect::<Vec<_>>())
            .unwrap_or_default();
        for peer in peers {
            let _ = peer.pc.close().await;
        }
    }
}

fn install_data_channel(
    weak_peer: std::sync::Weak<WebRtcPeer>,
    dc: Arc<RTCDataChannel>,
    device_id: String,
    on_message: RemoteTransportMessageHandler,
    on_state: RemoteTransportStateHandler,
) {
    if let Some(peer) = weak_peer.upgrade() {
        if let Ok(mut current) = peer.dc.lock() {
            *current = Some(Arc::clone(&dc));
        }
    }
    let open_device_id = device_id.clone();
    let open_state = Arc::clone(&on_state);
    dc.on_open(Box::new(move || {
        let open_state = Arc::clone(&open_state);
        let open_device_id = open_device_id.clone();
        Box::pin(async move {
            open_state(open_device_id, "path=direct".to_string());
        })
    }));
    let close_device_id = device_id.clone();
    let close_state = Arc::clone(&on_state);
    dc.on_close(Box::new(move || {
        let close_state = Arc::clone(&close_state);
        let close_device_id = close_device_id.clone();
        Box::pin(async move {
            close_state(close_device_id, "path=relay".to_string());
        })
    }));
    dc.on_message(Box::new(move |message: DataChannelMessage| {
        let on_message = Arc::clone(&on_message);
        let device_id = device_id.clone();
        Box::pin(async move {
            on_message(device_id, message.data.to_vec());
        })
    }));
}

fn pairing_handshake_from_envelope(
    envelope: &RemoteEnvelope,
) -> Option<RemoteTransportPairingRequest> {
    let device_id = envelope
        .device_id
        .clone()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| {
            envelope
                .payload
                .get("deviceId")
                .and_then(Value::as_str)
                .map(str::to_string)
        })?;
    let device_name = envelope
        .payload
        .get("deviceName")
        .and_then(Value::as_str)
        .unwrap_or("Mobile Device")
        .to_string();
    let device_public_key = envelope
        .payload
        .get("devicePublicKey")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    Some(RemoteTransportPairingRequest {
        device_id,
        device_name,
        device_public_key,
        pairing_id: envelope
            .payload
            .get("pairingId")
            .and_then(Value::as_str)
            .map(str::to_string),
        pairing_code: envelope
            .payload
            .get("code")
            .and_then(Value::as_str)
            .map(str::to_string),
        pairing_secret: envelope
            .payload
            .get("secret")
            .and_then(Value::as_str)
            .map(str::to_string),
    })
}

fn session_description_from_value(value: Value) -> Result<RTCSessionDescription, String> {
    let kind = value
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let sdp = value
        .get("sdp")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    match kind.as_str() {
        "offer" => RTCSessionDescription::offer(sdp).map_err(|error| error.to_string()),
        "answer" => RTCSessionDescription::answer(sdp).map_err(|error| error.to_string()),
        "pranswer" => RTCSessionDescription::pranswer(sdp).map_err(|error| error.to_string()),
        _ => Err("Unsupported WebRTC session description.".to_string()),
    }
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

fn with_protocol_path(value: &str) -> String {
    let Ok(mut url) = url::Url::parse(value) else {
        return value.to_string();
    };
    if url.path().trim_matches('/').is_empty() {
        url.set_path("/v3");
    }
    url.to_string().trim_end_matches('/').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex as StdMutex;

    #[test]
    fn relay_urls_use_v3_prefix_for_plain_domains() {
        assert_eq!(
            remote_server_url("https://codux-service.dux.plus"),
            "https://codux-service.dux.plus/v3"
        );
        assert_eq!(
            remote_server_url("https://codux-service.dux.plus/v3"),
            "https://codux-service.dux.plus/v3"
        );
    }

    #[test]
    fn websocket_url_joins_protocol_path() {
        let relay = remote_server_url("https://relay.example");
        assert_eq!(
            remote_url(
                &relay,
                "/ws/host",
                &[("hostId", "h1"), ("token", "t1")],
                true
            )
            .unwrap(),
            "wss://relay.example/v3/ws/host?hostId=h1&token=t1"
        );
    }

    #[test]
    fn controller_urls_are_built_from_shared_rules() {
        assert_eq!(
            remote_pairing_ticket_url("https://relay.example", "ticket-1").unwrap(),
            "https://relay.example/v3/api/tickets/ticket-1"
        );
        assert_eq!(
            remote_pairing_websocket_url("https://relay.example", "host-1", "device-key").unwrap(),
            "wss://relay.example/v3/ws/client?hostId=host-1&deviceId=device-key"
        );
        assert_eq!(
            remote_client_websocket_url("https://relay.example", "host-1", "device-1", Some("t"))
                .unwrap(),
            "wss://relay.example/v3/ws/client?hostId=host-1&deviceId=device-1&token=t"
        );
    }

    #[test]
    fn preferred_transport_helpers_match_pairing_and_controller_order() {
        let candidates = [
            ("websocketRelay", "https://relay.example"),
            ("webRtc", "https://relay.example"),
        ];
        assert_eq!(
            preferred_pairing_transport_kind(candidates),
            "websocketRelay"
        );
        assert_eq!(preferred_controller_transport_kind(candidates), "webRtc");
        assert_eq!(
            preferred_controller_transport_kind([("webRtc", "https://relay.example")]),
            ""
        );
    }

    #[test]
    fn controller_config_uses_relay_only_when_direct_lacks_signaling() {
        let relay_only = RemoteControllerTransportConfig {
            server_url: "https://relay.example".to_string(),
            host_id: "host-1".to_string(),
            device_id: "device-1".to_string(),
            transports: vec![RemoteTransportCandidate {
                kind: "websocketRelay".to_string(),
                url: "https://relay.example/v3".to_string(),
            }],
            ..Default::default()
        };
        assert_eq!(
            preferred_controller_transport_kind(
                relay_only
                    .transports
                    .iter()
                    .map(|candidate| (candidate.kind.as_str(), candidate.url.as_str()))
            ),
            "websocketRelay"
        );

        let direct_without_relay = RemoteControllerTransportConfig {
            transports: vec![RemoteTransportCandidate {
                kind: "webRtc".to_string(),
                url: "https://relay.example/v3".to_string(),
            }],
            ..relay_only
        };
        assert_eq!(
            preferred_controller_transport_kind(
                direct_without_relay
                    .transports
                    .iter()
                    .map(|candidate| (candidate.kind.as_str(), candidate.url.as_str()))
            ),
            ""
        );
    }

    #[tokio::test]
    async fn local_memory_transport_broadcasts_and_targets_messages() {
        let hub = LocalMemoryTransportHub::new();
        let received_a = Arc::new(StdMutex::new(Vec::<String>::new()));
        let received_b = Arc::new(StdMutex::new(Vec::<String>::new()));
        let state_a = Arc::new(StdMutex::new(Vec::<String>::new()));
        let state_b = Arc::new(StdMutex::new(Vec::<String>::new()));

        let a = hub.connect(
            "a",
            RemoteTransportKind::WebSocketRelay,
            {
                let received = Arc::clone(&received_a);
                Arc::new(move |source, data| {
                    received
                        .lock()
                        .unwrap()
                        .push(format!("{source}:{}", String::from_utf8(data).unwrap()));
                })
            },
            {
                let state = Arc::clone(&state_a);
                Arc::new(move |peer, status| {
                    state.lock().unwrap().push(format!("{peer}:{status}"));
                })
            },
        );
        let b = hub.connect(
            "b",
            RemoteTransportKind::WebSocketRelay,
            {
                let received = Arc::clone(&received_b);
                Arc::new(move |source, data| {
                    received
                        .lock()
                        .unwrap()
                        .push(format!("{source}:{}", String::from_utf8(data).unwrap()));
                })
            },
            {
                let state = Arc::clone(&state_b);
                Arc::new(move |peer, status| {
                    state.lock().unwrap().push(format!("{peer}:{status}"));
                })
            },
        );

        assert!(a.send(b"hello".to_vec(), None));
        assert_eq!(received_b.lock().unwrap().as_slice(), ["a:hello"]);
        assert!(b.send(b"direct".to_vec(), Some("a")));
        assert_eq!(received_a.lock().unwrap().as_slice(), ["b:direct"]);
        assert!(!a.send(b"missing".to_vec(), Some("missing")));
        b.shutdown().await;
        assert_eq!(
            state_b.lock().unwrap().last().map(String::as_str),
            Some("b:closed")
        );
    }

    #[tokio::test]
    async fn local_memory_transport_stops_routing_after_shutdown() {
        let hub = LocalMemoryTransportHub::new();
        let received = Arc::new(StdMutex::new(Vec::<String>::new()));
        let sender = hub.connect(
            "sender",
            RemoteTransportKind::WebSocketRelay,
            Arc::new(|_, _| {}),
            Arc::new(|_, _| {}),
        );
        let receiver = hub.connect(
            "receiver",
            RemoteTransportKind::WebSocketRelay,
            {
                let received = Arc::clone(&received);
                Arc::new(move |source, data| {
                    received
                        .lock()
                        .unwrap()
                        .push(format!("{source}:{}", String::from_utf8(data).unwrap()));
                })
            },
            Arc::new(|_, _| {}),
        );

        assert!(sender.send(b"before".to_vec(), Some("receiver")));
        receiver.shutdown().await;
        assert!(!sender.send(b"after".to_vec(), Some("receiver")));
        assert_eq!(received.lock().unwrap().as_slice(), ["sender:before"]);
    }

    #[test]
    fn remote_url_preserves_existing_base_path_and_escapes_query() {
        let url = remote_url(
            "https://relay.example/custom",
            "ws/client",
            &[("hostId", "host 1"), ("deviceId", "device+1")],
            true,
        )
        .unwrap();

        assert_eq!(
            url,
            "wss://relay.example/custom/ws/client?hostId=host+1&deviceId=device%2B1"
        );
    }

    #[test]
    fn relay_preset_round_trip_matches_default_servers() {
        assert_eq!(
            remote_relay_preset_for_url("https://codux-service.dux.plus"),
            "china"
        );
        assert_eq!(
            remote_relay_url_for_preset("global", ""),
            GLOBAL_RELAY_SERVER_URL
        );
    }
}
