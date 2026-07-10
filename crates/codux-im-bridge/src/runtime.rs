//! WeChat bridge runtime: the long-poll loop, status machine, and message
//! routing that connects incoming chat messages to a Codux session.
//!
//! The runtime is host-agnostic. It talks to the desktop/agent only through
//! [`HostSink`], which the host implements to write text into a bound terminal
//! session and to report which sessions exist. This keeps the crate free of
//! GPUI/app dependencies per the workspace rules.

use std::sync::Arc;
use std::time::Duration;

use tokio::sync::Mutex;

use crate::binding::{decide_access, AccessDecision, BindingStore, ChatBinding};
use crate::wechat::{
    message_state, message_text, message_type, ILinkClient, QrScanOutcome, WeChatCredentials,
    DEFAULT_BASE_URL, SESSION_EXPIRED_CODE,
};
use crate::Result;

const MAX_TEXT_LEN: usize = 4000;
const INITIAL_BACKOFF: Duration = Duration::from_secs(3);
const MAX_BACKOFF: Duration = Duration::from_secs(60);
const MAX_CONSECUTIVE_FAILURES: u32 = 5;

/// Live status of the WeChat bridge, mirrored to the UI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BridgeStatus {
    /// Not connected and not attempting to.
    Disconnected,
    /// A QR code has been fetched; awaiting scan. Carries the scan URL.
    WaitingScan { scan_url: String },
    /// QR scanned, awaiting confirmation.
    Scanned,
    /// Credentials obtained, establishing the poll loop.
    Connecting,
    /// Long-poll loop is running.
    Connected,
    /// Stopped with an error message.
    Error { message: String },
}

/// How the host is asked to act on an incoming, authorized message.
pub trait HostSink: Send + Sync {
    /// Write user text into the bound terminal session. Returns false if the
    /// session no longer exists so the runtime can drop the stale binding.
    fn write_to_session(&self, session_id: &str, text: &str) -> bool;

    /// Report bridge status changes so the UI can update.
    fn on_status(&self, status: BridgeStatus);

    /// A new, unknown peer asked to pair. The host shows the peer + code in its
    /// UI so the user can confirm with [`WeChatBridge::confirm_pairing`].
    fn on_pairing_request(&self, _chat_id: &str, _code: &str) {}

    /// A long-poll response arrived. Hosts can use this for diagnostics.
    fn on_poll_result(&self, _message_count: usize, _ret: i32, _errcode: Option<i32>) {}

    /// A long-poll request is about to start.
    fn on_poll_start(&self, _cursor_len: usize) {}

    /// A long-poll request failed before producing a response.
    fn on_poll_error(&self, _error: &str, _failures: u32) {}

    /// An incoming message was observed before routing filters are applied.
    fn on_incoming_message(
        &self,
        _chat_id: &str,
        _message_type: i32,
        _message_state: i32,
        _text_len: usize,
    ) {
    }

    /// A bound peer sent a message to a session. Hosts can use this to attach
    /// output relays lazily after the first real chat interaction.
    fn on_bound_message(&self, _chat_id: &str, _session_id: &str) {}

    /// Called immediately before a bound message is written to the terminal.
    /// Hosts that mirror terminal output should subscribe here so very fast
    /// commands cannot finish before the relay is attached.
    fn on_bound_message_start(&self, _chat_id: &str, _session_id: &str, _text: &str) {}

    /// The desktop approved a pairing and bound the peer to a session.
    fn on_pairing_confirmed(&self, _chat_id: &str, _session_id: &str) {}
}

/// Persistence hooks for credentials and the long-poll sync cursor. The host
/// owns file locations and encryption; the runtime only calls these.
pub trait CredentialStore: Send + Sync {
    fn load_credentials(&self) -> Option<WeChatCredentials>;
    fn save_credentials(&self, creds: &WeChatCredentials);
    fn clear_credentials(&self);
    fn load_sync_cursor(&self) -> String;
    fn save_sync_cursor(&self, cursor: &str);
}

/// A peer that has been offered a pairing code and awaits desktop approval.
#[derive(Debug, Clone)]
pub struct PendingPairing {
    pub chat_id: String,
    pub code: String,
    /// Context token of the peer's last message, used to send the
    /// confirmation reply once the desktop approves.
    pub context_token: String,
}

/// User-facing reply strings, injected so the host controls i18n.
#[derive(Debug, Clone)]
pub struct ReplyText {
    /// Sent to a new peer with a pairing code, e.g. "配对码：{code} ...".
    pub pairing_prompt: String,
    /// Sent to an allowed-but-unbound peer, e.g. "请在 Codux 中选择会话".
    pub needs_binding: String,
    /// Sent to a rejected peer, e.g. "未授权".
    pub rejected: String,
    /// Sent after the desktop confirms a pairing, e.g. "绑定成功".
    pub pairing_confirmed: String,
}

/// Shared runtime state guarded for the async loop.
struct Inner {
    client: Option<ILinkClient>,
    sync_cursor: String,
    running: bool,
}

/// The WeChat bridge. Cheap to clone; all state is shared behind an `Arc`.
#[derive(Clone)]
pub struct WeChatBridge {
    http: reqwest::Client,
    inner: Arc<Mutex<Inner>>,
    bindings: Arc<Mutex<BindingStore>>,
    pending: Arc<std::sync::Mutex<std::collections::HashMap<String, PendingPairing>>>,
    last_context_tokens: Arc<std::sync::Mutex<std::collections::HashMap<String, String>>>,
    sink: Arc<dyn HostSink>,
    creds: Arc<dyn CredentialStore>,
    reply: Arc<ReplyText>,
}

impl WeChatBridge {
    /// Build a bridge. `bindings_path` backs the chat-to-session store.
    pub fn new(
        bindings: BindingStore,
        sink: Arc<dyn HostSink>,
        creds: Arc<dyn CredentialStore>,
        reply: ReplyText,
    ) -> Self {
        Self {
            http: reqwest::Client::new(),
            inner: Arc::new(Mutex::new(Inner {
                client: None,
                sync_cursor: String::new(),
                running: false,
            })),
            bindings: Arc::new(Mutex::new(bindings)),
            pending: Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())),
            last_context_tokens: Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())),
            sink,
            creds,
            reply: Arc::new(reply),
        }
    }

    /// Access the binding store (for the host's pairing/allowlist UI actions).
    pub fn bindings(&self) -> Arc<Mutex<BindingStore>> {
        Arc::clone(&self.bindings)
    }

    /// Fetch a fresh login QR code and report `WaitingScan`. Returns the opaque
    /// QR handle to poll with [`poll_login_once`].
    pub async fn begin_login(&self) -> Result<String> {
        let qr = ILinkClient::fetch_qr_code(&self.http, DEFAULT_BASE_URL).await?;
        self.sink.on_status(BridgeStatus::WaitingScan {
            scan_url: qr.qrcode_img_content.clone(),
        });
        Ok(qr.qrcode)
    }

    /// Poll a login QR once. On confirmation, saves credentials and starts the
    /// poll loop. Returns true when login is resolved (confirmed or expired).
    pub async fn poll_login_once(&self, qrcode: &str) -> Result<bool> {
        match ILinkClient::poll_qr_status(&self.http, DEFAULT_BASE_URL, qrcode).await? {
            QrScanOutcome::Waiting => Ok(false),
            QrScanOutcome::Scanned => {
                self.sink.on_status(BridgeStatus::Scanned);
                Ok(false)
            }
            QrScanOutcome::Expired => {
                self.sink.on_status(BridgeStatus::Error {
                    message: "qr_expired".into(),
                });
                Ok(true)
            }
            QrScanOutcome::Confirmed(creds) => {
                self.creds.save_credentials(&creds);
                self.sink.on_status(BridgeStatus::Connecting);
                self.start_polling(creds).await;
                Ok(true)
            }
        }
    }

    /// Start the poll loop from already-saved credentials.
    pub async fn start_from_saved(&self) -> Result<()> {
        let Some(creds) = self.creds.load_credentials() else {
            return Err(crate::Error::Protocol("no saved credentials".into()));
        };
        self.start_polling(creds).await;
        Ok(())
    }

    /// Stop the poll loop; credentials are preserved.
    pub async fn stop(&self) {
        let mut inner = self.inner.lock().await;
        inner.running = false;
        inner.client = None;
        self.sink.on_status(BridgeStatus::Disconnected);
    }

    /// Stop and forget credentials + sync cursor.
    pub async fn logout(&self) {
        self.stop().await;
        self.creds.clear_credentials();
        self.creds.save_sync_cursor("");
        let mut inner = self.inner.lock().await;
        inner.sync_cursor.clear();
    }

    async fn start_polling(&self, creds: WeChatCredentials) {
        {
            let mut inner = self.inner.lock().await;
            inner.client = Some(ILinkClient::new(&creds));
            inner.sync_cursor = self.creds.load_sync_cursor();
            inner.running = true;
        }
        self.sink.on_status(BridgeStatus::Connected);

        self.poll_loop().await;
    }

    async fn poll_loop(&self) {
        let mut failures: u32 = 0;
        loop {
            // Snapshot the client + cursor without holding the lock across await.
            let (client, cursor, running) = {
                let inner = self.inner.lock().await;
                (
                    inner.client.clone(),
                    inner.sync_cursor.clone(),
                    inner.running,
                )
            };
            let Some(client) = client else {
                return;
            };
            if !running {
                return;
            }

            self.sink.on_poll_start(cursor.len());
            let updates = client.get_updates(&cursor).await;

            match updates {
                Ok(resp) => {
                    failures = 0;

                    if resp.errcode == Some(SESSION_EXPIRED_CODE) {
                        if !cursor.is_empty() {
                            self.set_cursor(String::new()).await;
                            tokio::time::sleep(Duration::from_secs(5)).await;
                            continue;
                        } else {
                            self.sink.on_status(BridgeStatus::Error {
                                message: "session_expired".into(),
                            });
                            return;
                        }
                    }

                    if resp.ret != 0 && resp.errcode.is_some() {
                        tracing::warn!(
                            ret = resp.ret,
                            errcode = ?resp.errcode,
                            "wechat getupdates server error"
                        );
                        continue;
                    }

                    if !resp.get_updates_buf.is_empty() {
                        self.set_cursor(resp.get_updates_buf.clone()).await;
                    }

                    self.sink
                        .on_poll_result(resp.msgs.len(), resp.ret, resp.errcode);
                    for msg in &resp.msgs {
                        self.handle_message(msg).await;
                    }
                }
                Err(e) => {
                    // Was the loop stopped while awaiting?
                    if !self.inner.lock().await.running {
                        return;
                    }
                    failures += 1;
                    self.sink.on_poll_error(&e.to_string(), failures);
                    let backoff = backoff_delay(failures);
                    tracing::warn!(
                        failures,
                        backoff_ms = backoff.as_millis() as u64,
                        error = %e,
                        "wechat poll failed"
                    );
                    if failures >= MAX_CONSECUTIVE_FAILURES {
                        tracing::warn!("wechat poll: too many consecutive failures");
                    }
                    tokio::time::sleep(backoff).await;
                }
            }
        }
    }

    async fn handle_message(&self, msg: &crate::wechat::IncomingMessage) {
        let text = message_text(msg);
        self.sink.on_incoming_message(
            &msg.from_user_id,
            msg.message_type,
            msg.message_state,
            text.chars().count(),
        );
        if msg.message_type == message_type::BOT {
            return;
        }
        if msg.message_type != message_type::USER && msg.message_type != 0 {
            return;
        }
        if msg.message_state != message_state::FINISH && msg.message_state != 0 {
            return;
        }
        if text.trim().is_empty() {
            return;
        }
        let chat_id = msg.from_user_id.clone();
        if chat_id.trim().is_empty() {
            return;
        }
        let context_token = msg.context_token.clone();
        self.last_context_tokens
            .lock()
            .unwrap()
            .insert(chat_id.clone(), context_token.clone());

        let decision = {
            let store = self.bindings.lock().await;
            decide_access(&store, &chat_id)
        };

        match decision {
            AccessDecision::Bound => {
                let session_id = {
                    let store = self.bindings.lock().await;
                    store.binding(&chat_id).map(|b| b.session_id.clone())
                };
                if let Some(session_id) = session_id {
                    self.sink
                        .on_bound_message_start(&chat_id, &session_id, &text);
                    let ok = self.sink.write_to_session(&session_id, &text);
                    if !ok {
                        // Session gone: drop the stale binding.
                        let mut store = self.bindings.lock().await;
                        store.unbind(&chat_id);
                        drop(store);
                        self.reply(&chat_id, &self.reply.needs_binding, &context_token)
                            .await;
                    } else {
                        self.sink.on_bound_message(&chat_id, &session_id);
                    }
                }
            }
            AccessDecision::NeedsBinding => {
                self.reply(&chat_id, &self.reply.needs_binding, &context_token)
                    .await;
            }
            AccessDecision::NeedsPairing => {
                let code = pairing_code(&chat_id);
                self.pending.lock().unwrap().insert(
                    chat_id.clone(),
                    PendingPairing {
                        chat_id: chat_id.clone(),
                        code: code.clone(),
                        context_token: context_token.clone(),
                    },
                );
                self.sink.on_pairing_request(&chat_id, &code);
                let prompt = self.reply.pairing_prompt.replace("{code}", &code);
                self.reply(&chat_id, &prompt, &context_token).await;
            }
            AccessDecision::Rejected => {
                self.reply(&chat_id, &self.reply.rejected, &context_token)
                    .await;
            }
        }
    }

    /// Peers currently awaiting desktop approval.
    pub fn pending_pairings(&self) -> Vec<PendingPairing> {
        self.pending.lock().unwrap().values().cloned().collect()
    }

    /// Drop a pending pairing without approving it.
    pub fn dismiss_pairing(&self, chat_id: &str) {
        self.pending.lock().unwrap().remove(chat_id);
    }

    /// Confirm a pairing code and bind the peer to a session. Called by the
    /// host after the user approves the code shown by the chat prompt. Sends
    /// the confirmation reply to the peer and clears the pending entry.
    pub async fn confirm_pairing(
        &self,
        chat_id: &str,
        code: &str,
        session_id: &str,
        workspace_id: Option<String>,
    ) -> bool {
        if pairing_code(chat_id) != code {
            return false;
        }
        {
            let mut store = self.bindings.lock().await;
            store.bind(ChatBinding {
                chat_id: chat_id.to_string(),
                session_id: session_id.to_string(),
                workspace_id,
                created_at: now_millis(),
            });
        }
        let pending = self.pending.lock().unwrap().remove(chat_id);
        if let Some(pending) = pending {
            let confirmed = self.reply.pairing_confirmed.clone();
            self.reply(chat_id, &confirmed, &pending.context_token)
                .await;
        }
        self.sink.on_pairing_confirmed(chat_id, session_id);
        true
    }

    async fn reply(&self, chat_id: &str, text: &str, context_token: &str) {
        let inner = self.inner.lock().await;
        let Some(client) = inner.client.as_ref() else {
            return;
        };
        for chunk in chunk_text(text, MAX_TEXT_LEN) {
            if let Err(e) = client.send_text(chat_id, &chunk, context_token).await {
                tracing::warn!(error = %e, "wechat send_text failed");
                break;
            }
        }
    }

    /// Send a reply out-of-band (e.g. streamed AI output the host produced).
    pub async fn send_reply(&self, chat_id: &str, text: &str, context_token: &str) {
        self.reply(chat_id, text, context_token).await;
    }

    /// Send a reply using the latest message context token for this chat peer.
    /// Returns false when the bridge has not seen a message from the peer in
    /// the current process, because WeChat replies require a context token.
    pub async fn send_reply_to_chat(&self, chat_id: &str, text: &str) -> bool {
        let context_token = self
            .last_context_tokens
            .lock()
            .unwrap()
            .get(chat_id)
            .cloned();
        let Some(context_token) = context_token else {
            return false;
        };
        self.reply(chat_id, text, &context_token).await;
        true
    }

    async fn set_cursor(&self, cursor: String) {
        {
            let mut inner = self.inner.lock().await;
            inner.sync_cursor = cursor.clone();
        }
        self.creds.save_sync_cursor(&cursor);
    }
}

fn backoff_delay(failures: u32) -> Duration {
    let exp = failures.saturating_sub(1).min(20);
    let millis = INITIAL_BACKOFF.as_millis().saturating_mul(1u128 << exp);
    let capped = millis.min(MAX_BACKOFF.as_millis());
    Duration::from_millis(capped as u64)
}

/// Split `text` into chunks no longer than `max` chars (by char boundary).
pub fn chunk_text(text: &str, max: usize) -> Vec<String> {
    if text.chars().count() <= max {
        return vec![text.to_string()];
    }
    let mut chunks = Vec::new();
    let mut current = String::new();
    for ch in text.chars() {
        if current.chars().count() >= max {
            chunks.push(std::mem::take(&mut current));
        }
        current.push(ch);
    }
    if !current.is_empty() {
        chunks.push(current);
    }
    chunks
}

/// A stable, non-secret 6-digit pairing code derived from the chat id. It only
/// gates first contact; the allowlist is the real access control afterward.
pub fn pairing_code(chat_id: &str) -> String {
    // FNV-1a over the id, folded to 6 digits. Deterministic per peer so the
    // code shown in the UI matches what the user is told to confirm.
    let mut hash: u64 = 0xcbf29ce484222325;
    for b in chat_id.as_bytes() {
        hash ^= *b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{:06}", hash % 1_000_000)
}

fn now_millis() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunk_text_short_stays_single() {
        assert_eq!(chunk_text("hello", 4000), vec!["hello".to_string()]);
    }

    #[test]
    fn chunk_text_splits_on_limit() {
        let text = "a".repeat(9000);
        let chunks = chunk_text(&text, 4000);
        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0].chars().count(), 4000);
        assert_eq!(chunks[2].chars().count(), 1000);
    }

    #[test]
    fn chunk_text_respects_char_boundaries() {
        let text = "中".repeat(5000);
        let chunks = chunk_text(&text, 4000);
        assert_eq!(chunks.len(), 2);
        // Rejoining must reproduce the original exactly.
        assert_eq!(chunks.concat(), text);
    }

    #[test]
    fn pairing_code_is_stable_and_six_digits() {
        let a = pairing_code("peer-abc");
        let b = pairing_code("peer-abc");
        assert_eq!(a, b);
        assert_eq!(a.len(), 6);
        assert!(a.chars().all(|c| c.is_ascii_digit()));
    }

    #[test]
    fn pairing_code_differs_between_peers() {
        assert_ne!(pairing_code("peer-1"), pairing_code("peer-2"));
    }

    #[test]
    fn backoff_grows_and_caps() {
        assert_eq!(backoff_delay(1), INITIAL_BACKOFF);
        assert_eq!(backoff_delay(2), Duration::from_secs(6));
        assert!(backoff_delay(20) <= MAX_BACKOFF);
        assert_eq!(backoff_delay(20), MAX_BACKOFF);
    }
}
