use serde_json::{Value, json};
use wecode_protocol::{REMOTE_PROTOCOL_VERSION, RemoteTransportCandidate, host_capabilities};

pub struct HostInfoPayload {
    pub host_id: String,
    pub runtime_instance_id: String,
    pub name: String,
    pub platform: String,
    pub app: String,
    pub transports: Vec<RemoteTransportCandidate>,
}

pub fn host_info_payload(input: HostInfoPayload) -> Value {
    json!({
        "hostId": input.host_id,
        "runtimeInstanceId": input.runtime_instance_id,
        "name": input.name,
        "platform": input.platform,
        "app": input.app,
        "protocolVersion": REMOTE_PROTOCOL_VERSION,
        "capabilities": host_capabilities(),
        "transports": input.transports,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use wecode_protocol::iroh_transport_candidate;

    #[test]
    fn host_info_payload_advertises_protocol_capabilities_and_transports() {
        let payload = host_info_payload(HostInfoPayload {
            host_id: "host-1".to_string(),
            runtime_instance_id: "runtime-1".to_string(),
            name: "WeCode Mac".to_string(),
            platform: "macos".to_string(),
            app: "WeCode".to_string(),
            transports: vec![iroh_transport_candidate(
                "https://relay.example/v3",
                "node-1",
                "https://relay.example",
            )],
        });

        assert_eq!(payload["hostId"], "host-1");
        assert_eq!(payload["runtimeInstanceId"], "runtime-1");
        assert_eq!(payload["protocolVersion"], REMOTE_PROTOCOL_VERSION);
        assert_eq!(payload["capabilities"]["domains"]["terminal"], true);
        assert_eq!(payload["transports"][0]["kind"], "iroh");
        assert_eq!(payload["transports"][0]["nodeId"], "node-1");
        assert_eq!(
            payload["transports"][0]["relayUrl"],
            "https://relay.example"
        );
    }
}
