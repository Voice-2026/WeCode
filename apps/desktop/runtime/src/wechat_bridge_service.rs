//! Desktop lifecycle wrapper around the WeChat chat bridge
//! (`wecode-im-bridge`): owns the singleton bridge instance, persists
//! credentials/cursor/bindings under the app support dir, and exposes a
//! poll-friendly status snapshot for the settings UI.
//!
//! The bridge routes an authorized WeChat peer's text into a bound terminal
//! session (`TerminalManager::write`), which is where the AI CLI runs. Pairing
//! follows the allowlist-first model: a new peer receives a pairing code and a
//! desktop user must confirm it against a chosen terminal session.

use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use parking_lot::Mutex;
use wecode_im_bridge::binding::{BindingState, BindingStore, ChatBinding};
use wecode_im_bridge::runtime::{BridgeStatus, CredentialStore, HostSink, ReplyText, WeChatBridge};
use wecode_im_bridge::wechat::WeChatCredentials;
use wecode_runtime_live::terminal_pty::{
    TerminalIoDirection, TerminalIoEvent, TerminalIoOrigin, TerminalManager,
};

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
}

impl OutputRelayState {
    fn new(session_id: String) -> Self {
        Self { session_id }
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
        wecode_im_bridge::binding::write_json_600(&self.paths.credentials_path, creds);
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
        wecode_im_bridge::binding::write_json_600(
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
        terminals.write_from_wechat(session_id, &bytes).is_ok()
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
        pairing_prompt: "WeCode 配对码 / pairing code: {code}\n请在 WeCode 桌面端「远程 → 微信」中确认。 / Confirm it in WeCode desktop under Remote → WeChat.".to_string(),
        needs_binding: "该微信已授权，但还没有绑定终端会话。请在 WeCode 桌面端选择一个会话完成绑定。 / Authorized but not bound to a terminal session yet. Pick one in WeCode desktop.".to_string(),
        rejected: "未授权访问。 / Not authorized.".to_string(),
        pairing_confirmed: "✅ 绑定成功。现在发送的消息会输入到绑定的 WeCode 终端。 / Paired. Your messages now go to the bound WeCode terminal.".to_string(),
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
    *cell = Some(terminals.clone());
    drop(cell);

    let paths = FileCredentialStore::default_paths();
    let bindings = std::fs::read_to_string(paths.bindings_path)
        .ok()
        .and_then(|text| serde_json::from_str::<BindingState>(&text).ok())
        .map(|state| state.bindings.into_values().collect::<Vec<_>>())
        .unwrap_or_default();
    let sessions = terminals
        .list()
        .into_iter()
        .map(|session| session.id)
        .collect::<std::collections::HashSet<_>>();
    for binding in bindings {
        if sessions.contains(&binding.session_id) {
            start_output_relay(binding.chat_id, binding.session_id, None);
        }
    }
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

fn start_output_relay(chat_id: String, session_id: String, _incoming_text: Option<&str>) {
    let should_spawn = {
        let mut relays = output_relays_cell().lock();
        if let Some(relay) = relays.get_mut(&chat_id)
            && relay.session_id == session_id
        {
            false
        } else {
            relays.insert(chat_id.clone(), OutputRelayState::new(session_id.clone()));
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
    let rx = match terminals.subscribe_io(&session_id) {
        Ok(rx) => rx,
        Err(_) => {
            output_relays_cell().lock().remove(&chat_id);
            return;
        }
    };
    let bridge = ensure_bridge();
    async_runtime::spawn(async move {
        relay_terminal_io_to_wechat(terminals, bridge, chat_id, session_id, rx).await;
    });
}

#[derive(Debug)]
struct TerminalIoBatch {
    direction: TerminalIoDirection,
    origin: TerminalIoOrigin,
    first_sequence: u64,
    last_sequence: u64,
    bytes: Vec<u8>,
    updated_at: Instant,
}

impl TerminalIoBatch {
    fn from_event(event: TerminalIoEvent) -> Self {
        Self {
            direction: event.direction,
            origin: event.origin,
            first_sequence: event.sequence,
            last_sequence: event.sequence,
            bytes: event.bytes,
            updated_at: Instant::now(),
        }
    }

    fn can_merge(&self, event: &TerminalIoEvent) -> bool {
        self.direction == event.direction
            && self.origin == event.origin
            && self.last_sequence.saturating_add(1) == event.sequence
            && self.bytes.len().saturating_add(event.bytes.len()) <= 12 * 1024
    }

    fn push(&mut self, event: TerminalIoEvent) {
        self.last_sequence = event.sequence;
        self.bytes.extend_from_slice(&event.bytes);
        self.updated_at = Instant::now();
    }

    fn push_input(&mut self, event: TerminalIoEvent) {
        self.last_sequence = event.sequence;
        self.bytes.extend_from_slice(&event.bytes);
        self.updated_at = Instant::now();
    }

    fn input_ready(&self) -> bool {
        self.bytes.iter().any(|byte| matches!(byte, b'\r' | b'\n'))
            || self.bytes.len() >= 12 * 1024
            || self
                .bytes
                .iter()
                .any(|byte| *byte < 0x20 && !matches!(*byte, b'\t' | 0x08))
    }
}

async fn relay_terminal_io_to_wechat(
    terminals: Arc<TerminalManager>,
    bridge: WeChatBridge,
    chat_id: String,
    session_id: String,
    rx: flume::Receiver<TerminalIoEvent>,
) {
    let mut local_input: Option<TerminalIoBatch> = None;
    let mut output_batches = VecDeque::<TerminalIoBatch>::new();
    let mut pending_echo: Option<String> = None;
    let mut last_screen_output: Option<String> = None;
    let mut last_output_at: Option<Instant> = None;
    let mut interval = tokio::time::interval(Duration::from_millis(100));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    loop {
        if !output_relay_is_current(&chat_id, &session_id) {
            return;
        }
        tokio::select! {
            event = rx.recv_async() => {
                let Ok(event) = event else {
                    clear_output_relay(&chat_id, &session_id);
                    return;
                };
                match (event.direction, event.origin) {
                    (TerminalIoDirection::Input, TerminalIoOrigin::WeChat) => {
                        pending_echo = terminal_input_echo_text(&event.bytes);
                    }
                    (TerminalIoDirection::Input, _) => {
                        if let Some(batch) = local_input.as_mut() {
                            batch.push_input(event);
                        } else {
                            local_input = Some(TerminalIoBatch::from_event(event));
                        }
                    }
                    (TerminalIoDirection::Output, _) => {
                        last_output_at = Some(Instant::now());
                        if let Some(batch) = output_batches
                            .back_mut()
                            .filter(|batch| batch.can_merge(&event))
                        {
                            batch.push(event);
                        } else {
                            output_batches.push_back(TerminalIoBatch::from_event(event));
                        }
                    }
                }
            }
            _ = interval.tick() => {
                if local_input.as_ref().is_some_and(TerminalIoBatch::input_ready) {
                    let batch = local_input.as_ref().expect("input batch exists");
                    let message = terminal_io_batch_message(batch);
                    if !bridge.send_reply_to_chat(&chat_id, &message).await {
                        continue;
                    }
                    pending_echo = terminal_input_echo_text(&batch.bytes);
                    crate::runtime_trace::runtime_trace(
                        "wechat",
                        &format!(
                            "terminal_io sent chat={} session={session_id} direction={:?} sequence={}-{} bytes={}",
                            masked_chat_id(&chat_id),
                            batch.direction,
                            batch.first_sequence,
                            batch.last_sequence,
                            batch.bytes.len(),
                        ),
                    );
                    local_input = None;
                }

                if local_input.is_some() {
                    continue;
                }
                if !last_output_at.is_some_and(|at| at.elapsed() >= Duration::from_millis(700)) {
                    continue;
                }
                while !output_batches.is_empty() {
                    let batch = output_batches.front().expect("output batch exists");
                    let rendered_screen = terminal_output_prefers_screen(
                        &terminals,
                        &session_id,
                        batch,
                    )
                    .then(|| rendered_terminal_screen(&terminals, &session_id))
                    .flatten();
                    let mut content = rendered_screen
                        .clone()
                        .unwrap_or_else(|| terminal_io_batch_content(batch));
                    let echo = pending_echo.clone();
                    if let Some(echo) = echo.as_deref() {
                        content = strip_terminal_echo(&content, echo);
                    }
                    if rendered_screen.is_some()
                        && last_screen_output.as_deref() == Some(content.as_str())
                    {
                        if echo.is_some() {
                            pending_echo = None;
                        }
                        output_batches.pop_front();
                        continue;
                    }
                    if content.trim().is_empty() {
                        if echo.is_some() {
                            pending_echo = None;
                        }
                        output_batches.pop_front();
                        continue;
                    }
                    let message = terminal_io_batch_message_with_content(batch, &content);
                    if !bridge.send_reply_to_chat(&chat_id, &message).await {
                        break;
                    }
                    if echo.is_some() {
                        pending_echo = None;
                    }
                    if rendered_screen.is_some() {
                        last_screen_output = Some(content.clone());
                    }
                    crate::runtime_trace::runtime_trace(
                        "wechat",
                        &format!(
                            "terminal_io sent chat={} session={session_id} direction={:?} sequence={}-{} bytes={}",
                            masked_chat_id(&chat_id),
                            batch.direction,
                            batch.first_sequence,
                            batch.last_sequence,
                            batch.bytes.len(),
                        ),
                    );
                    output_batches.pop_front();
                }
                if output_batches.is_empty() {
                    last_output_at = None;
                }
            }
        }
    }
}

fn terminal_io_batch_message(batch: &TerminalIoBatch) -> String {
    terminal_io_batch_message_with_content(batch, &terminal_io_batch_content(batch))
}

fn terminal_io_batch_message_with_content(batch: &TerminalIoBatch, content: &str) -> String {
    let direction = match batch.direction {
        TerminalIoDirection::Input => "输入",
        TerminalIoDirection::Output => "输出",
    };
    let sequence = if batch.first_sequence == batch.last_sequence {
        batch.first_sequence.to_string()
    } else {
        format!("{}-{}", batch.first_sequence, batch.last_sequence)
    };
    format!("【终端{direction} #{sequence}】\n{content}")
}

fn terminal_io_batch_content(batch: &TerminalIoBatch) -> String {
    match batch.direction {
        TerminalIoDirection::Input => terminal_input_display_text(&batch.bytes),
        TerminalIoDirection::Output => {
            let text = strip_terminal_sequences(&batch.bytes);
            if text.is_empty() {
                format!("〈终端控制数据：{} 字节〉", batch.bytes.len())
            } else {
                text
            }
        }
    }
}

fn terminal_input_echo_text(bytes: &[u8]) -> Option<String> {
    let text = String::from_utf8_lossy(bytes)
        .trim_end_matches(['\r', '\n'])
        .to_string();
    (!text.is_empty()).then_some(text)
}

fn terminal_output_prefers_screen(
    terminals: &TerminalManager,
    session_id: &str,
    batch: &TerminalIoBatch,
) -> bool {
    let escape_count = batch.bytes.iter().filter(|byte| **byte == 0x1b).count();
    escape_count >= 8
        || batch.bytes.len() >= 12 * 1024
        || terminals
            .screen_snapshot(session_id)
            .ok()
            .is_some_and(|snapshot| snapshot.input_mode.alternate_screen)
}

fn rendered_terminal_screen(terminals: &TerminalManager, session_id: &str) -> Option<String> {
    let snapshot = terminals.screen_snapshot(session_id).ok()?;
    let text = wecode_runtime_live::ai_runtime::screen_signal::screen_text_from_cells(&snapshot);
    let text = text.trim_end().to_string();
    (!text.is_empty()).then_some(text)
}

fn strip_terminal_echo(output: &str, echo: &str) -> String {
    let mut removed = false;
    let mut lines = output.lines().collect::<Vec<_>>();
    lines.retain(|line| {
        if removed {
            return true;
        }
        let line = line.trim();
        if line == echo || line.ends_with(echo) {
            removed = true;
            false
        } else {
            true
        }
    });
    let mut result = lines.join("\n");
    if output.ends_with('\n') && !result.is_empty() {
        result.push('\n');
    }
    result
}

fn terminal_input_display_text(bytes: &[u8]) -> String {
    let mut text = String::new();
    for ch in String::from_utf8_lossy(bytes).chars() {
        match ch {
            '\r' | '\n' => text.push_str("↵\n"),
            '\t' => text.push('⇥'),
            '\u{8}' | '\u{7f}' => text.push('⌫'),
            '\u{1b}' => text.push('␛'),
            ch if ch <= '\u{1f}' => {
                text.push('^');
                text.push(char::from_u32(ch as u32 + 0x40).unwrap_or('�'));
            }
            _ => text.push(ch),
        }
    }
    text
}

fn output_relay_is_current(chat_id: &str, session_id: &str) -> bool {
    output_relays_cell()
        .lock()
        .get(chat_id)
        .map(|relay| relay.session_id.as_str())
        == Some(session_id)
}

fn clear_output_relay(chat_id: &str, session_id: &str) {
    let mut relays = output_relays_cell().lock();
    if relays.get(chat_id).map(|relay| relay.session_id.as_str()) == Some(session_id) {
        relays.remove(chat_id);
    }
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

#[cfg(test)]
mod terminal_output_tests {
    use super::{TerminalIoBatch, strip_terminal_echo, terminal_io_batch_message};
    use std::time::Instant;
    use wecode_runtime_live::terminal_pty::{
        TerminalIoDirection, TerminalIoEvent, TerminalIoOrigin,
    };

    #[test]
    fn keeps_slow_printable_input_buffered_until_enter() {
        let mut batch = TerminalIoBatch {
            direction: TerminalIoDirection::Input,
            origin: TerminalIoOrigin::Local,
            first_sequence: 1,
            last_sequence: 1,
            bytes: b"c".to_vec(),
            updated_at: Instant::now() - std::time::Duration::from_secs(10),
        };
        assert!(!batch.input_ready());

        batch.push_input(TerminalIoEvent {
            sequence: 9,
            direction: TerminalIoDirection::Input,
            origin: TerminalIoOrigin::Local,
            bytes: b"laude\r".to_vec(),
        });

        assert!(batch.input_ready());
        assert_eq!(
            terminal_io_batch_message(&batch),
            "【终端输入 #1-9】\nclaude↵\n"
        );
    }

    #[test]
    fn formats_full_terminal_input_including_passwords_and_control_keys() {
        let batch = TerminalIoBatch {
            direction: TerminalIoDirection::Input,
            origin: TerminalIoOrigin::Local,
            first_sequence: 7,
            last_sequence: 9,
            bytes: "密码secret\r\u{3}".as_bytes().to_vec(),
            updated_at: Instant::now(),
        };

        assert_eq!(
            terminal_io_batch_message(&batch),
            "【终端输入 #7-9】\n密码secret↵\n^C"
        );
    }

    #[test]
    fn formats_terminal_output_without_ansi_sequences() {
        let batch = TerminalIoBatch {
            direction: TerminalIoDirection::Output,
            origin: TerminalIoOrigin::Pty,
            first_sequence: 10,
            last_sequence: 10,
            bytes: b"\x1b[32mok\x1b[0m\r\n".to_vec(),
            updated_at: Instant::now(),
        };

        assert_eq!(terminal_io_batch_message(&batch), "【终端输出 #10】\nok\n");
    }

    #[test]
    fn removes_the_command_echo_but_keeps_command_output() {
        assert_eq!(
            strip_terminal_echo("ls\nREADME.md\ndocs\n", "ls"),
            "README.md\ndocs\n"
        );
        assert_eq!(
            strip_terminal_echo("prompt $ ls\nREADME.md\n", "ls"),
            "README.md\n"
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
