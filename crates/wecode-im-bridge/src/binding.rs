//! Chat-to-session bindings and access control.
//!
//! A *binding* maps one chat peer (a WeChat `from_user_id`) to one WeCode
//! terminal session. Access control gates who may bind at all: a first-time
//! peer must present a pairing code shown in the desktop UI, and an optional
//! allowlist locks the bridge to already-approved peers.
//!
//! Bindings and the allowlist persist as JSON so they survive restarts. The
//! store is deliberately storage-only; routing decisions live in the runtime.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Access policy for new chat peers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccessPolicy {
    /// New peers may request a pairing code and bind after it is confirmed.
    Pairing,
    /// Only peers already on the allowlist may interact; no new pairing.
    Allowlist,
    /// The bridge ignores all inbound messages.
    Disabled,
}

impl Default for AccessPolicy {
    fn default() -> Self {
        AccessPolicy::Pairing
    }
}

/// One chat peer bound to a WeCode terminal session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatBinding {
    /// The chat peer id (WeChat `from_user_id`).
    pub chat_id: String,
    /// The WeCode terminal session id this peer drives.
    pub session_id: String,
    /// Optional workspace/project id the session belongs to.
    #[serde(default)]
    pub workspace_id: Option<String>,
    /// Epoch millis when the binding was created.
    #[serde(default)]
    pub created_at: u64,
}

/// Persisted bridge access state: policy, bindings, and the allowlist.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BindingState {
    #[serde(default)]
    pub policy: AccessPolicy,
    #[serde(default)]
    pub bindings: HashMap<String, ChatBinding>,
    #[serde(default)]
    pub allowlist: Vec<String>,
}

/// JSON-backed store for [`BindingState`].
pub struct BindingStore {
    path: PathBuf,
    state: BindingState,
}

impl BindingStore {
    /// Load state from `path`, or start empty if it does not exist.
    pub fn load(path: impl Into<PathBuf>) -> Self {
        let path = path.into();
        let state = std::fs::read_to_string(&path)
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default();
        Self { path, state }
    }

    /// A snapshot of the current state.
    pub fn state(&self) -> &BindingState {
        &self.state
    }

    /// The active access policy.
    pub fn policy(&self) -> AccessPolicy {
        self.state.policy
    }

    /// Set the access policy and persist.
    pub fn set_policy(&mut self, policy: AccessPolicy) {
        self.state.policy = policy;
        self.save();
    }

    /// The binding for `chat_id`, if any.
    pub fn binding(&self, chat_id: &str) -> Option<&ChatBinding> {
        self.state.bindings.get(chat_id)
    }

    /// True if `chat_id` is on the allowlist.
    pub fn is_allowed(&self, chat_id: &str) -> bool {
        self.state.allowlist.iter().any(|id| id == chat_id)
    }

    /// Create or replace a binding and add the peer to the allowlist.
    pub fn bind(&mut self, binding: ChatBinding) {
        let chat_id = binding.chat_id.clone();
        if !self.state.allowlist.iter().any(|id| id == &chat_id) {
            self.state.allowlist.push(chat_id.clone());
        }
        self.state.bindings.insert(chat_id, binding);
        self.save();
    }

    /// Remove a binding (the peer stays on the allowlist).
    pub fn unbind(&mut self, chat_id: &str) {
        self.state.bindings.remove(chat_id);
        self.save();
    }

    /// Remove a peer from the allowlist and drop its binding.
    pub fn revoke(&mut self, chat_id: &str) {
        self.state.allowlist.retain(|id| id != chat_id);
        self.state.bindings.remove(chat_id);
        self.save();
    }

    fn save(&self) {
        write_json_600(&self.path, &self.state);
    }
}

/// Whether a peer may proceed given the current policy and state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessDecision {
    /// Peer has an existing binding; route normally.
    Bound,
    /// Peer is allowed but not yet bound; the host must create a binding.
    NeedsBinding,
    /// Peer must present a pairing code before binding.
    NeedsPairing,
    /// Peer is rejected (allowlist mode, unknown peer, or disabled).
    Rejected,
}

/// Decide what to do with an inbound message from `chat_id`.
pub fn decide_access(store: &BindingStore, chat_id: &str) -> AccessDecision {
    match store.policy() {
        AccessPolicy::Disabled => AccessDecision::Rejected,
        AccessPolicy::Allowlist => {
            if store.binding(chat_id).is_some() {
                AccessDecision::Bound
            } else if store.is_allowed(chat_id) {
                AccessDecision::NeedsBinding
            } else {
                AccessDecision::Rejected
            }
        }
        AccessPolicy::Pairing => {
            if store.binding(chat_id).is_some() {
                AccessDecision::Bound
            } else if store.is_allowed(chat_id) {
                AccessDecision::NeedsBinding
            } else {
                AccessDecision::NeedsPairing
            }
        }
    }
}

/// Write `value` as pretty JSON with owner-only permissions (0600 on Unix).
pub fn write_json_600<T: Serialize>(path: &Path, value: &T) {
    let Ok(text) = serde_json::to_string_pretty(value) else {
        return;
    };
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if std::fs::write(path, text).is_ok() {
        set_owner_only(path);
    }
}

#[cfg(unix)]
fn set_owner_only(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600));
}

#[cfg(not(unix))]
fn set_owner_only(_path: &Path) {}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_store() -> (BindingStore, tempdir::Guard) {
        let guard = tempdir::Guard::new();
        let store = BindingStore::load(guard.path.join("bindings.json"));
        (store, guard)
    }

    #[test]
    fn pairing_policy_requires_code_for_new_peer() {
        let (store, _g) = tmp_store();
        assert_eq!(decide_access(&store, "peer1"), AccessDecision::NeedsPairing);
    }

    #[test]
    fn bound_peer_routes_normally() {
        let (mut store, _g) = tmp_store();
        store.bind(ChatBinding {
            chat_id: "peer1".into(),
            session_id: "sess1".into(),
            workspace_id: None,
            created_at: 0,
        });
        assert_eq!(decide_access(&store, "peer1"), AccessDecision::Bound);
    }

    #[test]
    fn allowlist_policy_rejects_unknown_peer() {
        let (mut store, _g) = tmp_store();
        store.set_policy(AccessPolicy::Allowlist);
        assert_eq!(decide_access(&store, "peer1"), AccessDecision::Rejected);
    }

    #[test]
    fn allowed_but_unbound_needs_binding() {
        let (mut store, _g) = tmp_store();
        store.bind(ChatBinding {
            chat_id: "peer1".into(),
            session_id: "sess1".into(),
            workspace_id: None,
            created_at: 0,
        });
        store.unbind("peer1");
        assert_eq!(decide_access(&store, "peer1"), AccessDecision::NeedsBinding);
    }

    #[test]
    fn disabled_policy_rejects_everyone() {
        let (mut store, _g) = tmp_store();
        store.set_policy(AccessPolicy::Disabled);
        store.bind(ChatBinding {
            chat_id: "peer1".into(),
            session_id: "sess1".into(),
            workspace_id: None,
            created_at: 0,
        });
        assert_eq!(decide_access(&store, "peer1"), AccessDecision::Rejected);
    }

    #[test]
    fn revoke_removes_from_allowlist_and_binding() {
        let (mut store, _g) = tmp_store();
        store.bind(ChatBinding {
            chat_id: "peer1".into(),
            session_id: "sess1".into(),
            workspace_id: None,
            created_at: 0,
        });
        store.revoke("peer1");
        assert!(!store.is_allowed("peer1"));
        assert!(store.binding("peer1").is_none());
    }

    #[test]
    fn state_persists_across_reload() {
        let guard = tempdir::Guard::new();
        let path = guard.path.join("bindings.json");
        {
            let mut store = BindingStore::load(&path);
            store.bind(ChatBinding {
                chat_id: "peer1".into(),
                session_id: "sess1".into(),
                workspace_id: Some("proj".into()),
                created_at: 42,
            });
        }
        let store = BindingStore::load(&path);
        let b = store.binding("peer1").expect("binding persisted");
        assert_eq!(b.session_id, "sess1");
        assert_eq!(b.workspace_id.as_deref(), Some("proj"));
    }

    /// Minimal self-cleaning temp dir so tests don't pull a new dependency.
    mod tempdir {
        use std::path::PathBuf;

        pub struct Guard {
            pub path: PathBuf,
        }

        impl Guard {
            pub fn new() -> Self {
                let mut path = std::env::temp_dir();
                let unique = format!(
                    "wecode-im-bridge-test-{}-{}",
                    std::process::id(),
                    super::super::now_millis_for_test()
                );
                path.push(unique);
                std::fs::create_dir_all(&path).expect("create temp dir");
                Self { path }
            }
        }

        impl Drop for Guard {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.path);
            }
        }
    }
}

#[cfg(test)]
fn now_millis_for_test() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    COUNTER.fetch_add(1, Ordering::Relaxed)
}
