use std::collections::{BTreeMap, HashMap, HashSet};

use serde::{Deserialize, Serialize};

pub type TerminalSequence = i64;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerminalBaselineRequest {
    pub session_id: String,
    pub request_id: Option<String>,
    pub offset: usize,
    pub max_chars: usize,
    pub chunk_chars: Option<usize>,
    pub tail: bool,
    pub resume_from_seq: Option<TerminalSequence>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalLaunchConfig {
    pub cwd: Option<String>,
    pub shell: Option<String>,
    pub command: Option<String>,
    pub cols: Option<u16>,
    pub rows: Option<u16>,
    pub scrollback_lines: Option<usize>,
    pub env: Option<HashMap<String, String>>,
    pub project_id: Option<String>,
    pub project_name: Option<String>,
    pub terminal_id: Option<String>,
    pub slot_id: Option<String>,
    pub session_key: Option<String>,
    pub title: Option<String>,
    pub tool: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum TerminalEvent {
    Output {
        #[serde(rename = "sessionId")]
        session_id: String,
        #[serde(skip_serializing_if = "String::is_empty")]
        text: String,
        #[serde(skip)]
        bytes: Vec<u8>,
    },
    Exit {
        #[serde(rename = "sessionId")]
        session_id: String,
        #[serde(rename = "exitCode")]
        exit_code: Option<i32>,
    },
    Error {
        #[serde(rename = "sessionId")]
        session_id: String,
        message: String,
    },
    Viewport {
        #[serde(rename = "sessionId")]
        session_id: String,
        owner: String,
        cols: u16,
        rows: u16,
        generation: u64,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TerminalViewportState {
    pub owner: String,
    pub cols: u16,
    pub rows: u16,
    pub generation: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalSessionSnapshot {
    pub id: String,
    pub title: String,
    pub slot_id: String,
    pub session_key: Option<String>,
    pub project_id: String,
    pub project_name: String,
    pub cwd: String,
    pub shell: String,
    pub command: String,
    pub cols: u16,
    pub rows: u16,
    pub status: String,
    pub is_running: bool,
    pub created_at: String,
    pub last_active_at: String,
    pub buffer_characters: usize,
    pub has_buffer: bool,
}

pub type TerminalEventSink = Box<dyn Fn(TerminalEvent) -> bool + Send + Sync + 'static>;

pub trait TerminalSessionHandle: Send + Sync {
    fn id(&self) -> &str;
    fn info(&self) -> TerminalSessionSnapshot;
    fn write(&self, data: &[u8]) -> Result<(), String>;
    fn resize(&self, cols: u16, rows: u16) -> Result<(), String>;
    fn claim_viewport(&self, owner: &str) -> Result<TerminalViewportState, String>;
    fn release_viewport(&self, owner: &str) -> Result<Option<TerminalViewportState>, String>;
    fn resize_viewport(
        &self,
        owner: &str,
        cols: u16,
        rows: u16,
    ) -> Result<Option<TerminalViewportState>, String>;
    fn viewport_state(&self) -> TerminalViewportState;
    fn snapshot(&self) -> String;
    fn snapshot_tail(&self, max_chars: usize) -> (String, usize);
    fn buffer_characters(&self) -> usize;
    fn clear_history(&self);
    fn kill(&self) -> Result<(), String>;
}

pub trait TerminalDriver: Send + Sync {
    type Session: TerminalSessionHandle + Clone + 'static;

    fn list(&self) -> Vec<TerminalSessionSnapshot>;
    fn create(
        &self,
        config: TerminalLaunchConfig,
        emit: TerminalEventSink,
    ) -> Result<Self::Session, String>;
    fn session(&self, session_id: &str) -> Result<Self::Session, String>;
    fn remove(&self, session_id: &str) -> Result<(), String>;
    fn subscribe_events(&self, session_id: &str, emit: TerminalEventSink) -> Result<(), String>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemotePtySnapshot {
    pub session_id: String,
    pub content: String,
    pub buffer_length: usize,
    pub sequence: TerminalSequence,
}

#[derive(Debug, Clone)]
pub struct RemotePtySession<T> {
    session_id: String,
    max_cached_chars: usize,
    content: String,
    buffer_length: usize,
    sequence: TerminalSequence,
    awaiting_baseline: bool,
    page_buffer: Option<RemotePtyPageBuffer>,
    held_sequenced_live: BTreeMap<TerminalSequence, T>,
    held_unsequenced_live: Vec<T>,
}

impl<T> RemotePtySession<T> {
    pub fn new(session_id: impl Into<String>, max_cached_chars: usize) -> Self {
        Self {
            session_id: session_id.into(),
            max_cached_chars,
            content: String::new(),
            buffer_length: 0,
            sequence: 0,
            awaiting_baseline: false,
            page_buffer: None,
            held_sequenced_live: BTreeMap::new(),
            held_unsequenced_live: Vec::new(),
        }
    }

    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    pub fn content(&self) -> &str {
        &self.content
    }

    pub fn buffer_length(&self) -> usize {
        self.buffer_length
    }

    pub fn sequence(&self) -> TerminalSequence {
        self.sequence
    }

    pub fn is_restoring_baseline(&self) -> bool {
        self.awaiting_baseline || self.page_buffer.is_some()
    }

    pub fn snapshot(&self) -> RemotePtySnapshot {
        RemotePtySnapshot {
            session_id: self.session_id.clone(),
            content: self.content.clone(),
            buffer_length: self.buffer_length,
            sequence: self.sequence,
        }
    }

    pub fn require_baseline(&mut self) {
        self.awaiting_baseline = true;
        self.page_buffer = None;
        self.held_sequenced_live.clear();
        self.held_unsequenced_live.clear();
    }

    pub fn reset_transient(&mut self, reset_sequence: bool) {
        self.awaiting_baseline = false;
        self.page_buffer = None;
        self.held_sequenced_live.clear();
        self.held_unsequenced_live.clear();
        if reset_sequence {
            self.sequence = 0;
        }
    }

    pub fn set_sequence(&mut self, sequence: TerminalSequence) {
        self.sequence = sequence;
    }

    pub fn hold_live(&mut self, sequence: Option<TerminalSequence>, output: T) -> bool {
        if !self.awaiting_baseline {
            return false;
        }
        if let Some(sequence) = sequence {
            self.held_sequenced_live.entry(sequence).or_insert(output);
        } else {
            self.held_unsequenced_live.push(output);
        }
        true
    }

    pub fn accept_baseline_page(
        &mut self,
        data: &str,
        offset: usize,
        buffer_length: Option<usize>,
        truncated: bool,
    ) -> RemotePtyBaselinePageResult {
        let mut page_buffer = if offset == 0 || self.page_buffer.is_none() {
            RemotePtyPageBuffer::new(buffer_length, offset)
        } else {
            self.page_buffer.take().expect("page buffer exists")
        };
        let accepted = page_buffer.accept(data, offset, buffer_length, truncated);
        if !accepted.accepted {
            self.page_buffer = None;
            return accepted;
        }
        if accepted.ready {
            self.page_buffer = None;
        } else {
            self.buffer_length = accepted.next_offset;
            self.page_buffer = Some(page_buffer);
        }
        accepted
    }

    pub fn replace_from_baseline(
        &mut self,
        content: &str,
        buffer_length: Option<usize>,
        sequence: Option<TerminalSequence>,
    ) -> Vec<T> {
        self.content = trim_to_char_limit(content, self.max_cached_chars);
        if let Some(buffer_length) = buffer_length {
            self.buffer_length = buffer_length;
        }
        let base_sequence = sequence.unwrap_or(self.sequence);
        self.sequence = base_sequence;
        self.awaiting_baseline = false;
        self.page_buffer = None;

        let mut replay = Vec::new();
        let held_sequenced_live = std::mem::take(&mut self.held_sequenced_live);
        for (sequence, output) in held_sequenced_live {
            if sequence > base_sequence {
                replay.push(output);
            }
        }
        replay.append(&mut self.held_unsequenced_live);
        replay
    }

    pub fn append_live(
        &mut self,
        data: &str,
        buffer_length: Option<usize>,
        sequence: Option<TerminalSequence>,
    ) {
        if !data.is_empty() {
            self.content =
                trim_to_char_limit(&format!("{}{}", self.content, data), self.max_cached_chars);
        }
        if let Some(buffer_length) = buffer_length {
            self.buffer_length = buffer_length;
        }
        if let Some(sequence) = sequence {
            self.sequence = sequence;
        }
    }

    pub fn clear(&mut self) {
        self.content.clear();
        self.buffer_length = 0;
        self.sequence = 0;
        self.reset_transient(false);
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RemotePtyBaselinePageResult {
    pub accepted: bool,
    pub duplicate: bool,
    pub ready: bool,
    pub data: String,
    pub next_offset: usize,
    pub progress: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalOutputSequenceAction {
    Accept,
    Duplicate,
    Baseline,
}

impl TerminalOutputSequenceAction {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Accept => "accept",
            Self::Duplicate => "duplicate",
            Self::Baseline => "baseline",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerminalOutputSequenceResult {
    pub action: TerminalOutputSequenceAction,
    pub previous_seq: TerminalSequence,
}

impl TerminalOutputSequenceResult {
    pub fn should_render(&self) -> bool {
        matches!(
            self.action,
            TerminalOutputSequenceAction::Accept | TerminalOutputSequenceAction::Baseline
        )
    }
}

#[derive(Debug, Clone, Default)]
pub struct TerminalOutputSequencer {
    seq_by_session: HashMap<String, TerminalSequence>,
    allow_next_live_rebase_sessions: HashSet<String>,
}

impl TerminalOutputSequencer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn sequence_for(&self, session_id: &str) -> TerminalSequence {
        self.seq_by_session.get(session_id).copied().unwrap_or(0)
    }

    pub fn is_resyncing(&self, _session_id: &str) -> bool {
        false
    }

    pub fn observe(
        &mut self,
        session_id: impl AsRef<str>,
        is_buffer: bool,
        output_seq: Option<TerminalSequence>,
        offset: Option<usize>,
        resets_sequence: bool,
    ) -> TerminalOutputSequenceResult {
        let session_id = session_id.as_ref();
        let previous_seq = self.sequence_for(session_id);
        if is_buffer {
            let should_reset = offset.unwrap_or(0) <= 0 || resets_sequence;
            if should_reset {
                self.allow_next_live_rebase_sessions
                    .insert(session_id.to_string());
                if let Some(output_seq) = output_seq {
                    self.seq_by_session
                        .insert(session_id.to_string(), output_seq);
                }
            } else if let Some(output_seq) = output_seq {
                if output_seq >= previous_seq {
                    self.seq_by_session
                        .insert(session_id.to_string(), output_seq);
                }
            }
            return TerminalOutputSequenceResult {
                action: TerminalOutputSequenceAction::Baseline,
                previous_seq,
            };
        }

        let Some(output_seq) = output_seq else {
            return TerminalOutputSequenceResult {
                action: TerminalOutputSequenceAction::Accept,
                previous_seq,
            };
        };
        if output_seq <= previous_seq {
            return TerminalOutputSequenceResult {
                action: TerminalOutputSequenceAction::Duplicate,
                previous_seq,
            };
        }
        let allow_rebase = self.allow_next_live_rebase_sessions.remove(session_id);
        if (allow_rebase || previous_seq > 0) && output_seq > previous_seq {
            self.seq_by_session
                .insert(session_id.to_string(), output_seq);
            return TerminalOutputSequenceResult {
                action: TerminalOutputSequenceAction::Accept,
                previous_seq,
            };
        }
        self.seq_by_session
            .insert(session_id.to_string(), output_seq);
        self.allow_next_live_rebase_sessions.remove(session_id);
        TerminalOutputSequenceResult {
            action: TerminalOutputSequenceAction::Accept,
            previous_seq,
        }
    }

    pub fn remove(&mut self, session_id: &str) {
        self.seq_by_session.remove(session_id);
        self.allow_next_live_rebase_sessions.remove(session_id);
    }

    pub fn reset(&mut self) {
        self.seq_by_session.clear();
        self.allow_next_live_rebase_sessions.clear();
    }
}

#[derive(Debug, Clone)]
struct RemotePtyPageBuffer {
    buffer: String,
    next_offset: usize,
    buffer_length: Option<usize>,
}

impl RemotePtyPageBuffer {
    fn new(buffer_length: Option<usize>, next_offset: usize) -> Self {
        Self {
            buffer: String::new(),
            next_offset,
            buffer_length,
        }
    }

    fn accept(
        &mut self,
        data: &str,
        offset: usize,
        buffer_length: Option<usize>,
        truncated: bool,
    ) -> RemotePtyBaselinePageResult {
        if self.buffer_length.is_none() {
            self.buffer_length = buffer_length;
        }
        if offset != self.next_offset {
            let data_chars = data.chars().count();
            let duplicate = offset.saturating_add(data_chars) <= self.next_offset;
            return RemotePtyBaselinePageResult {
                accepted: false,
                duplicate,
                ready: false,
                data: String::new(),
                next_offset: self.next_offset,
                progress: None,
            };
        }

        self.buffer.push_str(data);
        self.next_offset += data.chars().count();
        let expected_length = buffer_length.or(self.buffer_length);
        let complete_by_length = expected_length
            .map(|length| self.next_offset >= length)
            .unwrap_or(false);
        let ready = !truncated || complete_by_length;
        RemotePtyBaselinePageResult {
            accepted: true,
            duplicate: false,
            ready,
            data: if ready {
                self.buffer.clone()
            } else {
                String::new()
            },
            next_offset: self.next_offset,
            progress: expected_length
                .filter(|length| *length > 0)
                .map(|length| (self.next_offset as f64 / length as f64).clamp(0.0, 1.0)),
        }
    }
}

fn trim_to_char_limit(value: &str, max_chars: usize) -> String {
    let total = value.chars().count();
    if total <= max_chars {
        return value.to_string();
    }
    value.chars().skip(total - max_chars).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn restores_baseline_before_replaying_held_live_output() {
        let mut session = RemotePtySession::new("session-1", 64);
        session.require_baseline();

        assert!(session.hold_live(Some(11), "stale"));
        assert!(session.hold_live(Some(12), "new"));

        let page = session.accept_baseline_page("abcd", 0, Some(8), true);
        assert!(page.accepted);
        assert!(!page.ready);
        assert_eq!(page.next_offset, 4);

        let page = session.accept_baseline_page("efgh", 4, Some(8), false);
        assert!(page.ready);

        let replay = session.replace_from_baseline(&page.data, Some(8), Some(11));
        assert_eq!(session.content(), "abcdefgh");
        assert_eq!(replay, vec!["new"]);
    }

    #[test]
    fn rejects_out_of_order_baseline_pages() {
        let mut session = RemotePtySession::<String>::new("session-1", 64);
        session.require_baseline();

        let page = session.accept_baseline_page("abcd", 0, Some(8), true);
        assert!(page.accepted);

        let page = session.accept_baseline_page("gh", 6, Some(8), false);
        assert!(!page.accepted);
        assert_eq!(page.next_offset, 4);
    }

    #[test]
    fn trims_cache_on_character_boundaries() {
        let mut session = RemotePtySession::<String>::new("session-1", 4);

        session.append_live("a你好bcd", Some(7), Some(2));

        assert_eq!(session.content(), "好bcd");
        assert_eq!(session.buffer_length(), 7);
        assert_eq!(session.sequence(), 2);
    }

    #[test]
    fn output_sequencer_drops_duplicates_and_tracks_buffers() {
        let mut sequencer = TerminalOutputSequencer::new();

        let first = sequencer.observe("term-1", false, Some(1), None, false);
        let second = sequencer.observe("term-1", false, Some(2), None, false);
        let duplicate = sequencer.observe("term-1", false, Some(2), None, false);
        let baseline = sequencer.observe("term-1", true, Some(2), Some(0), false);
        let next = sequencer.observe("term-1", false, Some(3), None, false);

        assert_eq!(first.action, TerminalOutputSequenceAction::Accept);
        assert_eq!(second.action, TerminalOutputSequenceAction::Accept);
        assert_eq!(duplicate.action, TerminalOutputSequenceAction::Duplicate);
        assert_eq!(duplicate.previous_seq, 2);
        assert_eq!(baseline.action, TerminalOutputSequenceAction::Baseline);
        assert_eq!(next.action, TerminalOutputSequenceAction::Accept);
        assert_eq!(sequencer.sequence_for("term-1"), 3);
    }

    #[test]
    fn output_sequencer_allows_host_restart_sequence_reset() {
        let mut sequencer = TerminalOutputSequencer::new();
        sequencer.observe("term-1", false, Some(8), None, false);

        let baseline = sequencer.observe("term-1", true, Some(0), Some(0), false);
        let next = sequencer.observe("term-1", false, Some(1), None, false);

        assert_eq!(baseline.action, TerminalOutputSequenceAction::Baseline);
        assert_eq!(next.action, TerminalOutputSequenceAction::Accept);
        assert_eq!(sequencer.sequence_for("term-1"), 1);
    }
}
