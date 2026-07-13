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
    /// User-defined label shown by the desktop UI.
    #[serde(default)]
    pub note: Option<String>,
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
    /// The only peer currently allowed to drive its bound terminal.
    #[serde(default)]
    pub active_chat_id: Option<String>,
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
        let mut state = std::fs::read_to_string(&path)
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default();
        let migrated = normalize_active_binding(&mut state);
        let store = Self { path, state };
        if migrated {
            store.save();
        }
        store
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

    pub fn active_chat_id(&self) -> Option<&str> {
        self.state.active_chat_id.as_deref()
    }

    pub fn is_active(&self, chat_id: &str) -> bool {
        self.active_chat_id() == Some(chat_id)
    }

    /// Select the only bound peer allowed to drive a terminal.
    pub fn set_active(&mut self, chat_id: &str) -> bool {
        if !self.state.bindings.contains_key(chat_id) {
            return false;
        }
        if self.state.active_chat_id.as_deref() != Some(chat_id) {
            self.state.active_chat_id = Some(chat_id.to_string());
            self.save();
        }
        true
    }

    pub fn set_note(&mut self, chat_id: &str, note: &str) -> bool {
        let Some(binding) = self.state.bindings.get_mut(chat_id) else {
            return false;
        };
        let note = note.trim();
        let next = (!note.is_empty()).then(|| note.chars().take(64).collect::<String>());
        if binding.note != next {
            binding.note = next;
            self.save();
        }
        true
    }

    /// True if `chat_id` is on the allowlist.
    pub fn is_allowed(&self, chat_id: &str) -> bool {
        self.state.allowlist.iter().any(|id| id == chat_id)
    }

    /// Create or replace a binding and add the peer to the allowlist.
    pub fn bind(&mut self, mut binding: ChatBinding) {
        let chat_id = binding.chat_id.clone();
        if binding.note.is_none() {
            binding.note = self
                .state
                .bindings
                .get(&chat_id)
                .and_then(|existing| existing.note.clone());
        }
        if !self.state.allowlist.iter().any(|id| id == &chat_id) {
            self.state.allowlist.push(chat_id.clone());
        }
        self.state.bindings.insert(chat_id.clone(), binding);
        self.state.active_chat_id = Some(chat_id);
        self.save();
    }

    /// Remove a binding (the peer stays on the allowlist).
    pub fn unbind(&mut self, chat_id: &str) {
        self.state.bindings.remove(chat_id);
        normalize_active_binding(&mut self.state);
        self.save();
    }

    /// Remove a peer from the allowlist and drop its binding.
    pub fn revoke(&mut self, chat_id: &str) {
        self.state.allowlist.retain(|id| id != chat_id);
        self.state.bindings.remove(chat_id);
        normalize_active_binding(&mut self.state);
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
    /// Peer remains bound, but another peer is currently active.
    Inactive,
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
                if store.is_active(chat_id) {
                    AccessDecision::Bound
                } else {
                    AccessDecision::Inactive
                }
            } else if store.is_allowed(chat_id) {
                AccessDecision::NeedsBinding
            } else {
                AccessDecision::Rejected
            }
        }
        AccessPolicy::Pairing => {
            if store.binding(chat_id).is_some() {
                if store.is_active(chat_id) {
                    AccessDecision::Bound
                } else {
                    AccessDecision::Inactive
                }
            } else if store.is_allowed(chat_id) {
                AccessDecision::NeedsBinding
            } else {
                AccessDecision::NeedsPairing
            }
        }
    }
}

fn normalize_active_binding(state: &mut BindingState) -> bool {
    if state
        .active_chat_id
        .as_ref()
        .is_some_and(|chat_id| state.bindings.contains_key(chat_id))
    {
        return false;
    }
    let next = state
        .bindings
        .values()
        .max_by(|left, right| {
            left.created_at
                .cmp(&right.created_at)
                .then_with(|| left.chat_id.cmp(&right.chat_id))
        })
        .map(|binding| binding.chat_id.clone());
    let changed = state.active_chat_id != next;
    state.active_chat_id = next;
    changed
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
            note: None,
            created_at: 0,
        });
        assert_eq!(decide_access(&store, "peer1"), AccessDecision::Bound);
    }

    #[test]
    fn only_the_selected_binding_routes_messages() {
        let (mut store, _g) = tmp_store();
        store.bind(ChatBinding {
            chat_id: "peer1".into(),
            session_id: "sess1".into(),
            workspace_id: None,
            note: None,
            created_at: 1,
        });
        store.bind(ChatBinding {
            chat_id: "peer2".into(),
            session_id: "sess2".into(),
            workspace_id: None,
            note: None,
            created_at: 2,
        });

        assert_eq!(decide_access(&store, "peer1"), AccessDecision::Inactive);
        assert_eq!(decide_access(&store, "peer2"), AccessDecision::Bound);
        assert!(store.set_active("peer1"));
        assert_eq!(decide_access(&store, "peer1"), AccessDecision::Bound);
        assert_eq!(decide_access(&store, "peer2"), AccessDecision::Inactive);
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
            note: None,
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
            note: None,
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
            note: None,
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
                note: None,
                created_at: 42,
            });
        }
        let store = BindingStore::load(&path);
        let b = store.binding("peer1").expect("binding persisted");
        assert_eq!(b.session_id, "sess1");
        assert_eq!(b.workspace_id.as_deref(), Some("proj"));
        assert_eq!(store.active_chat_id(), Some("peer1"));
    }

    #[test]
    fn binding_note_persists_and_survives_rebinding() {
        let guard = tempdir::Guard::new();
        let path = guard.path.join("bindings.json");
        let mut store = BindingStore::load(&path);
        store.bind(ChatBinding {
            chat_id: "peer1".into(),
            session_id: "sess1".into(),
            workspace_id: None,
            note: None,
            created_at: 1,
        });
        assert!(store.set_note("peer1", "我的手机"));
        store.bind(ChatBinding {
            chat_id: "peer1".into(),
            session_id: "sess2".into(),
            workspace_id: None,
            note: None,
            created_at: 2,
        });

        let store = BindingStore::load(&path);
        let binding = store.binding("peer1").unwrap();
        assert_eq!(binding.note.as_deref(), Some("我的手机"));
        assert_eq!(binding.session_id, "sess2");
    }

    #[test]
    fn existing_bindings_migrate_to_the_most_recent_active_peer() {
        let guard = tempdir::Guard::new();
        let path = guard.path.join("bindings.json");
        std::fs::write(
            &path,
            r#"{
                "policy": "pairing",
                "bindings": {
                    "peer1": {"chat_id":"peer1","session_id":"sess1","created_at":1},
                    "peer2": {"chat_id":"peer2","session_id":"sess2","created_at":2}
                },
                "allowlist": ["peer1", "peer2"]
            }"#,
        )
        .unwrap();

        let store = BindingStore::load(&path);
        assert_eq!(store.active_chat_id(), Some("peer2"));
        let persisted: BindingState =
            serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        assert_eq!(persisted.active_chat_id.as_deref(), Some("peer2"));
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
