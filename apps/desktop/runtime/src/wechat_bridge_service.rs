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
use std::time::Duration;

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
}

impl OutputRelayState {
    fn new(session_id: String, begins_turn: bool) -> Self {
        Self {
            session_id,
            turn: u64::from(begins_turn),
            replied_turn: None,
        }
    }

    fn begin_turn(&mut self) {
        self.turn = self.turn.saturating_add(1);
        self.replied_turn = None;
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
        start_output_relay(chat_id.to_string(), session_id.to_string(), true);
    }

    fn on_pairing_confirmed(&self, chat_id: &str, session_id: &str) {
        crate::runtime_trace::runtime_trace(
            "wechat",
            &format!(
                "pairing_confirmed chat={} session={session_id}",
                masked_chat_id(chat_id)
            ),
        );
        start_output_relay(chat_id.to_string(), session_id.to_string(), false);
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
    start_output_relay(chat_id.clone(), session_id.clone(), false);
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

fn start_output_relay(chat_id: String, session_id: String, begins_turn: bool) {
    let should_spawn = {
        let mut relays = output_relays_cell().lock();
        if let Some(relay) = relays.get_mut(&chat_id)
            && relay.session_id == session_id
        {
            if begins_turn {
                relay.begin_turn();
            }
            false
        } else {
            relays.insert(
                chat_id.clone(),
                OutputRelayState::new(session_id.clone(), begins_turn),
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
    let mut interval = tokio::time::interval(Duration::from_millis(800));
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
        }
        tokio::select! {
            chunk = rx.recv_async() => {
                let Ok(_) = chunk else {
                    clear_output_relay(&chat_id, &session_id);
                    return;
                };
                output_active = true;
            }
            _ = interval.tick() => {
                if !output_active {
                    continue;
                }
                let _ = terminals.poll_ai_runtime_state();
                let Some((marker, reply)) = completed_terminal_reply(&terminals, &session_id) else {
                    continue;
                };
                if last_completion.as_ref() == Some(&marker) {
                    continue;
                }
                if output_relay_has_replied(&chat_id, &session_id, observed_turn) {
                    output_active = false;
                    continue;
                }
                if bridge.send_reply_to_chat(&chat_id, &reply).await {
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
    use super::{OutputRelayState, extract_latest_claude_assistant_reply};

    #[test]
    fn relay_allows_only_one_reply_per_incoming_turn() {
        let mut relay = OutputRelayState::new("terminal-1".to_string(), true);
        assert_eq!(relay.turn, 1);
        assert!(!relay.has_replied(1));

        relay.replied_turn = Some(1);
        assert!(relay.has_replied(1));

        relay.begin_turn();
        assert_eq!(relay.turn, 2);
        assert!(!relay.has_replied(2));
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
