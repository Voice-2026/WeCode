use codux_protocol::{
    REMOTE_TRANSPORT_PING, REMOTE_TRANSPORT_PONG, RemoteEnvelope, RemoteOutgoingEnvelope,
    RemoteTransportPairingRequest,
};
use serde_json::Value;

pub(crate) fn transport_pong_for_ping(
    envelope: &RemoteEnvelope,
    fallback_device_id: Option<&str>,
) -> Option<String> {
    if envelope.kind != REMOTE_TRANSPORT_PING {
        return None;
    }
    let device_id = envelope
        .device_id
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| fallback_device_id.filter(|value| !value.trim().is_empty()))
        .map(str::to_string);
    serde_json::to_string(&RemoteOutgoingEnvelope {
        kind: REMOTE_TRANSPORT_PONG.to_string(),
        device_id,
        session_id: None,
        seq: None,
        payload: envelope.payload.clone(),
    })
    .ok()
}

pub(crate) fn pairing_handshake_from_envelope(
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
    Some(RemoteTransportPairingRequest {
        device_id,
        device_name,
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
