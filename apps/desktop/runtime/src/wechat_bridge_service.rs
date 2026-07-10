//! Desktop lifecycle wrapper around the WeChat chat bridge
//! (`codux-im-bridge`): owns the singleton bridge instance, persists
//! credentials/cursor/bindings under the app support dir, and exposes a
//! poll-friendly status snapshot for the settings UI.
//!
//! The bridge routes an authorized WeChat peer's text into a bound terminal
//! session (`TerminalManager::write`), which is where the AI CLI runs. Pairing
//! follows the allowlist-first model: a new peer receives a pairing code and a
//! desktop user must confirm it against a chosen terminal session.

use std::collections::HashMap;
use std::io::{Read, Seek, SeekFrom};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use codux_im_bridge::binding::{BindingState, BindingStore, ChatBinding};
use codux_im_bridge::runtime::{BridgeStatus, CredentialStore, HostSink, ReplyText, WeChatBridge};
use codux_im_bridge::wechat::WeChatCredentials;
use codux_runtime_live::terminal_pty::TerminalManager;
use parking_lot::Mutex;

use crate::async_runtime;
use crate::runtime_paths;

static STATUS: OnceLock<Mutex<WeChatBridgeSnapshot>> = OnceLock::new();
static BRIDGE: OnceLock<Mutex<Option<WeChatBridge>>> = OnceLock::new();
static TERMINALS: OnceLock<Mutex<Option<Arc<TerminalManager>>>> = OnceLock::new();
static OUTPUT_RELAYS: OnceLock<Mutex<HashMap<String, OutputRelayState>>> = OnceLock::new();
/// Increments on every stop/login so stale login-poll loops exit.
static GENERATION: AtomicU64 = AtomicU64::new(0);

/// UI-facing snapshot of the bridge state. `scan_url` is set while waiting for
/// a scan so the settings pane can render the QR locally.
#[derive(Debug, Clone, Default)]
pub struct WeChatBridgeSnapshot {
    pub state: WeChatBridgeState,
    pub scan_url: Option<String>,
    pub error: Option<String>,
    /// True when credentials exist on disk (a session can be resumed).
    pub has_credentials: bool,
    /// Number of WeChat peers bound to terminal sessions.
    pub binding_count: usize,
    /// Number of peers authorized by the allowlist.
    pub allowlist_count: usize,
    /// A WeChat peer awaiting pairing confirmation: `(chat_id, code)`.
    pub pending_pairing: Option<(String, String)>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum WeChatBridgeState {
    #[default]
    Disconnected,
    WaitingScan,
    Scanned,
    Connecting,
    Connected,
    Error,
}

fn status_cell() -> &'static Mutex<WeChatBridgeSnapshot> {
    STATUS.get_or_init(|| {
        Mutex::new(WeChatBridgeSnapshot {
            has_credentials: FileCredentialStore::default_paths()
                .credentials_path
                .exists(),
            ..Default::default()
        })
    })
}

fn bridge_cell() -> &'static Mutex<Option<WeChatBridge>> {
    BRIDGE.get_or_init(|| Mutex::new(None))
}

fn terminals_cell() -> &'static Mutex<Option<Arc<TerminalManager>>> {
    TERMINALS.get_or_init(|| Mutex::new(None))
}

fn output_relays_cell() -> &'static Mutex<HashMap<String, OutputRelayState>> {
    OUTPUT_RELAYS.get_or_init(|| Mutex::new(HashMap::new()))
}

#[derive(Clone, Debug)]
struct OutputRelayState {
    session_id: String,
    turn: u64,
    replied_turn: Option<u64>,
    incoming_text: String,
    approval_fingerprint: Option<String>,
}

impl OutputRelayState {
    fn new(session_id: String, incoming_text: Option<&str>) -> Self {
        let begins_turn = incoming_text.is_some();
        Self {
            session_id,
            turn: u64::from(begins_turn),
            replied_turn: None,
            incoming_text: incoming_text.unwrap_or_default().to_string(),
            approval_fingerprint: None,
        }
    }

    fn begin_turn(&mut self, incoming_text: &str) {
        self.turn = self.turn.saturating_add(1);
        self.replied_turn = None;
        self.incoming_text = incoming_text.to_string();
        self.approval_fingerprint = None;
    }

    fn has_replied(&self, turn: u64) -> bool {
        self.replied_turn == Some(turn)
    }
}

fn masked_chat_id(chat_id: &str) -> String {
    let count = chat_id.chars().count();
    if count <= 8 {
        return "*".repeat(count.max(1));
    }
    let head: String = chat_id.chars().take(4).collect();
    let tail: String = chat_id
        .chars()
        .rev()
        .take(4)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    format!("{head}…{tail}")
}

/// Read the current snapshot (cheap; call from render).
pub fn wechat_bridge_snapshot() -> WeChatBridgeSnapshot {
    let mut snapshot = status_cell().lock().clone();
    let paths = FileCredentialStore::default_paths();
    if let Ok(text) = std::fs::read_to_string(paths.bindings_path)
        && let Ok(state) = serde_json::from_str::<BindingState>(&text)
    {
        snapshot.binding_count = state.bindings.len();
        snapshot.allowlist_count = state.allowlist.len();
    }
    snapshot
}

fn set_status(status: BridgeStatus) {
    let mut cell = status_cell().lock();
    match status {
        BridgeStatus::Disconnected => {
            crate::runtime_trace::runtime_trace("wechat", "status disconnected");
            cell.state = WeChatBridgeState::Disconnected;
            cell.scan_url = None;
            cell.error = None;
        }
        BridgeStatus::WaitingScan { scan_url } => {
            crate::runtime_trace::runtime_trace("wechat", "status waiting_scan");
            cell.state = WeChatBridgeState::WaitingScan;
            cell.scan_url = Some(scan_url);
            cell.error = None;
        }
        BridgeStatus::Scanned => {
            crate::runtime_trace::runtime_trace("wechat", "status scanned");
            cell.state = WeChatBridgeState::Scanned;
        }
        BridgeStatus::Connecting => {
            crate::runtime_trace::runtime_trace("wechat", "status connecting");
            cell.state = WeChatBridgeState::Connecting;
            cell.scan_url = None;
        }
        BridgeStatus::Connected => {
            crate::runtime_trace::runtime_trace("wechat", "status connected");
            cell.state = WeChatBridgeState::Connected;
            cell.scan_url = None;
            cell.error = None;
            cell.has_credentials = true;
        }
        BridgeStatus::Error { message } => {
            crate::runtime_trace::runtime_trace("wechat", &format!("status error {message}"));
            cell.state = WeChatBridgeState::Error;
            cell.scan_url = None;
            cell.error = Some(message);
        }
    }
}

// ===== storage =====

struct StorePaths {
    credentials_path: PathBuf,
    sync_path: PathBuf,
    bindings_path: PathBuf,
}

struct FileCredentialStore {
    paths: StorePaths,
}

impl FileCredentialStore {
    fn default_paths() -> StorePaths {
        let dir = runtime_paths::app_support_dir().join("wechat-bridge");
        StorePaths {
            credentials_path: dir.join("credentials.json"),
            sync_path: dir.join("sync.json"),
            bindings_path: dir.join("bindings.json"),
        }
    }

    fn new() -> Self {
        Self {
            paths: Self::default_paths(),
        }
    }
}

impl CredentialStore for FileCredentialStore {
    fn load_credentials(&self) -> Option<WeChatCredentials> {
        let text = std::fs::read_to_string(&self.paths.credentials_path).ok()?;
        serde_json::from_str(&text).ok()
    }

    fn save_credentials(&self, creds: &WeChatCredentials) {
        codux_im_bridge::binding::write_json_600(&self.paths.credentials_path, creds);
        status_cell().lock().has_credentials = true;
    }

    fn clear_credentials(&self) {
        let _ = std::fs::remove_file(&self.paths.credentials_path);
        status_cell().lock().has_credentials = false;
    }

    fn load_sync_cursor(&self) -> String {
        std::fs::read_to_string(&self.paths.sync_path)
            .ok()
            .and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok())
            .and_then(|v| v.get("cursor").and_then(|c| c.as_str()).map(String::from))
            .unwrap_or_default()
    }

    fn save_sync_cursor(&self, cursor: &str) {
        crate::runtime_trace::runtime_trace(
            "wechat",
            &format!("cursor_saved len={}", cursor.len()),
        );
        codux_im_bridge::binding::write_json_600(
            &self.paths.sync_path,
            &serde_json::json!({ "cursor": cursor }),
        );
    }
}

// ===== sink =====

struct TerminalSink;

impl HostSink for TerminalSink {
    fn write_to_session(&self, session_id: &str, text: &str) -> bool {
        let Some(terminals) = terminals_cell().lock().clone() else {
            return false;
        };
        // Submit as one line: text then CR, the same bytes the terminal sends
        // when the user presses Enter.
        let mut bytes = text.as_bytes().to_vec();
        bytes.push(b'\r');
        terminals.write(session_id, &bytes).is_ok()
    }

    fn on_status(&self, status: BridgeStatus) {
        set_status(status);
    }

    fn on_pairing_request(&self, chat_id: &str, code: &str) {
        crate::runtime_trace::runtime_trace(
            "wechat",
            &format!(
                "pairing_request chat={} code={code}",
                masked_chat_id(chat_id)
            ),
        );
        status_cell().lock().pending_pairing = Some((chat_id.to_string(), code.to_string()));
    }

    fn on_poll_result(&self, message_count: usize, ret: i32, errcode: Option<i32>) {
        crate::runtime_trace::runtime_trace(
            "wechat",
            &format!(
                "poll_result messages={message_count} ret={ret} errcode={}",
                errcode
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "none".to_string())
            ),
        );
    }

    fn on_poll_start(&self, cursor_len: usize) {
        crate::runtime_trace::runtime_trace(
            "wechat",
            &format!("poll_start cursor_len={cursor_len}"),
        );
    }

    fn on_poll_error(&self, error: &str, failures: u32) {
        crate::runtime_trace::runtime_trace(
            "wechat",
            &format!("poll_error failures={failures} error={error}"),
        );
    }

    fn on_incoming_message(
        &self,
        chat_id: &str,
        message_type: i32,
        message_state: i32,
        text_len: usize,
    ) {
        crate::runtime_trace::runtime_trace(
            "wechat",
            &format!(
                "incoming_message chat={} type={message_type} state={message_state} text_len={text_len}",
                masked_chat_id(chat_id)
            ),
        );
    }

    fn on_bound_message(&self, chat_id: &str, session_id: &str) {
        crate::runtime_trace::runtime_trace(
            "wechat",
            &format!(
                "bound_message chat={} session={session_id}",
                masked_chat_id(chat_id)
            ),
        );
    }

    fn on_bound_message_start(&self, chat_id: &str, session_id: &str, text: &str) {
        start_output_relay(chat_id.to_string(), session_id.to_string(), Some(text));
    }

    fn on_pairing_confirmed(&self, chat_id: &str, session_id: &str) {
        crate::runtime_trace::runtime_trace(
            "wechat",
            &format!(
                "pairing_confirmed chat={} session={session_id}",
                masked_chat_id(chat_id)
            ),
        );
        start_output_relay(chat_id.to_string(), session_id.to_string(), None);
    }
}

// ===== lifecycle =====

fn reply_text() -> ReplyText {
    ReplyText {
        pairing_prompt: "Codux 配对码 / pairing code: {code}\n请在 Codux 桌面端「远程 → 微信」中确认。 / Confirm it in Codux desktop under Remote → WeChat.".to_string(),
        needs_binding: "该微信已授权，但还没有绑定终端会话。请在 Codux 桌面端选择一个会话完成绑定。 / Authorized but not bound to a terminal session yet. Pick one in Codux desktop.".to_string(),
        rejected: "未授权访问。 / Not authorized.".to_string(),
        pairing_confirmed: "✅ 绑定成功。现在发送的消息会输入到绑定的 Codux 终端。 / Paired. Your messages now go to the bound Codux terminal.".to_string(),
    }
}

fn ensure_bridge() -> WeChatBridge {
    let mut cell = bridge_cell().lock();
    if let Some(bridge) = cell.as_ref() {
        return bridge.clone();
    }
    let paths = FileCredentialStore::default_paths();
    let bridge = WeChatBridge::new(
        BindingStore::load(paths.bindings_path),
        Arc::new(TerminalSink),
        Arc::new(FileCredentialStore::new()),
        reply_text(),
    );
    *cell = Some(bridge.clone());
    bridge
}

/// Provide the terminal manager the sink writes into. Call once at app start
/// (idempotent; the latest manager wins).
pub fn set_wechat_bridge_terminals(terminals: Arc<TerminalManager>) {
    let next_count = terminals.list().len();
    let mut cell = terminals_cell().lock();
    let current_count = cell
        .as_ref()
        .map(|manager| manager.list().len())
        .unwrap_or(0);
    if current_count > 0 && next_count == 0 {
        crate::runtime_trace::runtime_trace(
            "wechat",
            &format!("terminals_update skipped current={current_count} next=0"),
        );
        return;
    }
    crate::runtime_trace::runtime_trace(
        "wechat",
        &format!("terminals_update current={current_count} next={next_count}"),
    );
    *cell = Some(terminals);
}

pub fn wechat_bridge_fallback_terminal_session_id() -> Option<String> {
    terminals_cell().lock().as_ref().and_then(|manager| {
        manager
            .list()
            .into_iter()
            .filter(|session| session.is_running)
            .max_by(|left, right| left.last_active_at.cmp(&right.last_active_at))
            .or_else(|| {
                manager
                    .list()
                    .into_iter()
                    .max_by(|left, right| left.last_active_at.cmp(&right.last_active_at))
            })
            .map(|session| session.id)
    })
}

pub fn wechat_bridge_bound_session_ids() -> Vec<String> {
    let paths = FileCredentialStore::default_paths();
    std::fs::read_to_string(paths.bindings_path)
        .ok()
        .and_then(|text| serde_json::from_str::<BindingState>(&text).ok())
        .map(|state| {
            state
                .bindings
                .values()
                .map(|binding| binding.session_id.clone())
                .collect()
        })
        .unwrap_or_default()
}

pub fn wechat_bridge_bind_existing_to_session(session_id: &str) -> bool {
    let session_id = session_id.trim();
    if session_id.is_empty() {
        crate::runtime_trace::runtime_trace("wechat", "bind_existing skipped reason=empty_session");
        return false;
    }
    if status_cell().lock().pending_pairing.is_some() {
        return wechat_bridge_confirm_pairing(session_id);
    }

    let paths = FileCredentialStore::default_paths();
    let Some(chat_id) = std::fs::read_to_string(paths.bindings_path)
        .ok()
        .and_then(|text| serde_json::from_str::<BindingState>(&text).ok())
        .and_then(|state| {
            state
                .bindings
                .keys()
                .next()
                .cloned()
                .or_else(|| state.allowlist.first().cloned())
        })
    else {
        crate::runtime_trace::runtime_trace(
            "wechat",
            &format!("bind_existing skipped reason=no_chat session={session_id}"),
        );
        return false;
    };

    let bridge = ensure_bridge();
    let session_id = session_id.to_string();
    crate::runtime_trace::runtime_trace(
        "wechat",
        &format!(
            "bind_existing queued chat={} session={session_id}",
            masked_chat_id(&chat_id)
        ),
    );
    async_runtime::block_on(async {
        let bindings = bridge.bindings();
        let mut store = bindings.lock().await;
        store.bind(ChatBinding {
            chat_id: chat_id.clone(),
            session_id: session_id.clone(),
            workspace_id: None,
            created_at: current_millis(),
        });
    });
    start_output_relay(chat_id.clone(), session_id.clone(), None);
    crate::runtime_trace::runtime_trace(
        "wechat",
        &format!(
            "bind_existing ok chat={} session={session_id}",
            masked_chat_id(&chat_id)
        ),
    );
    true
}

fn current_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or_default()
}

fn start_output_relay(chat_id: String, session_id: String, incoming_text: Option<&str>) {
    let should_spawn = {
        let mut relays = output_relays_cell().lock();
        if let Some(relay) = relays.get_mut(&chat_id)
            && relay.session_id == session_id
        {
            if let Some(incoming_text) = incoming_text {
                relay.begin_turn(incoming_text);
            }
            false
        } else {
            relays.insert(
                chat_id.clone(),
                OutputRelayState::new(session_id.clone(), incoming_text),
            );
            true
        }
    };
    if !should_spawn {
        return;
    }

    let Some(terminals) = terminals_cell().lock().clone() else {
        output_relays_cell().lock().remove(&chat_id);
        return;
    };
    let rx = match terminals.subscribe_output(&session_id, false) {
        Ok(rx) => rx,
        Err(_) => {
            output_relays_cell().lock().remove(&chat_id);
            return;
        }
    };
    let bridge = ensure_bridge();
    async_runtime::spawn(async move {
        relay_terminal_output_to_wechat(terminals, bridge, chat_id, session_id, rx).await;
    });
}

async fn relay_terminal_output_to_wechat(
    terminals: Arc<TerminalManager>,
    bridge: WeChatBridge,
    chat_id: String,
    session_id: String,
    rx: flume::Receiver<Vec<u8>>,
) {
    let mut last_completion = terminal_completion_marker(&terminals, &session_id);
    let mut observed_turn = output_relay_turn(&chat_id, &session_id).unwrap_or_default();
    let mut output_active = false;
    let mut raw_output = Vec::new();
    let mut last_output_at = None;
    let mut interval = tokio::time::interval(Duration::from_millis(250));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    loop {
        if !output_relay_is_current(&chat_id, &session_id) {
            return;
        }
        let Some(current_turn) = output_relay_turn(&chat_id, &session_id) else {
            return;
        };
        if current_turn != observed_turn {
            observed_turn = current_turn;
            last_completion = terminal_completion_marker(&terminals, &session_id);
            output_active = false;
            raw_output.clear();
            last_output_at = None;
        }
        tokio::select! {
            chunk = rx.recv_async() => {
                let Ok(chunk) = chunk else {
                    clear_output_relay(&chat_id, &session_id);
                    return;
                };
                let Some(latest_turn) = output_relay_turn(&chat_id, &session_id) else {
                    return;
                };
                if latest_turn != observed_turn {
                    observed_turn = latest_turn;
                    raw_output.clear();
                }
                append_bounded(&mut raw_output, &chunk, 64 * 1024);
                output_active = true;
                last_output_at = Some(Instant::now());
            }
            _ = interval.tick() => {
                if !output_active {
                    continue;
                }
                let _ = terminals.poll_ai_runtime_state();

                if let Some((fingerprint, prompt)) = pending_terminal_approval(&terminals, &session_id)
                    && !output_relay_approval_was_sent(
                        &chat_id,
                        &session_id,
                        observed_turn,
                        &fingerprint,
                    )
                    && bridge.send_reply_to_chat(&chat_id, &prompt).await
                {
                    mark_output_relay_approval_sent(
                        &chat_id,
                        &session_id,
                        observed_turn,
                        fingerprint,
                    );
                    crate::runtime_trace::runtime_trace(
                        "wechat",
                        &format!(
                            "approval_prompt sent chat={} session={session_id} chars={}",
                            masked_chat_id(&chat_id),
                            prompt.chars().count()
                        ),
                    );
                }

                if let Some((marker, reply)) = completed_terminal_reply(&terminals, &session_id)
                    && last_completion.as_ref() != Some(&marker)
                    && !output_relay_has_replied(&chat_id, &session_id, observed_turn)
                    && bridge.send_reply_to_chat(&chat_id, &reply).await
                {
                    mark_output_relay_replied(&chat_id, &session_id, observed_turn);
                    crate::runtime_trace::runtime_trace(
                        "wechat",
                        &format!(
                            "assistant_reply sent chat={} session={session_id} chars={}",
                            masked_chat_id(&chat_id),
                            reply.chars().count()
                        ),
                    );
                    last_completion = Some(marker);
                    output_active = false;
                    continue;
                }

                if output_relay_has_replied(&chat_id, &session_id, observed_turn) {
                    output_active = false;
                    continue;
                }
                if !last_output_at.is_some_and(|at| at.elapsed() >= Duration::from_millis(900)) {
                    continue;
                }
                let incoming_text = output_relay_incoming_text(
                    &chat_id,
                    &session_id,
                    observed_turn,
                ).unwrap_or_default();
                let Some(reply) = shell_terminal_reply(
                    &terminals,
                    &session_id,
                    &raw_output,
                    &incoming_text,
                ) else {
                    continue;
                };
                if bridge.send_reply_to_chat(&chat_id, &reply).await {
                    mark_output_relay_replied(&chat_id, &session_id, observed_turn);
                    crate::runtime_trace::runtime_trace(
                        "wechat",
                        &format!(
                            "shell_reply sent chat={} session={session_id} chars={}",
                            masked_chat_id(&chat_id),
                            reply.chars().count()
                        ),
                    );
                    output_active = false;
                }
            }
        }
    }
}

fn output_relay_is_current(chat_id: &str, session_id: &str) -> bool {
    output_relays_cell()
        .lock()
        .get(chat_id)
        .map(|relay| relay.session_id.as_str())
        == Some(session_id)
}

fn output_relay_turn(chat_id: &str, session_id: &str) -> Option<u64> {
    output_relays_cell()
        .lock()
        .get(chat_id)
        .filter(|relay| relay.session_id == session_id)
        .map(|relay| relay.turn)
}

fn output_relay_has_replied(chat_id: &str, session_id: &str, turn: u64) -> bool {
    output_relays_cell()
        .lock()
        .get(chat_id)
        .filter(|relay| relay.session_id == session_id)
        .is_some_and(|relay| relay.has_replied(turn))
}

fn output_relay_incoming_text(chat_id: &str, session_id: &str, turn: u64) -> Option<String> {
    output_relays_cell()
        .lock()
        .get(chat_id)
        .filter(|relay| relay.session_id == session_id && relay.turn == turn)
        .map(|relay| relay.incoming_text.clone())
}

fn output_relay_approval_was_sent(
    chat_id: &str,
    session_id: &str,
    turn: u64,
    fingerprint: &str,
) -> bool {
    output_relays_cell()
        .lock()
        .get(chat_id)
        .filter(|relay| relay.session_id == session_id && relay.turn == turn)
        .and_then(|relay| relay.approval_fingerprint.as_deref())
        == Some(fingerprint)
}

fn mark_output_relay_approval_sent(
    chat_id: &str,
    session_id: &str,
    turn: u64,
    fingerprint: String,
) {
    let mut relays = output_relays_cell().lock();
    if let Some(relay) = relays.get_mut(chat_id)
        && relay.session_id == session_id
        && relay.turn == turn
    {
        relay.approval_fingerprint = Some(fingerprint);
    }
}

fn mark_output_relay_replied(chat_id: &str, session_id: &str, turn: u64) {
    let mut relays = output_relays_cell().lock();
    if let Some(relay) = relays.get_mut(chat_id)
        && relay.session_id == session_id
        && relay.turn == turn
    {
        relay.replied_turn = Some(turn);
    }
}

fn clear_output_relay(chat_id: &str, session_id: &str) {
    let mut relays = output_relays_cell().lock();
    if relays.get(chat_id).map(|relay| relay.session_id.as_str()) == Some(session_id) {
        relays.remove(chat_id);
    }
}

fn append_bounded(target: &mut Vec<u8>, chunk: &[u8], max_bytes: usize) {
    if chunk.len() >= max_bytes {
        target.clear();
        target.extend_from_slice(&chunk[chunk.len() - max_bytes..]);
        return;
    }
    let overflow = target
        .len()
        .saturating_add(chunk.len())
        .saturating_sub(max_bytes);
    if overflow > 0 {
        target.drain(..overflow.min(target.len()));
    }
    target.extend_from_slice(chunk);
}

fn pending_terminal_approval(
    terminals: &TerminalManager,
    session_id: &str,
) -> Option<(String, String)> {
    let state = terminals.ai_runtime_state_snapshot()?;
    let session = state
        .sessions
        .iter()
        .find(|session| session.terminal_id == session_id && session.state == "needsInput")?;
    let snapshot = terminals.screen_snapshot(session_id).ok()?;
    let screen = codux_runtime_live::ai_runtime::screen_signal::screen_text_from_cells(&snapshot);
    let prompt = approval_prompt_from_screen(&screen)?;
    let fingerprint = format!(
        "{}:{}",
        session.ai_session_id.as_deref().unwrap_or("unknown"),
        prompt
    );
    Some((fingerprint, prompt))
}

fn approval_prompt_from_screen(screen: &str) -> Option<String> {
    let mut lines = screen
        .lines()
        .map(clean_approval_line)
        .filter(|line| !line.trim().is_empty())
        .filter(|line| !is_terminal_approval_footer(line))
        .collect::<Vec<_>>();
    let tail_start = lines.len().saturating_sub(16);
    lines.drain(..tail_start);

    let first_option = lines
        .iter()
        .position(|line| approval_option_number(line).is_some());
    if let Some(first_option) = first_option {
        let prompt_start = lines[..first_option]
            .iter()
            .position(|line| is_approval_heading(line))
            .unwrap_or_else(|| first_option.saturating_sub(6));
        lines.drain(..prompt_start);
    } else if let Some(prompt_start) = lines.iter().rposition(|line| is_approval_heading(line)) {
        lines.drain(..prompt_start);
    }
    let title = lines.first()?.trim().to_string();
    let mut description = Vec::new();
    let mut options: Vec<(usize, String)> = Vec::new();
    for line in lines.into_iter().skip(1) {
        if let Some(number) = approval_option_number(&line) {
            options.push((number, line));
        } else if let Some((_, option)) = options.last_mut() {
            option.push(' ');
            option.push_str(line.trim());
        } else {
            description.push(line);
        }
    }

    let mut sections = vec!["【需要确认】".to_string(), title];
    sections.extend(description);
    sections.extend(options.iter().map(|(_, option)| option.clone()));
    sections.push(approval_reply_hint(&options));
    let mut prompt = sections.join("\n\n");
    if prompt.chars().count() > 2_000 {
        prompt = prompt
            .chars()
            .rev()
            .take(2_000)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
    }
    (!prompt.trim().is_empty()).then_some(prompt.trim().to_string())
}

fn clean_approval_line(line: &str) -> String {
    let line = line.trim();
    for prefix in ["❯", "›", ">"] {
        if let Some(rest) = line.strip_prefix(prefix)
            && approval_option_number(rest.trim_start()).is_some()
        {
            return rest.trim_start().to_string();
        }
    }
    line.to_string()
}

fn is_terminal_approval_footer(line: &str) -> bool {
    let line = line.to_lowercase();
    [
        "press enter to confirm",
        "esc to go back",
        "esc to cancel",
        "tab to amend",
        "ctrl+e to explain",
    ]
    .iter()
    .any(|marker| line.contains(marker))
}

fn is_approval_heading(line: &str) -> bool {
    let line = line.to_lowercase();
    [
        "hooks need review",
        "bash command",
        "permission required",
        "allow execution",
        "allow command",
        "apply this change",
        "do you want",
        "would you like",
        "approve?",
        "allow?",
        "proceed?",
        "confirm?",
        "[y/n]",
        "(y/n)",
    ]
    .iter()
    .any(|marker| line.contains(marker))
}

fn approval_option_number(line: &str) -> Option<usize> {
    let line = line.trim_start();
    let digit_count = line.chars().take_while(|ch| ch.is_ascii_digit()).count();
    if digit_count == 0 {
        return None;
    }
    let number = line[..digit_count].parse().ok()?;
    matches!(line[digit_count..].chars().next(), Some('.' | ')')).then_some(number)
}

fn approval_reply_hint(options: &[(usize, String)]) -> String {
    let numbers = options
        .iter()
        .map(|(number, _)| number.to_string())
        .collect::<Vec<_>>();
    match numbers.as_slice() {
        [] => "请直接回复 y 或 n".to_string(),
        [only] => format!("请回复 {only}"),
        [first, second] => format!("请回复 {first} 或 {second}"),
        _ => {
            let last = numbers.last().cloned().unwrap_or_default();
            format!(
                "请回复 {} 或 {last}",
                numbers[..numbers.len() - 1].join("、")
            )
        }
    }
}

fn shell_terminal_reply(
    terminals: &TerminalManager,
    session_id: &str,
    raw_output: &[u8],
    incoming_text: &str,
) -> Option<String> {
    let screen = terminals.screen_snapshot(session_id).ok()?;
    if screen.input_mode.alternate_screen {
        return None;
    }
    if let Some(session) = terminals.ai_runtime_state_snapshot().and_then(|state| {
        state
            .sessions
            .into_iter()
            .find(|session| session.terminal_id == session_id)
    }) {
        if session.is_running || matches!(session.state.as_str(), "responding" | "needsInput") {
            return None;
        }
        if !looks_like_shell_command(incoming_text) {
            return None;
        }
    }
    terminal_shell_reply_text(raw_output, incoming_text)
}

fn looks_like_shell_command(text: &str) -> bool {
    let text = text.trim();
    let first = text.split_whitespace().next().unwrap_or_default();
    matches!(
        first,
        "ls" | "pwd"
            | "cd"
            | "git"
            | "cat"
            | "rg"
            | "grep"
            | "find"
            | "echo"
            | "printf"
            | "which"
            | "where"
            | "whoami"
            | "date"
            | "env"
            | "export"
            | "cargo"
            | "npm"
            | "pnpm"
            | "yarn"
            | "node"
            | "python"
            | "python3"
            | "claude"
            | "codex"
            | "kiro"
            | "kiro-cli"
            | "agy"
            | "opencode"
            | "mimo"
    ) || first.starts_with('/')
        || first.starts_with("./")
        || first.starts_with("~/")
        || text.contains(" && ")
        || text.contains(" | ")
}

fn terminal_shell_reply_text(raw_output: &[u8], incoming_text: &str) -> Option<String> {
    let raw_output = truncate_before_next_prompt(raw_output);
    let plain = strip_terminal_sequences(raw_output);
    let incoming_text = incoming_text.trim();
    let mut lines = plain.lines().map(str::trim_end).collect::<Vec<_>>();
    while lines.first().is_some_and(|line| line.trim().is_empty()) {
        lines.remove(0);
    }
    if lines.first().is_some_and(|line| {
        let line = line.trim();
        line == incoming_text || (!incoming_text.is_empty() && line.ends_with(incoming_text))
    }) {
        lines.remove(0);
    }
    while lines.last().is_some_and(|line| line.trim().is_empty()) {
        lines.pop();
    }
    let mut reply = lines.join("\n").trim().to_string();
    if reply.chars().count() > 8_000 {
        reply = reply.chars().take(8_000).collect::<String>();
        reply.push_str("\n…输出已截断");
    }
    (!reply.is_empty()).then_some(reply)
}

fn truncate_before_next_prompt(bytes: &[u8]) -> &[u8] {
    const PROMPT_MARK: &[u8] = b"\x1b]133;A";
    bytes
        .windows(PROMPT_MARK.len())
        .rposition(|window| window == PROMPT_MARK)
        .map(|index| &bytes[..index])
        .unwrap_or(bytes)
}

fn strip_terminal_sequences(bytes: &[u8]) -> String {
    #[derive(Clone, Copy)]
    enum State {
        Text,
        Escape,
        Csi,
        Osc,
        OscEscape,
        String,
        StringEscape,
    }

    let mut state = State::Text;
    let mut plain = Vec::with_capacity(bytes.len());
    for &byte in bytes {
        state = match state {
            State::Text if byte == 0x1b => State::Escape,
            State::Text => {
                plain.push(byte);
                State::Text
            }
            State::Escape => match byte {
                b'[' => State::Csi,
                b']' => State::Osc,
                b'P' | b'_' | b'^' => State::String,
                _ => State::Text,
            },
            State::Csi if (0x40..=0x7e).contains(&byte) => State::Text,
            State::Csi => State::Csi,
            State::Osc if byte == 0x07 => State::Text,
            State::Osc if byte == 0x1b => State::OscEscape,
            State::Osc => State::Osc,
            State::OscEscape if byte == b'\\' => State::Text,
            State::OscEscape => State::Osc,
            State::String if byte == 0x1b => State::StringEscape,
            State::String => State::String,
            State::StringEscape if byte == b'\\' => State::Text,
            State::StringEscape => State::String,
        };
    }

    let decoded = String::from_utf8_lossy(&plain).replace("\r\n", "\n");
    let mut result = String::with_capacity(decoded.len());
    for ch in decoded.chars() {
        match ch {
            '\r' => {
                if !result.ends_with('\n') {
                    result.push('\n');
                }
            }
            '\u{8}' => {
                result.pop();
            }
            '\n' | '\t' => result.push(ch),
            ch if !ch.is_control() => result.push(ch),
            _ => {}
        }
    }
    result
}

fn terminal_completion_marker(terminals: &TerminalManager, session_id: &str) -> Option<String> {
    let state = terminals.ai_runtime_state_snapshot()?;
    let session = state
        .sessions
        .iter()
        .find(|session| session.terminal_id == session_id)?;
    completed_session_marker(session)
}

fn completed_terminal_reply(
    terminals: &TerminalManager,
    session_id: &str,
) -> Option<(String, String)> {
    let state = terminals.ai_runtime_state_snapshot()?;
    let session = state
        .sessions
        .iter()
        .find(|session| session.terminal_id == session_id)?;
    let marker = completed_session_marker(session)?;
    let reply = session
        .transcript_path
        .as_deref()
        .and_then(latest_claude_assistant_reply)
        .or_else(|| session.latest_assistant_preview.clone())?;
    let reply = reply.trim().to_string();
    (!reply.is_empty()).then_some((marker, reply))
}

fn completed_session_marker(
    session: &codux_runtime_live::ai_runtime::AISessionSnapshot,
) -> Option<String> {
    if session.is_running || session.state != "idle" || !session.has_completed_turn {
        return None;
    }
    Some(format!(
        "{}:{}:{}",
        session.ai_session_id.as_deref().unwrap_or("unknown"),
        session
            .completed_turn_started_at
            .map(f64::to_bits)
            .unwrap_or_default(),
        session.latest_assistant_preview.as_deref().unwrap_or("")
    ))
}

fn latest_claude_assistant_reply(transcript_path: &str) -> Option<String> {
    const MAX_TAIL_BYTES: u64 = 2 * 1024 * 1024;
    let mut file = std::fs::File::open(transcript_path).ok()?;
    let length = file.metadata().ok()?.len();
    let start = length.saturating_sub(MAX_TAIL_BYTES);
    file.seek(SeekFrom::Start(start)).ok()?;
    let mut bytes = Vec::with_capacity((length - start) as usize);
    file.read_to_end(&mut bytes).ok()?;
    if start > 0
        && let Some(first_line_end) = bytes.iter().position(|byte| *byte == b'\n')
    {
        bytes.drain(..=first_line_end);
    }
    let text = String::from_utf8(bytes).ok()?;
    extract_latest_claude_assistant_reply(&text)
}

fn extract_latest_claude_assistant_reply(jsonl: &str) -> Option<String> {
    for line in jsonl.lines().rev() {
        let Ok(row) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        if row.get("type").and_then(|value| value.as_str()) != Some("assistant") {
            continue;
        }
        let message = row.get("message")?;
        if message.get("stop_reason").and_then(|value| value.as_str()) != Some("end_turn") {
            continue;
        }
        let blocks = message.get("content")?.as_array()?;
        let reply = blocks
            .iter()
            .filter(|block| block.get("type").and_then(|value| value.as_str()) == Some("text"))
            .filter_map(|block| block.get("text").and_then(|value| value.as_str()))
            .map(str::trim)
            .filter(|text| !text.is_empty())
            .collect::<Vec<_>>()
            .join("\n\n");
        if !reply.is_empty() {
            return Some(reply);
        }
    }
    None
}

#[cfg(test)]
mod terminal_output_tests {
    use super::{
        OutputRelayState, approval_prompt_from_screen, extract_latest_claude_assistant_reply,
        terminal_shell_reply_text,
    };

    #[test]
    fn relay_allows_only_one_reply_per_incoming_turn() {
        let mut relay = OutputRelayState::new("terminal-1".to_string(), Some("hello"));
        assert_eq!(relay.turn, 1);
        assert!(!relay.has_replied(1));
        assert_eq!(relay.incoming_text, "hello");

        relay.replied_turn = Some(1);
        assert!(relay.has_replied(1));

        relay.begin_turn("next");
        assert_eq!(relay.turn, 2);
        assert!(!relay.has_replied(2));
        assert_eq!(relay.incoming_text, "next");
    }

    #[test]
    fn extracts_only_the_latest_completed_assistant_reply() {
        let jsonl = r#"{"type":"assistant","message":{"stop_reason":"end_turn","content":[{"type":"text","text":"old"}]}}
{"type":"assistant","message":{"stop_reason":null,"content":[{"type":"text","text":"working"},{"type":"tool_use","name":"Bash"}]}}
{"type":"assistant","message":{"stop_reason":"end_turn","content":[{"type":"thinking","thinking":"private"},{"type":"text","text":"final answer"}]}}"#;

        assert_eq!(
            extract_latest_claude_assistant_reply(jsonl).as_deref(),
            Some("final answer")
        );
    }

    #[test]
    fn joins_multiple_text_blocks_and_preserves_unicode() {
        let jsonl = r#"{"type":"assistant","message":{"stop_reason":"end_turn","content":[{"type":"text","text":"第一段"},{"type":"tool_use","name":"Read"},{"type":"text","text":"第二段"}]}}"#;

        assert_eq!(
            extract_latest_claude_assistant_reply(jsonl).as_deref(),
            Some("第一段\n\n第二段")
        );
    }

    #[test]
    fn extracts_approval_prompt_from_visible_screen_tail() {
        let screen = "old output\nmore output\nBash command\n  cargo test\nDo you want to proceed?\n❯ 1. Yes\n  2. No\nPress enter to confirm or esc to go back";

        let prompt = approval_prompt_from_screen(screen).expect("approval prompt");
        assert_eq!(
            prompt,
            "【需要确认】\n\nBash command\n\ncargo test\n\nDo you want to proceed?\n\n1. Yes\n\n2. No\n\n请回复 1 或 2"
        );
        assert!(!prompt.contains("Press enter"));
        assert!(!prompt.contains('❯'));
    }

    #[test]
    fn formats_codex_hook_review_as_separate_wechat_options() {
        let screen = "Hooks need review\n4 hooks are new or changed.\nHooks can run outside the sandbox after you trust them.\n› 1. Review hooks\n2. Trust all and continue\n3. Continue without trusting (hooks won't run)\nPress enter to confirm or esc to go back";

        assert_eq!(
            approval_prompt_from_screen(screen).as_deref(),
            Some(
                "【需要确认】\n\nHooks need review\n\n4 hooks are new or changed.\n\nHooks can run outside the sandbox after you trust them.\n\n1. Review hooks\n\n2. Trust all and continue\n\n3. Continue without trusting (hooks won't run)\n\n请回复 1、2 或 3"
            )
        );
    }

    #[test]
    fn cleans_shell_output_and_stops_before_the_next_prompt() {
        let raw = b"ls\r\n\x1b[32mCargo.toml\x1b[0m\r\n\xe4\xb8\xad\xe6\x96\x87.txt\r\n\x1b]133;A\x07prompt";

        assert_eq!(
            terminal_shell_reply_text(raw, "ls").as_deref(),
            Some("Cargo.toml\n中文.txt")
        );
    }
}

/// Begin QR login: fetches a QR code and polls it until confirmed/expired.
/// Status transitions surface via [`wechat_bridge_snapshot`].
pub fn wechat_bridge_begin_login() {
    let generation = GENERATION.fetch_add(1, Ordering::SeqCst) + 1;
    let bridge = ensure_bridge();
    async_runtime::spawn(async move {
        let qrcode = match bridge.begin_login().await {
            Ok(qrcode) => qrcode,
            Err(e) => {
                set_status(BridgeStatus::Error {
                    message: e.to_string(),
                });
                return;
            }
        };
        loop {
            if GENERATION.load(Ordering::SeqCst) != generation {
                return; // superseded by a newer login/stop
            }
            match bridge.poll_login_once(&qrcode).await {
                Ok(true) => return,
                Ok(false) => {}
                Err(e) => {
                    set_status(BridgeStatus::Error {
                        message: e.to_string(),
                    });
                    return;
                }
            }
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        }
    });
}

/// Resume the poll loop from saved credentials (no QR needed).
pub fn wechat_bridge_start_saved() {
    GENERATION.fetch_add(1, Ordering::SeqCst);
    let bridge = ensure_bridge();
    set_status(BridgeStatus::Connecting);
    crate::runtime_trace::runtime_trace("wechat", "start_saved queued");
    async_runtime::spawn(async move {
        if let Err(e) = bridge.start_from_saved().await {
            set_status(BridgeStatus::Error {
                message: e.to_string(),
            });
        }
    });
}

/// Stop polling; credentials are kept for a later resume.
pub fn wechat_bridge_stop() {
    GENERATION.fetch_add(1, Ordering::SeqCst);
    let bridge = ensure_bridge();
    async_runtime::spawn(async move {
        bridge.stop().await;
    });
}

/// Stop and forget credentials, cursor, and QR state.
pub fn wechat_bridge_logout() {
    GENERATION.fetch_add(1, Ordering::SeqCst);
    let bridge = ensure_bridge();
    async_runtime::spawn(async move {
        bridge.logout().await;
    });
}

/// Confirm the pending pairing request, binding that peer to `session_id`.
/// Returns false when nothing is pending (the code comes from the snapshot, so
/// it always matches).
pub fn wechat_bridge_confirm_pairing(session_id: &str) -> bool {
    let Some((chat_id, code)) = status_cell().lock().pending_pairing.clone() else {
        crate::runtime_trace::runtime_trace(
            "wechat",
            &format!("confirm_pairing skipped reason=no_pending session={session_id}"),
        );
        return false;
    };
    let bridge = ensure_bridge();
    let session_id = session_id.to_string();
    crate::runtime_trace::runtime_trace(
        "wechat",
        &format!(
            "confirm_pairing queued chat={} session={session_id}",
            masked_chat_id(&chat_id)
        ),
    );
    async_runtime::spawn(async move {
        let confirmed = bridge
            .confirm_pairing(&chat_id, &code, &session_id, None)
            .await;
        if confirmed {
            status_cell().lock().pending_pairing = None;
            crate::runtime_trace::runtime_trace(
                "wechat",
                &format!(
                    "confirm_pairing ok chat={} session={session_id}",
                    masked_chat_id(&chat_id)
                ),
            );
        } else {
            crate::runtime_trace::runtime_trace(
                "wechat",
                &format!(
                    "confirm_pairing failed chat={} session={session_id}",
                    masked_chat_id(&chat_id)
                ),
            );
        }
    });
    true
}

/// Drop the pending pairing request without binding.
pub fn wechat_bridge_dismiss_pairing() {
    status_cell().lock().pending_pairing = None;
}
