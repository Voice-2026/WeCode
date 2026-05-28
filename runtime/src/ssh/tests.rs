use super::helpers::{render_ssh_launch_context_for_profiles, sanitize_request};
use super::*;
use std::sync::Mutex;

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
