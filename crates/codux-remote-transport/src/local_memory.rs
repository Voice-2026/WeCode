use crate::{RemoteTransport, RemoteTransportMessageHandler, RemoteTransportStateHandler};
use async_trait::async_trait;
use codux_protocol::RemoteTransportKind;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

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
