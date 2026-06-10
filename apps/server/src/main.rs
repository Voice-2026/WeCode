use anyhow::Context;
use axum::{
    Json, Router,
    extract::{
        Query, State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
};
use clap::Parser;
use codux_protocol::{
    REMOTE_PROTOCOL_VERSION, RemoteRelayDecision, RemoteRelayEnvelope, RemoteRelayPeerWindow,
    RemoteRelayPolicy, relay_error_envelope, relay_hello_envelope,
};
use futures_util::{SinkExt, StreamExt};
use rand::{Rng, distributions::Alphanumeric};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::Arc,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tokio::sync::{Mutex, mpsc};
use tracing::{info, warn};

#[derive(Parser, Debug)]
struct Args {
    #[arg(long, env = "CODUX_SERVER_ADDR", default_value = "0.0.0.0:8088")]
    addr: SocketAddr,
}

#[derive(Clone)]
struct AppState {
    inner: Arc<Mutex<HubState>>,
}

#[derive(Default)]
struct HubState {
    hosts: HashMap<String, PeerSender>,
    clients: HashMap<String, PeerSender>,
    tickets: HashMap<String, TicketEntry>,
    tickets_by_code: HashMap<String, String>,
}

#[derive(Clone)]
struct PeerSender {
    host_id: String,
    tx: mpsc::UnboundedSender<RemoteRelayEnvelope>,
}

struct TicketEntry {
    payload: Value,
    expires_at: Instant,
    code: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct TicketResponse {
    ticket: String,
    expires_at: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct HostQuery {
    host_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ClientQuery {
    host_id: String,
    device_id: String,
}

#[derive(Clone, Copy)]
enum Role {
    Host,
    Client,
}

struct Peer {
    role: Role,
    host_id: String,
    device_id: String,
    relay_window: RemoteRelayPeerWindow,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    let args = Args::parse();
    let state = AppState {
        inner: Arc::new(Mutex::new(HubState::default())),
    };
    let app = Router::new()
        .route("/healthz", get(health))
        .route("/v3/healthz", get(health))
        .route("/v3/api/tickets", post(create_ticket))
        .route("/v3/api/tickets/{ticket}", get(get_ticket))
        .route("/v3/api/pairings/code/{code}", get(get_pairing_code))
        .route("/v3/ws/host", get(host_socket))
        .route("/v3/ws/client", get(client_socket))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(args.addr)
        .await
        .with_context(|| format!("bind {}", args.addr))?;
    info!("codux rust relay listening addr={}", args.addr);
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .context("serve relay")
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
}

async fn health() -> impl IntoResponse {
    Json(json!({ "ok": true, "protocolVersion": REMOTE_PROTOCOL_VERSION }))
}

async fn create_ticket(
    State(state): State<AppState>,
    Json(payload): Json<Value>,
) -> impl IntoResponse {
    let Ok(data) = serde_json::to_vec(&payload) else {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "invalid_json" })),
        );
    };
    let ticket = token(12);
    let policy = RemoteRelayPolicy::default();
    let RemoteRelayDecision::Allow = policy.validate_ticket_payload_size(data.len()) else {
        return (
            StatusCode::PAYLOAD_TOO_LARGE,
            Json(json!({ "error": "ticket_payload_too_large" })),
        );
    };
    let expires_at = Instant::now() + Duration::from_secs(policy.ticket_ttl_secs);
    let mut hub = state.inner.lock().await;
    hub.prune_tickets();
    if matches!(
        policy.validate_ticket_capacity(hub.tickets.len()),
        RemoteRelayDecision::Reject(_)
    ) {
        return (
            StatusCode::TOO_MANY_REQUESTS,
            Json(json!({ "error": "too_many_active_tickets" })),
        );
    }
    hub.insert_ticket(ticket.clone(), payload, expires_at);
    (
        StatusCode::OK,
        Json(json!(TicketResponse {
            ticket,
            expires_at: unix_millis(
                SystemTime::now() + Duration::from_secs(policy.ticket_ttl_secs)
            ),
        })),
    )
}

async fn get_ticket(
    State(state): State<AppState>,
    axum::extract::Path(ticket): axum::extract::Path<String>,
) -> impl IntoResponse {
    let mut hub = state.inner.lock().await;
    hub.prune_tickets();
    let Some(entry) = hub.take_ticket(ticket.trim()) else {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "ticket_not_found_or_expired" })),
        );
    };
    (StatusCode::OK, Json(entry.payload))
}

async fn get_pairing_code(
    State(state): State<AppState>,
    axum::extract::Path(code): axum::extract::Path<String>,
) -> impl IntoResponse {
    let Some(code) = normalize_pairing_code(&code) else {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "invalid_pairing_code" })),
        );
    };
    let mut hub = state.inner.lock().await;
    hub.prune_tickets();
    let Some(entry) = hub.take_pairing_code(&code) else {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "pairing_code_not_found_or_expired" })),
        );
    };
    (StatusCode::OK, Json(entry.payload))
}

async fn host_socket(
    State(state): State<AppState>,
    Query(query): Query<HostQuery>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    if query.host_id.trim().is_empty() {
        return StatusCode::BAD_REQUEST.into_response();
    }
    ws.on_upgrade(move |socket| run_peer(socket, state, Role::Host, query.host_id, String::new()))
        .into_response()
}

async fn client_socket(
    State(state): State<AppState>,
    Query(query): Query<ClientQuery>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    if query.host_id.trim().is_empty() || query.device_id.trim().is_empty() {
        return StatusCode::BAD_REQUEST.into_response();
    }
    ws.on_upgrade(move |socket| {
        run_peer(socket, state, Role::Client, query.host_id, query.device_id)
    })
    .into_response()
}

async fn run_peer(
    socket: WebSocket,
    state: AppState,
    role: Role,
    host_id: String,
    device_id: String,
) {
    let (mut sink, mut stream) = socket.split();
    let (tx, mut rx) = mpsc::unbounded_channel::<RemoteRelayEnvelope>();
    let mut peer = Peer {
        role,
        host_id: host_id.clone(),
        device_id: device_id.clone(),
        relay_window: RemoteRelayPeerWindow::default(),
    };
    {
        let mut hub = state.inner.lock().await;
        match role {
            Role::Host => {
                if let Some(old) = hub.hosts.insert(
                    host_id.clone(),
                    PeerSender {
                        host_id: host_id.clone(),
                        tx: tx.clone(),
                    },
                ) {
                    let _ = old.tx.send(relay_error(&host_id, "", "replaced"));
                }
            }
            Role::Client => {
                if let Some(old) = hub.clients.insert(
                    device_id.clone(),
                    PeerSender {
                        host_id: host_id.clone(),
                        tx: tx.clone(),
                    },
                ) {
                    let _ = old.tx.send(relay_error(&host_id, &device_id, "replaced"));
                }
            }
        }
    }
    let _ = tx.send(relay_hello_envelope(
        host_id.clone(),
        device_id.clone(),
        json!({ "role": role_name(role) }),
        Some(now_millis()),
    ));

    let writer = tokio::spawn(async move {
        while let Some(message) = rx.recv().await {
            let Ok(text) = serde_json::to_string(&message) else {
                continue;
            };
            if sink.send(Message::Text(text.into())).await.is_err() {
                break;
            }
        }
    });

    while let Some(message) = stream.next().await {
        let Ok(Message::Text(text)) = message else {
            continue;
        };
        let Ok(mut envelope) = serde_json::from_str::<RemoteRelayEnvelope>(&text) else {
            continue;
        };
        envelope.at = Some(now_millis());
        if !allow_relay_message(&mut peer, &envelope, &tx) {
            continue;
        }
        forward_envelope(&state, &peer, envelope).await;
    }

    writer.abort();
    let mut hub = state.inner.lock().await;
    match role {
        Role::Host => {
            if hub
                .hosts
                .get(&host_id)
                .map(|sender| sender.tx.same_channel(&tx))
                .unwrap_or(false)
            {
                hub.hosts.remove(&host_id);
            }
        }
        Role::Client => {
            if hub
                .clients
                .get(&device_id)
                .map(|sender| sender.tx.same_channel(&tx))
                .unwrap_or(false)
            {
                hub.clients.remove(&device_id);
            }
        }
    }
}

async fn forward_envelope(state: &AppState, peer: &Peer, mut envelope: RemoteRelayEnvelope) {
    let hub = state.inner.lock().await;
    match peer.role {
        Role::Host => {
            if !envelope.device_id.is_empty() {
                if let Some(client) = hub.clients.get(&envelope.device_id) {
                    let _ = client.tx.send(envelope);
                }
            } else {
                for client in hub
                    .clients
                    .values()
                    .filter(|client| client.host_id == peer.host_id)
                {
                    let _ = client.tx.send(envelope.clone());
                }
            }
        }
        Role::Client => {
            envelope.host_id = peer.host_id.clone();
            envelope.device_id = peer.device_id.clone();
            if let Some(host) = hub.hosts.get(&peer.host_id) {
                let _ = host.tx.send(envelope);
            }
        }
    }
}

fn allow_relay_message(
    peer: &mut Peer,
    envelope: &RemoteRelayEnvelope,
    tx: &mpsc::UnboundedSender<RemoteRelayEnvelope>,
) -> bool {
    let policy = RemoteRelayPolicy::default();
    let size = serde_json::to_vec(envelope)
        .map(|data| data.len())
        .unwrap_or(usize::MAX);
    match policy.validate_message(envelope, size, &mut peer.relay_window, now_millis()) {
        RemoteRelayDecision::Allow => true,
        RemoteRelayDecision::Reject(error) => send_relay_error(peer, tx, error),
    }
}

fn send_relay_error(
    peer: &Peer,
    tx: &mpsc::UnboundedSender<RemoteRelayEnvelope>,
    error: &str,
) -> bool {
    warn!(
        "relay message rejected role={} host={} device={} error={}",
        role_name(peer.role),
        peer.host_id,
        peer.device_id,
        error
    );
    let _ = tx.send(relay_error(&peer.host_id, &peer.device_id, error));
    false
}

impl HubState {
    fn insert_ticket(&mut self, ticket: String, payload: Value, expires_at: Instant) {
        let code = pairing_code_from_payload(&payload);
        if let Some(code) = &code {
            if let Some(old_ticket) = self.tickets_by_code.insert(code.clone(), ticket.clone()) {
                self.tickets.remove(&old_ticket);
            }
        }
        self.tickets.insert(
            ticket,
            TicketEntry {
                payload,
                expires_at,
                code,
            },
        );
    }

    fn take_ticket(&mut self, ticket: &str) -> Option<TicketEntry> {
        let entry = self.tickets.remove(ticket.trim())?;
        if let Some(code) = &entry.code {
            if self
                .tickets_by_code
                .get(code)
                .is_some_and(|mapped| mapped == ticket.trim())
            {
                self.tickets_by_code.remove(code);
            }
        }
        Some(entry)
    }

    fn take_pairing_code(&mut self, code: &str) -> Option<TicketEntry> {
        let ticket = self.tickets_by_code.remove(code)?;
        self.tickets.remove(&ticket)
    }

    fn prune_tickets(&mut self) {
        let now = Instant::now();
        self.tickets.retain(|_, ticket| ticket.expires_at > now);
        self.tickets_by_code
            .retain(|_, ticket| self.tickets.contains_key(ticket));
    }
}

fn pairing_code_from_payload(payload: &Value) -> Option<String> {
    normalize_pairing_code(payload.get("code")?.as_str()?)
}

fn normalize_pairing_code(code: &str) -> Option<String> {
    let value: String = code.chars().filter(|ch| ch.is_ascii_digit()).collect();
    if value.len() == 6 { Some(value) } else { None }
}

fn relay_error(host_id: &str, device_id: &str, error: &str) -> RemoteRelayEnvelope {
    relay_error_envelope(host_id, device_id, error, Some(now_millis()))
}

fn role_name(role: Role) -> &'static str {
    match role {
        Role::Host => "host",
        Role::Client => "client",
    }
}

fn token(len: usize) -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(len)
        .map(char::from)
        .collect()
}

fn now_millis() -> i64 {
    unix_millis(SystemTime::now())
}

fn unix_millis(time: SystemTime) -> i64 {
    time.duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hub_prunes_expired_tickets() {
        let mut hub = HubState::default();
        hub.tickets.insert(
            "expired".to_string(),
            TicketEntry {
                payload: json!({ "ok": true }),
                expires_at: Instant::now() - Duration::from_secs(1),
                code: Some("111111".to_string()),
            },
        );
        hub.tickets_by_code
            .insert("111111".to_string(), "expired".to_string());
        hub.tickets.insert(
            "active".to_string(),
            TicketEntry {
                payload: json!({ "ok": true }),
                expires_at: Instant::now() + Duration::from_secs(1),
                code: Some("222222".to_string()),
            },
        );
        hub.tickets_by_code
            .insert("222222".to_string(), "active".to_string());

        hub.prune_tickets();

        assert!(!hub.tickets.contains_key("expired"));
        assert!(!hub.tickets_by_code.contains_key("111111"));
        assert!(hub.tickets.contains_key("active"));
        assert_eq!(
            hub.tickets_by_code.get("222222"),
            Some(&"active".to_string())
        );
    }

    #[test]
    fn hub_indexes_pairing_tickets_by_six_digit_code_once() {
        let mut hub = HubState::default();
        hub.insert_ticket(
            "ticket-1".to_string(),
            json!({ "code": "123456", "hostId": "host-1" }),
            Instant::now() + Duration::from_secs(60),
        );

        let entry = hub.take_pairing_code("123456").expect("pairing code");
        assert_eq!(entry.payload["hostId"], "host-1");
        assert!(hub.take_pairing_code("123456").is_none());
        assert!(hub.take_ticket("ticket-1").is_none());
    }

    #[test]
    fn hub_removes_code_index_when_ticket_is_taken_by_qr() {
        let mut hub = HubState::default();
        hub.insert_ticket(
            "ticket-1".to_string(),
            json!({ "code": "123456", "hostId": "host-1" }),
            Instant::now() + Duration::from_secs(60),
        );

        assert!(hub.take_ticket("ticket-1").is_some());
        assert!(hub.take_pairing_code("123456").is_none());
    }
}
