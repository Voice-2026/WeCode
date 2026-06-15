use super::RemoteService;
use super::types::{RemoteEnvelope, RemoteOutgoingEnvelope};
use serde_json::Value;
use std::collections::HashMap;

impl RemoteService {
    pub fn parse_incoming_envelope(&self, text: &str) -> Result<RemoteEnvelope, String> {
        serde_json::from_str::<RemoteEnvelope>(text).map_err(|error| error.to_string())
    }

    pub fn outgoing_transport_text(
        &self,
        kind: &str,
        device_id: Option<&str>,
        session_id: Option<&str>,
        payload: Value,
        send_seq_by_device: &mut HashMap<String, i64>,
    ) -> Option<String> {
        let seq = device_id
            .filter(|value| !value.trim().is_empty())
            .map(|device_id| {
                let seq = send_seq_by_device.get(device_id).copied().unwrap_or(0) + 1;
                send_seq_by_device.insert(device_id.to_string(), seq);
                seq
            });
        let envelope = RemoteOutgoingEnvelope {
            kind: kind.to_string(),
            device_id: device_id.map(str::to_string),
            session_id: session_id.map(str::to_string),
            seq,
            payload,
        };
        serde_json::to_string(&envelope).ok()
    }
}
