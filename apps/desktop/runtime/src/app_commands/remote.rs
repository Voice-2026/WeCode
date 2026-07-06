use super::*;

pub fn remote_status(service: &RuntimeService) -> RemoteSummary {
    service.reload_remote()
}
pub fn remote_snapshot_emit(service: &RuntimeService) -> RemoteSummary {
    service.reload_remote()
}
pub fn remote_reconnect(service: &RuntimeService) -> Result<RemoteSummary, String> {
    service.reconnect_remote()
}
pub fn remote_devices_refresh(service: &RuntimeService) -> Result<RemoteSummary, String> {
    service.refresh_remote_devices()
}
pub fn remote_device_revoke(
    service: &RuntimeService,
    device_id: String,
) -> Result<RemoteSummary, String> {
    service.revoke_remote_device(&device_id)
}
pub fn remote_pairing_create(service: &RuntimeService) -> Result<RemoteSummary, String> {
    service.create_remote_pairing()
}
pub fn remote_pairing_cancel(
    service: &RuntimeService,
    pairing_id: String,
) -> Result<RemoteSummary, String> {
    service.cancel_remote_pairing(&pairing_id)
}
pub fn remote_pairing_confirm(
    service: &RuntimeService,
    pairing_id: String,
) -> Result<RemoteSummary, String> {
    service.confirm_remote_pairing(&pairing_id)
}
pub fn remote_pairing_reject(
    service: &RuntimeService,
    pairing_id: String,
) -> Result<RemoteSummary, String> {
    service.reject_remote_pairing(&pairing_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use uuid::Uuid;

    #[test]
    fn remote_commands_delegate_to_runtime_service_without_network_when_disabled() {
        let support_dir =
            std::env::temp_dir().join(format!("codux-app-command-remote-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&support_dir).expect("support dir");
        std::fs::write(
            support_dir.join("settings.json"),
            serde_json::to_string_pretty(&json!({
                "remote": {
                    "isEnabled": false,
                    "relayUrl": "http://relay.example"
                }
            }))
            .expect("settings json"),
        )
        .expect("write settings");

        let service = RuntimeService::new(support_dir.clone());
        let status = remote_status(&service);
        assert!(!status.enabled);
        assert_eq!(status.status, "stopped");

        let emitted = remote_snapshot_emit(&service);
        assert_eq!(emitted.status, "stopped");

        let reconnected = remote_reconnect(&service).expect("disabled reconnect");
        assert!(!reconnected.enabled);

        let refreshed = remote_devices_refresh(&service).expect("disabled refresh");
        assert_eq!(refreshed.devices, 0);

        assert!(
            remote_device_revoke(&service, String::new())
                .expect_err("missing device id")
                .contains("Missing device id")
        );
        assert!(
            remote_pairing_create(&service)
                .expect_err("disabled pairing")
                .contains("Remote Host is disabled")
        );
        assert!(
            remote_pairing_cancel(&service, String::new())
                .expect_err("missing cancel id")
                .contains("Missing pairing id")
        );
        assert!(
            remote_pairing_confirm(&service, String::new())
                .expect_err("missing confirm id")
                .contains("Missing pairing id")
        );
        assert!(
            remote_pairing_reject(&service, String::new())
                .expect_err("missing reject id")
                .contains("Missing pairing id")
        );

        let _ = std::fs::remove_dir_all(support_dir);
    }
}
