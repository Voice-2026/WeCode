use super::helpers::{render_ssh_launch_context_for_profiles, sanitize_request};
use super::*;
use serde_json::Value;
use std::fs;
use std::sync::Mutex;
use uuid::Uuid;

fn profile_with_secret() -> SSHConnectionProfile {
    SSHConnectionProfile {
        id: "profile-1".to_string(),
        name: "Production".to_string(),
        host: "example.com".to_string(),
        port: 2222,
        username: "root".to_string(),
        credential_kind: "password".to_string(),
        private_key_path: "/Users/me/.ssh/id_ed25519".to_string(),
        updated_at: 1,
        password: Some("secret-password".to_string()),
        key_passphrase: Some("secret-passphrase".to_string()),
    }
}

#[test]
fn password_profiles_require_password() {
    let result = sanitize_request(SSHProfileUpsertRequest {
        id: None,
        name: "Production".to_string(),
        host: "example.com".to_string(),
        port: 22,
        username: "root".to_string(),
        credential_kind: "password".to_string(),
        private_key_path: None,
        password: None,
        key_passphrase: None,
    });
    assert!(result.is_err());
}

#[test]
fn launch_context_lists_profiles_without_secrets() {
    let mut profiles = vec![profile_with_secret()];
    let context = render_ssh_launch_context_for_profiles(&mut profiles, None).unwrap();
    assert!(context.contains("codux-ssh list"));
    assert!(context.contains("codux-ssh <profile-id>"));
    assert!(context.contains("codux-ssh <profile-id> -- '<remote-command>'"));
    assert!(context.contains("do not look for or use `codux` or `dmux`"));
    assert!(context.contains("Production"));
    assert!(context.contains("root@example.com:2222"));
    assert!(context.contains("profile-1"));
    assert!(!context.contains("secret-password"));
    assert!(!context.contains("secret-passphrase"));
    assert!(!context.contains("/Users/me/.ssh/id_ed25519"));
}

#[test]
fn launch_command_only_references_profile_id() {
    let profile = profile_with_secret();
    let store = SSHStore {
        profiles: Mutex::new(vec![profile]),
        state_file: PathBuf::from("/tmp/codux-ssh-test.json"),
    };
    let command = store.launch_command("profile-1".to_string()).unwrap();
    assert!(command.command.contains("codux-ssh"));
    assert!(command.command.contains("profile-1"));
    assert!(!command.command.contains("secret-password"));
    assert!(!command.command.contains("secret-passphrase"));
}

#[test]
fn ssh_store_uses_shared_config_document_snapshot() {
    let support_dir = std::env::temp_dir().join(format!("codux-ssh-store-{}", Uuid::new_v4()));
    fs::create_dir_all(&support_dir).unwrap();
    let store = SSHStore::from_support_dir(support_dir.clone());

    store
        .upsert(SSHProfileUpsertRequest {
            id: Some("profile-1".to_string()),
            name: "Production".to_string(),
            host: "example.com".to_string(),
            port: 2222,
            username: "root".to_string(),
            credential_kind: "password".to_string(),
            private_key_path: None,
            password: Some("secret-password".to_string()),
            key_passphrase: None,
        })
        .unwrap();

    let path = ssh_profiles_file_path_in(support_dir.clone());
    let raw = crate::config::ConfigDocumentStore::for_file(path).snapshot();
    let profiles = raw.as_array().expect("ssh profiles root array");
    assert_eq!(profiles.len(), 1);
    assert_eq!(
        profiles[0].get("id").and_then(Value::as_str),
        Some("profile-1")
    );

    fs::remove_dir_all(support_dir).ok();
}
