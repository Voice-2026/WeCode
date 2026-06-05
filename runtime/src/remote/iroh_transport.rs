use super::types::RemoteEnvelope;
use iroh::{Endpoint, NodeAddr, RelayMode, SecretKey, endpoint::Connection};
#[cfg(test)]
use iroh::{NodeId, RelayUrl};
use serde::{Deserialize, Serialize};
use serde_json::Value;
#[cfg(test)]
use serde_json::json;
#[cfg(test)]
use std::net::SocketAddr;
#[cfg(test)]
use std::str::FromStr;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    sync::mpsc,
};

pub(crate) const CODUX_REMOTE_ALPN: &[u8] = b"codux/remote/iroh/v1";

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RemoteIrohNodeAddr {
    pub(crate) node_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) relay_url: Option<String>,
    #[serde(default)]
    pub(crate) direct_addresses: Vec<String>,
}

impl RemoteIrohNodeAddr {
    pub(crate) fn from_node_addr(addr: NodeAddr) -> Self {
        Self {
            node_id: addr.node_id.to_string(),
            relay_url: addr.relay_url.map(|url| url.to_string()),
            direct_addresses: addr
                .direct_addresses
                .into_iter()
                .map(|addr| addr.to_string())
                .collect(),
        }
    }

    #[cfg(test)]
    pub(crate) fn to_node_addr(&self) -> Result<NodeAddr, String> {
        let node_id = NodeId::from_str(self.node_id.trim()).map_err(|error| error.to_string())?;
        let relay_url = self
            .relay_url
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .map(RelayUrl::from_str)
            .transpose()
            .map_err(|error| error.to_string())?;
        let direct_addresses = self
            .direct_addresses
            .iter()
            .filter_map(|value| SocketAddr::from_str(value.trim()).ok())
            .collect::<Vec<_>>();
        Ok(NodeAddr::from_parts(node_id, relay_url, direct_addresses))
    }
}

#[derive(Clone, Debug)]
pub(crate) struct RemoteIrohHandshake {
    pub(crate) device_id: String,
    pub(crate) device_name: String,
    pub(crate) device_public_key: String,
    pub(crate) pairing_id: Option<String>,
    pub(crate) pairing_code: Option<String>,
    pub(crate) pairing_secret: Option<String>,
}

type MessageHandler = Arc<dyn Fn(String, Vec<u8>) + Send + Sync + 'static>;
type StateHandler = Arc<dyn Fn(String, String) + Send + Sync + 'static>;
type PairingHandler = Arc<dyn Fn(RemoteIrohHandshake) + Send + Sync + 'static>;

pub(crate) struct RemoteIrohHostTransport {
    endpoint: Endpoint,
    peers: Mutex<HashMap<String, mpsc::UnboundedSender<Vec<u8>>>>,
    on_message: MessageHandler,
    on_state: StateHandler,
    on_pairing: PairingHandler,
}

pub(crate) fn iroh_secret_key_from_settings(value: &str) -> (SecretKey, String) {
    let decoded = super::crypto::remote_base64_url_decode(value)
        .ok()
        .and_then(|bytes| <[u8; 32]>::try_from(bytes.as_slice()).ok());
    let secret_key = decoded
        .map(|bytes| SecretKey::from_bytes(&bytes))
        .unwrap_or_else(|| SecretKey::generate(rand::rngs::OsRng));
    let encoded = super::crypto::remote_base64_url_encode(&secret_key.to_bytes());
    (secret_key, encoded)
}

impl RemoteIrohHostTransport {
    pub(crate) async fn bind(
        secret_key: SecretKey,
        on_message: MessageHandler,
        on_state: StateHandler,
        on_pairing: PairingHandler,
    ) -> Result<Arc<Self>, String> {
        let endpoint = Endpoint::builder()
            .secret_key(secret_key)
            .alpns(vec![CODUX_REMOTE_ALPN.to_vec()])
            .relay_mode(RelayMode::Default)
            .discovery_n0()
            .bind()
            .await
            .map_err(|error| error.to_string())?;
        let transport = Arc::new(Self {
            endpoint,
            peers: Mutex::new(HashMap::new()),
            on_message,
            on_state,
            on_pairing,
        });
        transport.spawn_accept_loop();
        Ok(transport)
    }

    pub(crate) async fn node_addr(&self) -> Result<RemoteIrohNodeAddr, String> {
        self.endpoint
            .node_addr()
            .await
            .map(RemoteIrohNodeAddr::from_node_addr)
            .map_err(|error| error.to_string())
    }

    pub(crate) async fn shutdown(&self) {
        if let Ok(mut peers) = self.peers.lock() {
            peers.clear();
        }
        self.endpoint.close().await;
    }

    pub(crate) fn send(&self, data: Vec<u8>, device_id: Option<&str>) -> bool {
        let Some(device_id) = device_id else {
            crate::runtime_trace::runtime_trace("remote", "iroh_send drop reason=missing_device");
            return false;
        };
        let sent = self
            .peers
            .lock()
            .ok()
            .and_then(|peers| peers.get(device_id).cloned())
            .map(|tx| tx.send(data).is_ok())
            .unwrap_or(false);
        crate::runtime_trace::runtime_trace(
            "remote",
            &format!("iroh_send device={device_id} sent={sent}"),
        );
        sent
    }

    fn spawn_accept_loop(self: &Arc<Self>) {
        let owner = Arc::clone(self);
        crate::async_runtime::spawn(async move {
            while let Some(incoming) = owner.endpoint.accept().await {
                let owner = Arc::clone(&owner);
                crate::async_runtime::spawn(async move {
                    let Ok(connecting) = incoming.accept() else {
                        return;
                    };
                    let Ok(connection) = connecting.await else {
                        return;
                    };
                    owner.handle_connection(connection).await;
                });
            }
        });
    }

    async fn handle_connection(self: Arc<Self>, connection: Connection) {
        let peer = connection
            .remote_node_id()
            .map(|node| node.to_string())
            .unwrap_or_default();
        let (tx, mut rx) = mpsc::unbounded_channel::<Vec<u8>>();
        let mut device_id: Option<String> = None;
        loop {
            tokio::select! {
                inbound = connection.accept_bi() => {
                    let Ok((mut send, mut recv)) = inbound else {
                        break;
                    };
                    let Ok(data) = read_frame(&mut recv).await else {
                        break;
                    };
                    let Ok(raw) = serde_json::from_slice::<RemoteEnvelope>(&data) else {
                        let _ = write_frame(&mut send, br#"{"type":"error","payload":{"message":"Invalid remote envelope."}}"#).await;
                        let _ = send.finish();
                        continue;
                    };
                    if raw.kind == "pairing.request" {
                        if let Some(handshake) = pairing_handshake_from_envelope(&raw) {
                            device_id = Some(handshake.device_id.clone());
                            self.register_peer(&handshake.device_id, tx.clone());
                            (self.on_pairing)(handshake);
                        }
                    }
                    if device_id.is_none() {
                        device_id = raw
                            .device_id
                            .clone()
                            .filter(|value| !value.trim().is_empty());
                    }
                    if let Some(id) = device_id.clone() {
                        crate::runtime_trace::runtime_trace(
                            "remote",
                            &format!(
                                "iroh_recv raw_type={} device={} session={}",
                                raw.kind,
                                id,
                                raw.session_id.as_deref().unwrap_or("")
                            ),
                        );
                        self.register_peer(&id, tx.clone());
                        (self.on_message)(id, data);
                    }
                    let _ = write_frame(&mut send, br#"{"ok":true}"#).await;
                    let _ = send.finish();
                }
                outbound = rx.recv() => {
                    let Some(data) = outbound else {
                        break;
                    };
                    let Ok((mut send, _)) = connection.open_bi().await else {
                        break;
                    };
                    if write_frame(&mut send, &data).await.is_err() {
                        break;
                    }
                    let _ = send.finish();
                }
            }
        }
        if let Some(id) = device_id {
            let mut remove = false;
            if let Ok(peers) = self.peers.lock() {
                remove = peers
                    .get(&id)
                    .map(|peer_tx| peer_tx.same_channel(&tx))
                    .unwrap_or(false);
            }
            if remove {
                if let Ok(mut peers) = self.peers.lock() {
                    peers.remove(&id);
                }
                (self.on_state)(id, "closed".to_string());
            }
        } else if !peer.is_empty() {
            (self.on_state)(peer, "closed".to_string());
        }
    }

    fn register_peer(&self, device_id: &str, tx: mpsc::UnboundedSender<Vec<u8>>) {
        if let Ok(mut peers) = self.peers.lock() {
            peers.insert(device_id.to_string(), tx);
        }
        crate::runtime_trace::runtime_trace("remote", &format!("iroh_peer device={device_id}"));
        (self.on_state)(device_id.to_string(), "connected".to_string());
    }
}

#[cfg(test)]
pub(crate) async fn iroh_client_send(
    addr: RemoteIrohNodeAddr,
    message: Vec<u8>,
) -> Result<Vec<u8>, String> {
    iroh_client_send_with_hold(addr, message, None).await
}

#[cfg(test)]
pub(crate) async fn iroh_client_send_with_hold(
    addr: RemoteIrohNodeAddr,
    message: Vec<u8>,
    hold: Option<std::time::Duration>,
) -> Result<Vec<u8>, String> {
    let endpoint = Endpoint::builder()
        .relay_mode(RelayMode::Default)
        .bind()
        .await
        .map_err(|error| error.to_string())?;
    let connection = endpoint
        .connect(addr.to_node_addr()?, CODUX_REMOTE_ALPN)
        .await
        .map_err(|error| error.to_string())?;
    let (mut send, mut recv) = connection
        .open_bi()
        .await
        .map_err(|error| error.to_string())?;
    write_frame(&mut send, &message).await?;
    send.finish().map_err(|error| error.to_string())?;
    let response = read_frame(&mut recv).await?;
    if let Some(hold) = hold {
        tokio::time::sleep(hold).await;
    }
    connection.close(0_u32.into(), b"done");
    endpoint.close().await;
    Ok(response)
}

async fn write_frame<W>(writer: &mut W, data: &[u8]) -> Result<(), String>
where
    W: AsyncWriteExt + Unpin,
{
    let len = u32::try_from(data.len()).map_err(|_| "Remote message is too large.".to_string())?;
    writer
        .write_all(&len.to_be_bytes())
        .await
        .map_err(|error| error.to_string())?;
    writer
        .write_all(data)
        .await
        .map_err(|error| error.to_string())
}

async fn read_frame<R>(reader: &mut R) -> Result<Vec<u8>, String>
where
    R: AsyncReadExt + Unpin,
{
    let mut header = [0_u8; 4];
    reader
        .read_exact(&mut header)
        .await
        .map_err(|error| error.to_string())?;
    let len = u32::from_be_bytes(header) as usize;
    if len > 8 * 1024 * 1024 {
        return Err("Remote message is too large.".to_string());
    }
    let mut data = vec![0_u8; len];
    reader
        .read_exact(&mut data)
        .await
        .map_err(|error| error.to_string())?;
    Ok(data)
}

fn pairing_handshake_from_envelope(envelope: &RemoteEnvelope) -> Option<RemoteIrohHandshake> {
    let device_id = envelope
        .device_id
        .clone()
        .filter(|value| !value.trim().is_empty())?;
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
    Some(RemoteIrohHandshake {
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

#[cfg(test)]
pub(crate) fn iroh_pairing_request_payload(
    pairing_id: &str,
    code: &str,
    secret: &str,
    name: &str,
    public_key: &str,
) -> Value {
    json!({
        "pairingId": pairing_id,
        "code": code,
        "secret": secret,
        "deviceName": name,
        "devicePublicKey": public_key,
    })
}
