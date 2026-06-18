//! Minimal headless host: serve a few real runtime domains over the Iroh
//! transport so a controller (desktop client or mobile) can browse this
//! machine's files and read its host info. This is the first real slice of the
//! "headless controlled-end" — terminal/Git/AI domains follow the same
//! dispatch shape (see plan/interconnect-plan.md), reusing the stateless
//! payload builders in `codux-runtime-core`.

use codux_protocol::{
    REMOTE_ERROR, REMOTE_FILE_LIST, REMOTE_FILE_READ, REMOTE_HOST_INFO, REMOTE_TRANSPORT_IROH,
    REMOTE_TRANSPORT_PING, REMOTE_TRANSPORT_PONG,
};
use codux_remote_transport::{
    RemoteHostTransportConfig, RemoteTransport, RemoteTransportCandidate, RemoteTransportFactory,
};
use codux_runtime_core::{
    file::{file_list_payload, file_read_payload},
    host::{host_info_payload, HostInfoPayload},
};
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};

/// What the agent needs to stand up a host endpoint.
pub struct AgentHostConfig {
    pub host_id: String,
    pub host_token: String,
    pub name: String,
    pub relay_preset: String,
    pub relay_url: String,
}

type TransportSlot = Arc<Mutex<Option<Arc<dyn RemoteTransport>>>>;

/// Build the message handler that dispatches incoming envelopes to the served
/// domains and replies through the (post-connect) transport handle.
fn make_handler(
    slot: TransportSlot,
    host_id: String,
    name: String,
) -> codux_remote_transport::RemoteTransportMessageHandler {
    Arc::new(move |_source: String, data: Vec<u8>| {
        let Ok(envelope) = serde_json::from_slice::<Value>(&data) else {
            return;
        };
        let kind = envelope.get("type").and_then(Value::as_str).unwrap_or("");
        let device_id = envelope.get("deviceId").and_then(Value::as_str);
        let request_id = envelope.get("requestId").and_then(Value::as_str);
        let payload = envelope.get("payload").cloned().unwrap_or(Value::Null);

        // (reply_kind, reply_payload). `None` => nothing to send back.
        let reply: Option<(&str, Value)> = match kind {
            REMOTE_TRANSPORT_PING => Some((REMOTE_TRANSPORT_PONG, json!({}))),
            REMOTE_HOST_INFO => Some((
                REMOTE_HOST_INFO,
                // Transports left empty: the controller already knows the path
                // it connected on; host.info here carries identity/capabilities.
                host_info_payload(HostInfoPayload {
                    host_id: host_id.clone(),
                    runtime_instance_id: format!("{host_id}-agent"),
                    name: name.clone(),
                    platform: std::env::consts::OS.to_string(),
                    app: "codux-agent".to_string(),
                    transports: Vec::new(),
                }),
            )),
            REMOTE_FILE_LIST => {
                let path = payload.get("path").and_then(Value::as_str);
                let purpose = payload.get("purpose").and_then(Value::as_str);
                Some((REMOTE_FILE_LIST, file_list_payload(path, purpose)))
            }
            REMOTE_FILE_READ => match payload.get("path").and_then(Value::as_str) {
                Some(path) => match file_read_payload(path) {
                    Ok(value) => Some((REMOTE_FILE_READ, value)),
                    Err(error) => Some((REMOTE_ERROR, json!({ "message": error }))),
                },
                None => Some((REMOTE_ERROR, json!({ "message": "File path is required." }))),
            },
            _ => None,
        };

        let Some((reply_kind, reply_payload)) = reply else {
            return;
        };
        let mut reply_envelope = json!({ "type": reply_kind, "payload": reply_payload });
        if let Some(device_id) = device_id {
            reply_envelope["deviceId"] = json!(device_id);
        }
        if let Some(request_id) = request_id {
            reply_envelope["requestId"] = json!(request_id);
        }
        let Ok(bytes) = serde_json::to_vec(&reply_envelope) else {
            return;
        };
        if let Ok(guard) = slot.lock() {
            if let Some(transport) = guard.as_ref() {
                transport.send(bytes, device_id);
            }
        }
    })
}

/// Connect a host transport with the dispatch handler. Returns the transport
/// handle and the slot it has been stored in (for replies).
async fn connect_serving_host(
    cfg: &AgentHostConfig,
) -> Result<(Arc<dyn RemoteTransport>, TransportSlot), String> {
    let slot: TransportSlot = Arc::new(Mutex::new(None));
    let config = RemoteHostTransportConfig {
        relay_url: cfg.relay_url.clone(),
        relay_preset: cfg.relay_preset.clone(),
        iroh_relay_url: String::new(),
        iroh_relay_authentication: String::new(),
        host_id: cfg.host_id.clone(),
        host_token: cfg.host_token.clone(),
    };
    let host = RemoteTransportFactory::connect_host(
        &config,
        make_handler(Arc::clone(&slot), cfg.host_id.clone(), cfg.name.clone()),
        Arc::new(|_| Ok(())),
        Arc::new(|_, _| {}),
        Arc::new(|_| {}),
        None,
    )
    .await?;
    if let Ok(mut guard) = slot.lock() {
        *guard = Some(Arc::clone(&host));
    }
    Ok((host, slot))
}

/// Run the headless host until the process is stopped, printing the pairing
/// candidate so a controller can connect.
pub async fn run_host(cfg: AgentHostConfig) -> Result<(), String> {
    let (host, _slot) = connect_serving_host(&cfg).await?;
    println!("codux-agent host ready");
    println!("hostId={}", cfg.host_id);
    println!("name={}", cfg.name);
    println!("platform={}", std::env::consts::OS);
    if let Some((node_id, relay_url)) = host.iroh_candidate() {
        println!("nodeId={node_id}");
        println!("relay={relay_url}");
    }
    if let Some(ticket) = host.iroh_endpoint_ticket() {
        println!("ticket={ticket}");
    }
    // Serve until the process is terminated.
    std::future::pending::<()>().await;
    Ok(())
}

/// In-process round trip: stand up the serving host, connect a controller, ask
/// for a directory listing, and confirm a real reply comes back. Proves the
/// headless host actually serves a domain end to end.
pub async fn run_serve_smoke_async() -> Result<String, String> {
    use codux_remote_transport::RemoteControllerTransportConfig;
    use tokio::sync::oneshot;

    let cfg = AgentHostConfig {
        host_id: "host-serve-smoke".to_string(),
        host_token: "token-serve-smoke".to_string(),
        name: "codux-agent-smoke".to_string(),
        relay_preset: "global".to_string(),
        relay_url: "https://relay.example".to_string(),
    };
    let (host, _slot) = connect_serving_host(&cfg).await?;
    let (node_id, relay_url) = host
        .iroh_candidate()
        .ok_or_else(|| "iroh host candidate missing".to_string())?;

    let (reply_tx, reply_rx) = oneshot::channel::<String>();
    let reply_tx = Arc::new(Mutex::new(Some(reply_tx)));
    let controller_config = RemoteControllerTransportConfig {
        relay_url: cfg.relay_url.clone(),
        host_id: cfg.host_id.clone(),
        device_id: "device-serve-smoke".to_string(),
        device_token: "token-serve-smoke".to_string(),
        transports: vec![RemoteTransportCandidate {
            kind: REMOTE_TRANSPORT_IROH.to_string(),
            url: "https://relay.example/v3".to_string(),
            node_id,
            relay_url,
            ticket: host.iroh_endpoint_ticket().unwrap_or_default(),
            relay_authentication: String::new(),
        }],
    };
    let controller = RemoteTransportFactory::connect_controller(
        &controller_config,
        {
            let reply_tx = Arc::clone(&reply_tx);
            Arc::new(move |_source: String, data: Vec<u8>| {
                let Ok(envelope) = serde_json::from_slice::<Value>(&data) else {
                    return;
                };
                if envelope.get("type").and_then(Value::as_str) == Some(REMOTE_FILE_LIST) {
                    if let Ok(mut guard) = reply_tx.lock() {
                        if let Some(tx) = guard.take() {
                            let _ = tx.send(envelope.get("payload").cloned().unwrap_or(Value::Null).to_string());
                        }
                    }
                }
            })
        },
        Arc::new(|_, _| {}),
        None,
    )
    .await?;

    let request = json!({
        "type": REMOTE_FILE_LIST,
        "deviceId": controller_config.device_id,
        "requestId": "serve-smoke-1",
        "payload": { "purpose": "projectFiles" },
    });
    let bytes = serde_json::to_vec(&request).map_err(|error| error.to_string())?;
    if !controller.send(bytes, None) {
        return Err("controller send failed".to_string());
    }
    let observed = tokio::time::timeout(std::time::Duration::from_secs(5), reply_rx)
        .await
        .map_err(|_| "file.list reply timeout".to_string())?
        .map_err(|_| "file.list reply receiver closed".to_string())?;
    host.shutdown().await;
    controller.shutdown().await;
    if !observed.contains("entries") {
        return Err(format!("file.list reply missing entries: {observed}"));
    }
    Ok(format!("codux-agent-serve-ok\nfile.list reply has entries"))
}
