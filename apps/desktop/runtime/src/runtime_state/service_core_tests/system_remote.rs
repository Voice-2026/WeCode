#[test]
fn file_watch_events_are_queued_and_drained_for_gpui() {
    let support_dir =
        std::env::temp_dir().join(format!("wecode-file-watch-events-{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(&support_dir).expect("create support dir");
    let service = RuntimeService::new(PathBuf::from(&support_dir));

    service
        .file_watch_events
        .lock()
        .expect("file event queue")
        .push_back(FileChangeEvent {
            project_path: "/tmp/project".to_string(),
            changed_paths: vec!["/tmp/project/src/main.rs".to_string()],
        });

    let events = service.drain_file_change_events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].project_path, "/tmp/project");
    assert!(service.drain_file_change_events().is_empty());

    let _ = fs::remove_dir_all(support_dir);
}

#[test]
fn revoke_remote_device_preserves_connected_host_snapshot() {
    let support_dir =
        std::env::temp_dir().join(format!("wecode-revoke-remote-{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(&support_dir).expect("create support dir");
    fs::write(
        support_dir.join("settings.json"),
        serde_json::to_string_pretty(&serde_json::json!({
            "remote": {
                "isEnabled": true,
                "relayUrl": crate::remote::remote_relay_url_for_preset("china-tencent", ""),
                "hostID": "host-1",
                "hostToken": "secret-token",
                "cachedDevices": [
                    {"id": "device-1", "hostId": "host-1", "name": "Phone", "online": true}
                ]
            }
        }))
        .expect("settings json"),
    )
    .expect("write settings");

    let service = RuntimeService::new(PathBuf::from(&support_dir));
    let mut connected = service.remote_host.reload_snapshot_from_settings();
    connected.status = "connected".to_string();
    connected.message = "Remote transport connected.".to_string();
    service.remote_host.apply_snapshot(connected);

    let summary = service
        .revoke_remote_device("device-1")
        .expect("revoke device");

    assert_eq!(summary.status, "connected");
    assert_eq!(summary.message, "Remote transport connected.");
    assert_eq!(summary.devices, 0);
    assert!(summary.device_list.is_empty());

    let _ = fs::remove_dir_all(support_dir);
}

#[test]
fn runtime_dock_badge_count_matches_tauri_attention_semantics() {
    let mut snapshot = AIRuntimeStateSnapshot::default();

    assert_eq!(runtime_dock_badge_count(true, &snapshot), None);

    snapshot.needs_input_count = 2;
    snapshot.completion_count = 3;

    assert_eq!(runtime_dock_badge_count(true, &snapshot), Some(5));
    assert_eq!(runtime_dock_badge_count(false, &snapshot), None);
}
