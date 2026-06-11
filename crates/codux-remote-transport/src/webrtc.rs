use crate::control_messages::transport_pong_for_ping_bytes;
use crate::url_rules::remote_stun_urls;
use crate::websocket::{RemoteWebSocketControllerTransport, RemoteWebSocketHostTransport};
use crate::{
    RemoteHostTransportConfig, RemoteTransport, RemoteTransportLogHandler,
    RemoteTransportMessageHandler, RemoteTransportPairingHandler, RemoteTransportStateHandler,
};
use async_trait::async_trait;
use codux_protocol::{RemoteEnvelope, RemoteOutgoingEnvelope, RemoteTransportKind};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use webrtc::api::APIBuilder;
use webrtc::api::media_engine::MediaEngine;
use webrtc::data_channel::RTCDataChannel;
use webrtc::data_channel::data_channel_message::DataChannelMessage;
use webrtc::data_channel::data_channel_state::RTCDataChannelState;
use webrtc::ice_transport::ice_candidate::RTCIceCandidateInit;
use webrtc::ice_transport::ice_server::RTCIceServer;
use webrtc::peer_connection::RTCPeerConnection;
use webrtc::peer_connection::configuration::RTCConfiguration;
use webrtc::peer_connection::peer_connection_state::RTCPeerConnectionState;
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;

pub(crate) struct RemoteControllerCompositeTransport {
    relay: Arc<RemoteWebSocketControllerTransport>,
    pc: Arc<RTCPeerConnection>,
    dc: Mutex<Option<Arc<RTCDataChannel>>>,
    direct_tx: mpsc::UnboundedSender<ControllerDirectSend>,
    direct_route: DirectRouteState,
    relay_route: RelayRouteState,
    on_state: RemoteTransportStateHandler,
}

struct ControllerDirectSend {
    channel: Arc<RTCDataChannel>,
    text: String,
    fallback_data: Vec<u8>,
}

#[derive(Clone, Default)]
pub(crate) struct DirectRouteState {
    ready: Arc<Mutex<bool>>,
}

impl DirectRouteState {
    pub(crate) fn set_ready(&self, ready: bool) {
        if let Ok(mut current) = self.ready.lock() {
            *current = ready;
        }
    }

    pub(crate) fn is_ready(&self) -> bool {
        self.ready.lock().map(|ready| *ready).unwrap_or(false)
    }

    pub(crate) fn mark_unhealthy(&self) -> bool {
        self.ready
            .lock()
            .map(|mut ready| {
                let was_ready = *ready;
                *ready = false;
                was_ready
            })
            .unwrap_or(false)
    }
}

#[derive(Clone, Default)]
pub(crate) struct RelayRouteState {
    ready: Arc<Mutex<bool>>,
}

impl RelayRouteState {
    pub(crate) fn set_ready(&self, ready: bool) {
        if let Ok(mut current) = self.ready.lock() {
            *current = ready;
        }
    }

    pub(crate) fn is_ready(&self) -> bool {
        self.ready.lock().map(|ready| *ready).unwrap_or(false)
    }
}

pub(crate) fn controller_relay_state_handler(
    relay_route: RelayRouteState,
    direct_route: DirectRouteState,
    on_state: RemoteTransportStateHandler,
) -> RemoteTransportStateHandler {
    Arc::new(move |device_id, state| {
        if state == "connected" {
            relay_route.set_ready(true);
            if !direct_route.is_ready() {
                on_state(device_id, "connected:path=relay".to_string());
            }
            return;
        }
        if state == "closed" {
            relay_route.set_ready(false);
            if !direct_route.is_ready() {
                on_state(device_id, "closed".to_string());
            }
            return;
        }
        on_state(device_id, state);
    })
}

impl RemoteControllerCompositeTransport {
    pub(crate) async fn connect(
        config: &crate::RemoteControllerTransportConfig,
        relay: Arc<RemoteWebSocketControllerTransport>,
        direct_route: DirectRouteState,
        relay_route: RelayRouteState,
        on_message: RemoteTransportMessageHandler,
        on_state: RemoteTransportStateHandler,
    ) -> Result<Arc<Self>, String> {
        let mut media_engine = MediaEngine::default();
        media_engine
            .register_default_codecs()
            .map_err(|error| error.to_string())?;
        let api = APIBuilder::new().with_media_engine(media_engine).build();
        let pc = Arc::new(
            api.new_peer_connection(RTCConfiguration {
                ice_servers: vec![RTCIceServer {
                    urls: if config.stun_urls.is_empty() {
                        remote_stun_urls()
                    } else {
                        config.stun_urls.clone()
                    },
                    ..Default::default()
                }],
                ..Default::default()
            })
            .await
            .map_err(|error| error.to_string())?,
        );
        let (direct_tx, direct_rx) = mpsc::unbounded_channel::<ControllerDirectSend>();
        let transport = Arc::new(Self {
            relay,
            pc: Arc::clone(&pc),
            dc: Mutex::new(None),
            direct_tx,
            direct_route,
            relay_route,
            on_state: Arc::clone(&on_state),
        });
        transport.spawn_direct_writer(direct_rx);
        let weak_transport = Arc::downgrade(&transport);
        transport
            .relay
            .set_control_handler(Some(Arc::new(move |_, envelope| {
                if !envelope.kind.starts_with("webrtc.") {
                    return false;
                }
                if let Some(transport) = weak_transport.upgrade() {
                    tokio::spawn(async move {
                        transport.handle_signal(envelope).await;
                    });
                }
                true
            })));

        let ice_relay = Arc::clone(&transport.relay);
        let ice_device_id = config.device_id.clone();
        pc.on_ice_candidate(Box::new(move |candidate| {
            let ice_relay = Arc::clone(&ice_relay);
            let ice_device_id = ice_device_id.clone();
            Box::pin(async move {
                let Some(candidate) = candidate else {
                    return;
                };
                if let Ok(candidate) = candidate.to_json() {
                    send_controller_signal(
                        &ice_relay,
                        "webrtc.ice",
                        &ice_device_id,
                        json!({ "candidate": candidate }),
                    );
                }
            })
        }));

        let state_handler = Arc::clone(&on_state);
        let direct_route = transport.direct_route.clone();
        let state_transport = Arc::clone(&transport);
        pc.on_peer_connection_state_change(Box::new(move |state| {
            let state_handler = Arc::clone(&state_handler);
            let direct_route = direct_route.clone();
            let relay_route = state_transport.relay_route.clone();
            let relay = Arc::clone(&state_transport.relay);
            Box::pin(async move {
                if matches!(
                    state,
                    RTCPeerConnectionState::Failed
                        | RTCPeerConnectionState::Disconnected
                        | RTCPeerConnectionState::Closed
                ) {
                    if direct_route.mark_unhealthy() {
                        if relay.is_open() && relay_route.is_ready() {
                            state_handler(String::new(), "connected:path=relay".to_string());
                        } else {
                            relay_route.set_ready(false);
                            state_handler(String::new(), "closed".to_string());
                        }
                    }
                }
            })
        }));

        let dc = pc
            .create_data_channel("codux", None)
            .await
            .map_err(|error| error.to_string())?;
        install_controller_data_channel(
            Arc::clone(&transport),
            Arc::clone(&dc),
            on_message,
            on_state,
        );
        if let Ok(mut current) = transport.dc.lock() {
            *current = Some(dc);
        }

        let offer = pc
            .create_offer(None)
            .await
            .map_err(|error| error.to_string())?;
        let mut gathering_complete = pc.gathering_complete_promise().await;
        pc.set_local_description(offer)
            .await
            .map_err(|error| error.to_string())?;
        let _ = gathering_complete.recv().await;
        let description = pc
            .local_description()
            .await
            .ok_or_else(|| "Missing WebRTC local offer.".to_string())?;
        send_controller_signal(
            &transport.relay,
            "webrtc.offer",
            &config.device_id,
            json!({ "description": description }),
        );

        Ok(transport)
    }

    fn spawn_direct_writer(
        self: &Arc<Self>,
        mut rx: mpsc::UnboundedReceiver<ControllerDirectSend>,
    ) {
        let direct_route = self.direct_route.clone();
        let relay_route = self.relay_route.clone();
        let relay = Arc::clone(&self.relay);
        let on_state = Arc::clone(&self.on_state);
        tokio::spawn(async move {
            while let Some(message) = rx.recv().await {
                if message.channel.ready_state() != RTCDataChannelState::Open {
                    if relay.is_open() && relay_route.is_ready() {
                        let _ = relay.send(message.fallback_data, None);
                    } else {
                        relay_route.set_ready(false);
                        on_state(String::new(), "closed".to_string());
                    }
                    continue;
                }
                if message.channel.send_text(&message.text).await.is_err()
                    && direct_route.mark_unhealthy()
                {
                    if relay.is_open() && relay_route.is_ready() {
                        on_state(String::new(), "connected:path=relay".to_string());
                        let _ = relay.send(message.fallback_data, None);
                    } else {
                        relay_route.set_ready(false);
                        on_state(String::new(), "closed".to_string());
                    }
                }
            }
        });
    }

    async fn handle_signal(&self, envelope: RemoteEnvelope) {
        match envelope.kind.as_str() {
            "webrtc.answer" => {
                if let Some(description) = envelope
                    .payload
                    .get("description")
                    .cloned()
                    .and_then(|value| session_description_from_value(value).ok())
                {
                    let _ = self.pc.set_remote_description(description).await;
                }
            }
            "webrtc.ice" => {
                if let Some(candidate) = envelope
                    .payload
                    .get("candidate")
                    .cloned()
                    .and_then(|value| serde_json::from_value::<RTCIceCandidateInit>(value).ok())
                {
                    let _ = self.pc.add_ice_candidate(candidate).await;
                }
            }
            _ => {}
        }
    }

    fn mark_direct_unhealthy(&self) -> bool {
        let degraded = self.direct_route.mark_unhealthy();
        if degraded {
            self.publish_current_path_after_direct_loss();
        }
        degraded
    }

    fn publish_current_path_after_direct_loss(&self) {
        if self.relay.is_open() && self.relay_route.is_ready() {
            (self.on_state)(String::new(), "connected:path=relay".to_string());
        } else {
            self.relay_route.set_ready(false);
            (self.on_state)(String::new(), "closed".to_string());
        }
    }
}

#[async_trait]
impl RemoteTransport for RemoteControllerCompositeTransport {
    fn kind(&self) -> RemoteTransportKind {
        RemoteTransportKind::WebRtc
    }

    fn send(&self, data: Vec<u8>, device_id: Option<&str>) -> bool {
        if self.direct_route.is_ready() {
            let channel = self.dc.lock().ok().and_then(|dc| dc.clone());
            if let Some(channel) = channel {
                if channel.ready_state() == RTCDataChannelState::Open {
                    if let Ok(text) = String::from_utf8(data.clone()) {
                        return self
                            .direct_tx
                            .send(ControllerDirectSend {
                                channel,
                                text,
                                fallback_data: data,
                            })
                            .is_ok();
                    }
                } else {
                    self.mark_direct_unhealthy();
                }
            } else {
                self.mark_direct_unhealthy();
            }
        }
        self.relay.send(data, device_id)
    }

    fn mark_direct_unhealthy(&self) -> bool {
        RemoteControllerCompositeTransport::mark_direct_unhealthy(self)
    }

    fn probe_preferred_route(&self) -> bool {
        if let Some(state) = preferred_route_probe_state(&self.direct_route) {
            (self.on_state)(String::new(), state.to_string());
            true
        } else {
            false
        }
    }

    async fn shutdown(&self) {
        self.relay.shutdown().await;
        let _ = self.pc.close().await;
    }
}

pub(crate) fn preferred_route_probe_state(direct_route: &DirectRouteState) -> Option<&'static str> {
    direct_route.is_ready().then_some("connected:path=direct")
}

pub struct RemoteWebRtcHostTransport {
    relay: Mutex<Option<Arc<RemoteWebSocketHostTransport>>>,
    peers: Mutex<HashMap<String, Arc<WebRtcPeer>>>,
    ice_servers: Vec<String>,
    on_message: RemoteTransportMessageHandler,
    on_state: RemoteTransportStateHandler,
    on_log: Option<RemoteTransportLogHandler>,
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
                if channel.ready_state() == RTCDataChannelState::Open {
                    if let Ok(text) = String::from_utf8(data.clone()) {
                        let relay = self.relay.lock().ok().and_then(|value| value.clone());
                        let data = data.clone();
                        let channel = Arc::clone(&channel);
                        tokio::spawn(async move {
                            if let Err(_error) = channel.send_text(text).await {
                                if let Some(relay) = relay {
                                    let _ = relay.send(data, None);
                                }
                            }
                        });
                        return true;
                    }
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
    let close_peer = weak_peer.clone();
    let close_device_id = device_id.clone();
    let close_state = Arc::clone(&on_state);
    dc.on_close(Box::new(move || {
        let close_peer = close_peer.clone();
        let close_state = Arc::clone(&close_state);
        let close_device_id = close_device_id.clone();
        Box::pin(async move {
            if let Some(peer) = close_peer.upgrade() {
                if let Ok(mut current) = peer.dc.lock() {
                    *current = None;
                }
            }
            close_state(close_device_id, "path=relay".to_string());
        })
    }));
    let message_dc = Arc::clone(&dc);
    dc.on_message(Box::new(move |message: DataChannelMessage| {
        let on_message = Arc::clone(&on_message);
        let device_id = device_id.clone();
        let dc = Arc::clone(&message_dc);
        Box::pin(async move {
            if let Some(pong) = transport_pong_for_ping_bytes(&message.data, Some(&device_id)) {
                let _ = dc.send_text(pong).await;
                return;
            }
            on_message(device_id, message.data.to_vec());
        })
    }));
}

fn install_controller_data_channel(
    transport: Arc<RemoteControllerCompositeTransport>,
    dc: Arc<RTCDataChannel>,
    on_message: RemoteTransportMessageHandler,
    on_state: RemoteTransportStateHandler,
) {
    let open_transport = Arc::clone(&transport);
    let open_state = Arc::clone(&on_state);
    dc.on_open(Box::new(move || {
        let open_transport = Arc::clone(&open_transport);
        let open_state = Arc::clone(&open_state);
        Box::pin(async move {
            open_transport.direct_route.set_ready(true);
            open_state(String::new(), "connected:path=direct".to_string());
        })
    }));
    let close_transport = Arc::clone(&transport);
    let close_state = Arc::clone(&on_state);
    dc.on_close(Box::new(move || {
        let close_transport = Arc::clone(&close_transport);
        let close_state = Arc::clone(&close_state);
        Box::pin(async move {
            if let Ok(mut current) = close_transport.dc.lock() {
                *current = None;
            }
            if close_transport.direct_route.mark_unhealthy() {
                close_state(String::new(), "connected:path=relay".to_string());
            }
        })
    }));
    dc.on_message(Box::new(move |message: DataChannelMessage| {
        let on_message = Arc::clone(&on_message);
        Box::pin(async move {
            on_message(String::new(), message.data.to_vec());
        })
    }));
}

fn send_controller_signal(
    relay: &RemoteWebSocketControllerTransport,
    kind: &str,
    device_id: &str,
    payload: Value,
) -> bool {
    let envelope = RemoteOutgoingEnvelope {
        kind: kind.to_string(),
        device_id: Some(device_id.to_string()),
        session_id: None,
        seq: None,
        payload,
    };
    let Ok(data) = serde_json::to_vec(&envelope) else {
        return false;
    };
    relay.send(data, None)
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
