use super::*;
use crate::control_messages::transport_pong_for_ping;
use crate::health::{ControllerHealthState, ControllerHealthTransport};
use crate::webrtc::{
    DirectRouteState, RelayRouteState, controller_relay_state_handler, preferred_route_probe_state,
};
use codux_protocol::{REMOTE_TRANSPORT_PING, REMOTE_TRANSPORT_PONG, RemoteOutgoingEnvelope};
use serde_json::{Value, json};
use std::sync::atomic::AtomicBool;
use std::sync::{Mutex, Mutex as StdMutex};
use tokio::sync::mpsc;

struct TestTransport {
    sent: StdMutex<Vec<String>>,
    unhealthy_count: StdMutex<usize>,
    send_results: StdMutex<Vec<bool>>,
}

#[async_trait]
impl RemoteTransport for TestTransport {
    fn kind(&self) -> RemoteTransportKind {
        RemoteTransportKind::WebRtc
    }

    fn send(&self, data: Vec<u8>, _device_id: Option<&str>) -> bool {
        self.sent
            .lock()
            .unwrap()
            .push(String::from_utf8(data).unwrap());
        self.send_results.lock().unwrap().pop().unwrap_or(true)
    }

    fn mark_direct_unhealthy(&self) -> bool {
        let mut unhealthy_count = self.unhealthy_count.lock().unwrap();
        *unhealthy_count += 1;
        true
    }

    fn probe_preferred_route(&self) -> bool {
        false
    }

    async fn shutdown(&self) {}
}

fn test_health_state(
    messages: Arc<StdMutex<Vec<String>>>,
    states: Arc<StdMutex<Vec<String>>>,
) -> Arc<ControllerHealthState> {
    Arc::new(ControllerHealthState::new(
        "device-1".to_string(),
        {
            let messages = Arc::clone(&messages);
            Arc::new(move |_, data| {
                messages
                    .lock()
                    .unwrap()
                    .push(String::from_utf8(data).unwrap());
            })
        },
        {
            let states = Arc::clone(&states);
            Arc::new(move |_, state| {
                states.lock().unwrap().push(state);
            })
        },
        None,
    ))
}

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
        remote_pairing_code_url("https://relay.example", "123456").unwrap(),
        "https://relay.example/v3/api/pairings/code/123456"
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

#[test]
fn controller_prefers_webrtc_when_relay_and_direct_are_available() {
    let config = RemoteControllerTransportConfig {
        server_url: "https://relay.example".to_string(),
        host_id: "host-1".to_string(),
        device_id: "device-1".to_string(),
        transports: vec![
            RemoteTransportCandidate {
                kind: "websocketRelay".to_string(),
                url: "https://relay.example/v3".to_string(),
            },
            RemoteTransportCandidate {
                kind: "webRtc".to_string(),
                url: "https://relay.example/v3".to_string(),
            },
        ],
        ..Default::default()
    };

    assert_eq!(
        preferred_controller_transport_kind(
            config
                .transports
                .iter()
                .map(|candidate| (candidate.kind.as_str(), candidate.url.as_str()))
        ),
        "webRtc"
    );
}

#[test]
fn direct_route_state_degrades_once_after_unhealthy_signal() {
    let route = DirectRouteState::default();
    assert!(!route.is_ready());

    route.set_ready(true);
    assert!(route.is_ready());
    assert!(route.mark_unhealthy());
    assert!(!route.is_ready());
    assert!(!route.mark_unhealthy());
}

#[test]
fn preferred_route_probe_does_not_force_relay_when_direct_is_not_ready() {
    let route = DirectRouteState::default();

    assert_eq!(preferred_route_probe_state(&route), None);

    route.set_ready(true);
    assert_eq!(
        preferred_route_probe_state(&route),
        Some("connected:path=direct")
    );
}

#[test]
fn controller_health_intercepts_pong_and_reports_latency_state() {
    let messages = Arc::new(StdMutex::new(Vec::<String>::new()));
    let states = Arc::new(StdMutex::new(Vec::<String>::new()));
    let health = test_health_state(Arc::clone(&messages), Arc::clone(&states));
    health.handle_state("device-1".to_string(), "connected:path=direct".to_string());
    let ping = health.next_ping();
    assert!(health.begin_ping(ping.clone()));
    health.handle_message(
        "device-1".to_string(),
        serde_json::to_vec(&RemoteOutgoingEnvelope {
            kind: REMOTE_TRANSPORT_PONG.to_string(),
            device_id: None,
            session_id: None,
            seq: None,
            payload: json!({ "id": ping.id }),
        })
        .unwrap(),
    );

    assert!(messages.lock().unwrap().is_empty());
    assert!(
        states
            .lock()
            .unwrap()
            .iter()
            .any(|state| state.starts_with("latency:rtt=") && state.contains("path=direct"))
    );
}

#[test]
fn controller_health_direct_timeout_degrades_to_inner_relay() {
    let messages = Arc::new(StdMutex::new(Vec::<String>::new()));
    let states = Arc::new(StdMutex::new(Vec::<String>::new()));
    let health = test_health_state(messages, Arc::clone(&states));
    health.set_path("direct");
    let inner = Arc::new(TestTransport {
        sent: StdMutex::new(Vec::new()),
        unhealthy_count: StdMutex::new(0),
        send_results: StdMutex::new(Vec::new()),
    });
    let transport = ControllerHealthTransport {
        inner: Arc::clone(&inner) as Arc<dyn RemoteTransport>,
        health,
        closed: AtomicBool::new(false),
    };
    let ping = transport.health.next_ping();
    assert!(transport.health.begin_ping(ping.clone()));
    transport.handle_ping_timeout(&ping.id);

    assert_eq!(*inner.unhealthy_count.lock().unwrap(), 1);
    let states = states.lock().unwrap();
    assert!(
        states
            .iter()
            .any(|state| state == "latency:timeout=1;path=direct")
    );
    assert_eq!(transport.health.path(), "relay");
}

#[test]
fn controller_health_send_failure_on_direct_degrades_and_retries_message() {
    let messages = Arc::new(StdMutex::new(Vec::<String>::new()));
    let states = Arc::new(StdMutex::new(Vec::<String>::new()));
    let health = test_health_state(messages, states);
    health.set_path("direct");
    let inner = Arc::new(TestTransport {
        sent: StdMutex::new(Vec::new()),
        unhealthy_count: StdMutex::new(0),
        send_results: StdMutex::new(vec![true, false]),
    });
    let transport = ControllerHealthTransport {
        inner: Arc::clone(&inner) as Arc<dyn RemoteTransport>,
        health,
        closed: AtomicBool::new(false),
    };

    assert!(transport.send(b"project.select".to_vec(), None));

    assert_eq!(*inner.unhealthy_count.lock().unwrap(), 1);
    assert_eq!(transport.health.path(), "relay");
    assert_eq!(
        inner.sent.lock().unwrap().as_slice(),
        ["project.select", "project.select"]
    );
}

#[test]
fn controller_relay_state_reports_closed_when_direct_is_not_ready() {
    let states = Arc::new(StdMutex::new(Vec::<String>::new()));
    let relay_route = RelayRouteState::default();
    let direct_route = DirectRouteState::default();
    let handler = controller_relay_state_handler(relay_route.clone(), direct_route.clone(), {
        let states = Arc::clone(&states);
        Arc::new(move |_, state| states.lock().unwrap().push(state))
    });

    handler(String::new(), "connected".to_string());
    handler(String::new(), "closed".to_string());

    assert!(!relay_route.is_ready());
    assert_eq!(
        states.lock().unwrap().as_slice(),
        ["connected:path=relay", "closed"]
    );
}

#[test]
fn controller_relay_closed_does_not_downgrade_visible_direct_path() {
    let states = Arc::new(StdMutex::new(Vec::<String>::new()));
    let relay_route = RelayRouteState::default();
    let direct_route = DirectRouteState::default();
    direct_route.set_ready(true);
    let handler = controller_relay_state_handler(relay_route.clone(), direct_route, {
        let states = Arc::clone(&states);
        Arc::new(move |_, state| states.lock().unwrap().push(state))
    });

    handler(String::new(), "connected".to_string());
    handler(String::new(), "closed".to_string());

    assert!(!relay_route.is_ready());
    assert!(states.lock().unwrap().is_empty());
}

#[tokio::test]
async fn controller_health_ping_targets_controller_device() {
    let messages = Arc::new(StdMutex::new(Vec::<String>::new()));
    let states = Arc::new(StdMutex::new(Vec::<String>::new()));
    let health = test_health_state(messages, states);
    let inner = Arc::new(TestTransport {
        sent: StdMutex::new(Vec::new()),
        unhealthy_count: StdMutex::new(0),
        send_results: StdMutex::new(Vec::new()),
    });
    let transport = Arc::new(ControllerHealthTransport {
        inner: Arc::clone(&inner) as Arc<dyn RemoteTransport>,
        health,
        closed: AtomicBool::new(false),
    });

    transport.send_health_ping().await;

    let sent = inner.sent.lock().unwrap();
    assert_eq!(sent.len(), 1);
    let envelope: Value = serde_json::from_str(&sent[0]).unwrap();
    assert_eq!(
        envelope.get("type").and_then(Value::as_str),
        Some(REMOTE_TRANSPORT_PING)
    );
    assert_eq!(
        envelope.get("deviceId").and_then(Value::as_str),
        Some("device-1")
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
fn host_websocket_intercepts_transport_ping_and_replies_pong() {
    let (tx, mut rx) = mpsc::unbounded_channel::<String>();
    let received = Arc::new(StdMutex::new(Vec::<String>::new()));
    let states = Arc::new(StdMutex::new(Vec::<String>::new()));
    let transport = RemoteWebSocketHostTransport {
        tx: Mutex::new(Some(tx)),
        on_message: {
            let received = Arc::clone(&received);
            Arc::new(move |_, data| {
                received
                    .lock()
                    .unwrap()
                    .push(String::from_utf8(data).unwrap());
            })
        },
        on_state: {
            let states = Arc::clone(&states);
            Arc::new(move |device, state| {
                states.lock().unwrap().push(format!("{device}:{state}"));
            })
        },
        on_pairing: Arc::new(|_| {}),
        on_control: None,
        on_log: None,
    };

    transport.handle_text(
        serde_json::to_string(&RemoteOutgoingEnvelope {
            kind: REMOTE_TRANSPORT_PING.to_string(),
            device_id: Some("device-1".to_string()),
            session_id: None,
            seq: None,
            payload: json!({ "id": "ping-1" }),
        })
        .unwrap(),
    );

    let pong = rx.try_recv().expect("transport pong");
    let envelope: Value = serde_json::from_str(&pong).unwrap();
    assert_eq!(
        envelope.get("type").and_then(Value::as_str),
        Some(REMOTE_TRANSPORT_PONG)
    );
    assert_eq!(
        envelope.get("deviceId").and_then(Value::as_str),
        Some("device-1")
    );
    assert_eq!(envelope["payload"]["id"], "ping-1");
    assert!(received.lock().unwrap().is_empty());
    assert_eq!(states.lock().unwrap().as_slice(), ["device-1:connected"]);
}

#[test]
fn transport_pong_for_ping_uses_fallback_device_id() {
    let ping = RemoteEnvelope {
        kind: REMOTE_TRANSPORT_PING.to_string(),
        device_id: None,
        session_id: None,
        seq: None,
        payload: json!({ "id": "ping-2" }),
    };

    let pong = transport_pong_for_ping(&ping, Some("device-2")).expect("pong");
    let envelope: Value = serde_json::from_str(&pong).unwrap();
    assert_eq!(
        envelope.get("type").and_then(Value::as_str),
        Some(REMOTE_TRANSPORT_PONG)
    );
    assert_eq!(
        envelope.get("deviceId").and_then(Value::as_str),
        Some("device-2")
    );
    assert_eq!(envelope["payload"]["id"], "ping-2");
}

#[tokio::test]
async fn controller_relay_control_handler_intercepts_webrtc_signaling() {
    let received = Arc::new(StdMutex::new(Vec::<String>::new()));
    let handled = Arc::new(StdMutex::new(Vec::<String>::new()));
    let transport = RemoteWebSocketControllerTransport {
        tx: Mutex::new(None),
        closed: AtomicBool::new(false),
        on_message: {
            let received = Arc::clone(&received);
            Arc::new(move |_, data| {
                received
                    .lock()
                    .unwrap()
                    .push(String::from_utf8(data).unwrap());
            })
        },
        on_state: Arc::new(|_, _| {}),
        on_control: Mutex::new(Some({
            let handled = Arc::clone(&handled);
            Arc::new(move |_, envelope| {
                if envelope.kind.starts_with("webrtc.") {
                    handled.lock().unwrap().push(envelope.kind);
                    return true;
                }
                false
            })
        })),
        on_log: None,
    };

    transport.handle_text(
        serde_json::to_string(&RemoteOutgoingEnvelope {
            kind: "webrtc.answer".to_string(),
            device_id: None,
            session_id: None,
            seq: None,
            payload: json!({}),
        })
        .unwrap(),
    );
    transport.handle_text(
        serde_json::to_string(&RemoteOutgoingEnvelope {
            kind: "project.list".to_string(),
            device_id: None,
            session_id: None,
            seq: None,
            payload: json!({}),
        })
        .unwrap(),
    );

    assert_eq!(handled.lock().unwrap().as_slice(), ["webrtc.answer"]);
    assert_eq!(received.lock().unwrap().len(), 1);
    assert!(received.lock().unwrap()[0].contains("project.list"));
}

#[test]
fn controller_websocket_send_fails_after_close() {
    let (tx, _rx) = mpsc::unbounded_channel::<String>();
    let transport = RemoteWebSocketControllerTransport {
        tx: Mutex::new(Some(tx)),
        closed: AtomicBool::new(false),
        on_message: Arc::new(|_, _| {}),
        on_state: Arc::new(|_, _| {}),
        on_control: Mutex::new(None),
        on_log: None,
    };

    assert!(transport.send(b"before".to_vec(), None));
    transport.close_sender();

    assert!(!transport.is_open());
    assert!(!transport.send(b"after".to_vec(), None));
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
