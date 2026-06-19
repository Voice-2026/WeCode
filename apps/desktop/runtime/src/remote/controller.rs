//! Desktop-as-controller runtime: dial OUT to a remote host (another desktop's
//! `RemoteHostRuntime` or a headless agent) and drive its domains over Iroh.
//! This is the inverse of `RemoteHostRuntime`.
//!
//! Replies are correlated by message **type** — the host echoes a domain
//! specific reply type, not a `requestId`, for general domains — using a FIFO
//! waiter list, the same proven scheme the mobile controller and the agent use.
//! The request path is synchronous (send, then block on a channel that the
//! transport's message callback feeds) so it composes cleanly with the
//! synchronous `RuntimeService` domain methods that will route through it.

use codux_protocol::{
    REMOTE_AI_STATS, REMOTE_ERROR, REMOTE_FILE_CREATE_DIRECTORY, REMOTE_FILE_DIRECTORY_CREATED,
    REMOTE_FILE_LIST, REMOTE_GIT_STATUS, REMOTE_HOST_INFO, REMOTE_PROJECT_LIST,
    REMOTE_TRANSPORT_IROH,
};
use codux_remote_transport::{
    RemoteControllerTransportConfig, RemoteTransport, RemoteTransportCandidate,
};
use serde_json::{json, Value};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use super::transport_factory::RemoteTransportFactory;

const DEFAULT_REQUEST_TIMEOUT: Duration = Duration::from_secs(20);

/// Everything needed to dial a remote host — produced from a pairing ticket.
#[derive(Clone, Debug)]
pub struct RemoteControllerTarget {
    pub host_id: String,
    pub device_id: String,
    pub device_token: String,
    pub relay_url: String,
    pub node_id: String,
    pub ticket: String,
    pub relay_authentication: String,
}

struct Waiter {
    id: u64,
    expect: String,
    tx: Sender<Result<Value, String>>,
}

#[derive(Default)]
struct ControllerInner {
    waiters: Mutex<Vec<Waiter>>,
    events: Mutex<VecDeque<(String, Value)>>,
}

impl ControllerInner {
    /// Route one inbound envelope: resolve the first waiter expecting this reply
    /// type, fail the oldest waiter on `error`, or queue it as an unsolicited
    /// event (resource update, terminal output, broadcast).
    fn route(&self, data: &[u8]) {
        let Ok(envelope) = serde_json::from_slice::<Value>(data) else {
            return;
        };
        let kind = envelope
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        let payload = envelope.get("payload").cloned().unwrap_or(Value::Null);
        {
            let mut waiters = self.waiters.lock().unwrap();
            if kind == REMOTE_ERROR {
                if !waiters.is_empty() {
                    let waiter = waiters.remove(0);
                    let message = payload
                        .get("message")
                        .and_then(Value::as_str)
                        .unwrap_or("remote host error")
                        .to_string();
                    let _ = waiter.tx.send(Err(message));
                    return;
                }
            } else if let Some(index) = waiters.iter().position(|waiter| waiter.expect == kind) {
                let waiter = waiters.remove(index);
                let _ = waiter.tx.send(Ok(payload));
                return;
            }
        }
        self.events.lock().unwrap().push_back((kind, payload));
    }

    fn remove_waiter(&self, id: u64) {
        self.waiters.lock().unwrap().retain(|waiter| waiter.id != id);
    }
}

pub struct RemoteController {
    transport: Arc<dyn RemoteTransport>,
    device_id: String,
    inner: Arc<ControllerInner>,
    next_id: AtomicU64,
}

impl RemoteController {
    pub async fn connect(target: &RemoteControllerTarget) -> Result<Self, String> {
        let inner = Arc::new(ControllerInner::default());
        let config = RemoteControllerTransportConfig {
            relay_url: target.relay_url.clone(),
            host_id: target.host_id.clone(),
            device_id: target.device_id.clone(),
            device_token: target.device_token.clone(),
            transports: vec![RemoteTransportCandidate {
                kind: REMOTE_TRANSPORT_IROH.to_string(),
                url: String::new(),
                node_id: target.node_id.clone(),
                relay_url: target.relay_url.clone(),
                ticket: target.ticket.clone(),
                relay_authentication: target.relay_authentication.clone(),
            }],
        };
        let message_inner = Arc::clone(&inner);
        let transport = RemoteTransportFactory::connect_controller(
            &config,
            Arc::new(move |_source: String, data: Vec<u8>| message_inner.route(&data)),
            Arc::new(|_, _| {}),
        )
        .await?;
        Ok(Self {
            transport,
            device_id: target.device_id.clone(),
            inner,
            next_id: AtomicU64::new(1),
        })
    }

    /// Send a request and block until a reply of type `expect` arrives (or the
    /// host returns an error, or the request times out).
    pub fn request(&self, expect: &str, kind: &str, payload: Value) -> Result<Value, String> {
        self.request_with_timeout(expect, kind, payload, DEFAULT_REQUEST_TIMEOUT)
    }

    pub fn request_with_timeout(
        &self,
        expect: &str,
        kind: &str,
        payload: Value,
        timeout: Duration,
    ) -> Result<Value, String> {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let (tx, rx) = mpsc::channel();
        self.inner.waiters.lock().unwrap().push(Waiter {
            id,
            expect: expect.to_string(),
            tx,
        });
        let envelope = json!({ "type": kind, "deviceId": self.device_id, "payload": payload });
        let bytes = match serde_json::to_vec(&envelope) {
            Ok(bytes) => bytes,
            Err(error) => {
                self.inner.remove_waiter(id);
                return Err(error.to_string());
            }
        };
        if !self.transport.send(bytes, None) {
            self.inner.remove_waiter(id);
            return Err(format!("failed to send {kind} to remote host"));
        }
        match rx.recv_timeout(timeout) {
            Ok(result) => result,
            Err(_) => {
                self.inner.remove_waiter(id);
                Err(format!("timed out waiting for {expect} from remote host"))
            }
        }
    }

    /// Drain unsolicited messages (resource updates, terminal output, broadcasts).
    pub fn drain_events(&self) -> Vec<(String, Value)> {
        self.inner.events.lock().unwrap().drain(..).collect()
    }

    pub async fn shutdown(&self) {
        self.transport.shutdown().await;
    }

    // ---- Typed domain helpers -----------------------------------------------

    pub fn host_info(&self) -> Result<Value, String> {
        self.request(REMOTE_HOST_INFO, REMOTE_HOST_INFO, json!({}))
    }

    pub fn file_list(&self, path: Option<&str>, purpose: Option<&str>) -> Result<Value, String> {
        let mut payload = json!({});
        if let Some(path) = path {
            payload["path"] = json!(path);
        }
        if let Some(purpose) = purpose {
            payload["purpose"] = json!(purpose);
        }
        self.request(REMOTE_FILE_LIST, REMOTE_FILE_LIST, payload)
    }

    pub fn create_directory(&self, path: &str) -> Result<Value, String> {
        self.request(
            REMOTE_FILE_DIRECTORY_CREATED,
            REMOTE_FILE_CREATE_DIRECTORY,
            json!({ "path": path }),
        )
    }

    pub fn git_status(&self, project_id: &str, project_path: &str) -> Result<Value, String> {
        self.request(
            REMOTE_GIT_STATUS,
            REMOTE_GIT_STATUS,
            json!({ "projectId": project_id, "projectPath": project_path }),
        )
    }

    pub fn ai_stats(&self, project_id: &str) -> Result<Value, String> {
        self.request(REMOTE_AI_STATS, REMOTE_AI_STATS, json!({ "projectId": project_id }))
    }

    pub fn project_list(&self) -> Result<Value, String> {
        self.request(REMOTE_PROJECT_LIST, REMOTE_PROJECT_LIST, json!({}))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn waiter(inner: &ControllerInner, expect: &str) -> mpsc::Receiver<Result<Value, String>> {
        let (tx, rx) = mpsc::channel();
        inner.waiters.lock().unwrap().push(Waiter {
            id: 1,
            expect: expect.to_string(),
            tx,
        });
        rx
    }

    #[test]
    fn route_resolves_waiter_by_reply_type() {
        let inner = ControllerInner::default();
        let rx = waiter(&inner, REMOTE_GIT_STATUS);
        inner.route(br#"{"type":"git.status","payload":{"isRepository":true}}"#);
        let result = rx.recv_timeout(Duration::from_secs(1)).unwrap();
        assert_eq!(result.unwrap()["isRepository"], json!(true));
    }

    #[test]
    fn route_error_fails_oldest_waiter() {
        let inner = ControllerInner::default();
        let rx = waiter(&inner, REMOTE_FILE_LIST);
        inner.route(br#"{"type":"error","payload":{"message":"nope"}}"#);
        let result = rx.recv_timeout(Duration::from_secs(1)).unwrap();
        assert_eq!(result, Err("nope".to_string()));
    }

    #[test]
    fn route_unmatched_reply_is_queued_as_event() {
        let inner = ControllerInner::default();
        inner.route(br#"{"type":"terminal.output","payload":{"data":"x"}}"#);
        let events = inner.events.lock().unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].0, "terminal.output");
    }

    #[test]
    fn route_matches_same_type_waiters_fifo() {
        let inner = ControllerInner::default();
        let (tx1, rx1) = mpsc::channel();
        let (tx2, rx2) = mpsc::channel();
        {
            let mut waiters = inner.waiters.lock().unwrap();
            waiters.push(Waiter { id: 1, expect: REMOTE_FILE_LIST.to_string(), tx: tx1 });
            waiters.push(Waiter { id: 2, expect: REMOTE_FILE_LIST.to_string(), tx: tx2 });
        }
        inner.route(br#"{"type":"file.list","payload":{"n":1}}"#);
        inner.route(br#"{"type":"file.list","payload":{"n":2}}"#);
        assert_eq!(rx1.recv_timeout(Duration::from_secs(1)).unwrap().unwrap()["n"], json!(1));
        assert_eq!(rx2.recv_timeout(Duration::from_secs(1)).unwrap().unwrap()["n"], json!(2));
    }
}
