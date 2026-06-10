use codux_protocol::{
    REMOTE_AI_STATS, REMOTE_ERROR, REMOTE_FILE_DELETE, REMOTE_FILE_DELETED, REMOTE_FILE_LIST,
    REMOTE_FILE_READ, REMOTE_FILE_RENAME, REMOTE_FILE_RENAMED, REMOTE_FILE_WRITE,
    REMOTE_FILE_WRITTEN, REMOTE_GIT_STATUS, REMOTE_HELLO, REMOTE_HOST_INFO, REMOTE_HOST_OFFLINE,
    REMOTE_PROJECT_ADD, REMOTE_PROJECT_EDIT, REMOTE_PROJECT_LIST, REMOTE_PROJECT_REMOVE,
    REMOTE_PROJECT_SELECT, REMOTE_PROJECT_SELECTED, REMOTE_PROJECT_UPDATED,
    REMOTE_PROTOCOL_VERSION, REMOTE_RELAY_ERROR, REMOTE_RESOURCE_AI_STATS, REMOTE_RESOURCE_FILES,
    REMOTE_RESOURCE_GIT_STATUS, REMOTE_RESOURCE_PROJECTS, REMOTE_RESOURCE_SUBSCRIBE,
    REMOTE_RESOURCE_TERMINALS, REMOTE_RESOURCE_UNSUBSCRIBE, REMOTE_RESOURCE_WORKTREES,
    REMOTE_SECURE_MESSAGE, REMOTE_TERMINAL_BUFFER, REMOTE_TERMINAL_CLOSE, REMOTE_TERMINAL_CLOSED,
    REMOTE_TERMINAL_CREATE, REMOTE_TERMINAL_CREATED, REMOTE_TERMINAL_INPUT,
    REMOTE_TERMINAL_INPUT_ACK, REMOTE_TERMINAL_LIST, REMOTE_TERMINAL_OUTPUT,
    REMOTE_TERMINAL_OUTPUT_ACK, REMOTE_TERMINAL_SUBSCRIBE, REMOTE_TERMINAL_UNSUBSCRIBE,
    REMOTE_TERMINAL_UPLOAD_ACK, REMOTE_TERMINAL_UPLOAD_CHUNK, REMOTE_TERMINAL_UPLOAD_FINISH,
    REMOTE_TERMINAL_UPLOAD_START, REMOTE_TERMINAL_UPLOADED, REMOTE_TERMINAL_VIEWPORT_CLAIM,
    REMOTE_TERMINAL_VIEWPORT_RELEASE, REMOTE_TERMINAL_VIEWPORT_RESIZE,
    REMOTE_TERMINAL_VIEWPORT_STATE, REMOTE_TRANSPORT_PING, REMOTE_TRANSPORT_PONG,
    REMOTE_TRANSPORT_WEBRTC, REMOTE_TRANSPORT_WEBSOCKET_RELAY, REMOTE_WORKTREE_CREATE,
    REMOTE_WORKTREE_DELETE, REMOTE_WORKTREE_LIST, REMOTE_WORKTREE_MERGE, REMOTE_WORKTREE_SELECT,
    REMOTE_WORKTREE_UPDATED, relay_blocks_message_type,
};
use codux_remote_transport::{
    preferred_controller_transport_kind, preferred_pairing_transport_kind,
    remote_client_websocket_url, remote_pairing_ticket_url, remote_pairing_websocket_url,
    remote_server_url, remote_stun_urls,
};
use codux_terminal_core::{RemotePtySession, TerminalOutputSequencer};
use serde_json::json;
use std::ffi::{CStr, CString, c_char};
use std::ptr;

type FfiRemotePtySession = RemotePtySession<i64>;

#[unsafe(no_mangle)]
pub extern "C" fn codux_protocol_version() -> *mut c_char {
    string_to_c(REMOTE_PROTOCOL_VERSION)
}

#[unsafe(no_mangle)]
pub extern "C" fn codux_protocol_message_type(name: *const c_char) -> *mut c_char {
    let Some(name) = c_to_string(name) else {
        return ptr::null_mut();
    };
    string_to_c(message_type_by_name(&name).unwrap_or_default())
}

#[unsafe(no_mangle)]
pub extern "C" fn codux_protocol_resource_type(name: *const c_char) -> *mut c_char {
    let Some(name) = c_to_string(name) else {
        return ptr::null_mut();
    };
    string_to_c(resource_type_by_name(&name).unwrap_or_default())
}

#[unsafe(no_mangle)]
pub extern "C" fn codux_protocol_transport_kind(name: *const c_char) -> *mut c_char {
    let Some(name) = c_to_string(name) else {
        return ptr::null_mut();
    };
    string_to_c(transport_kind_by_name(&name).unwrap_or_default())
}

#[unsafe(no_mangle)]
pub extern "C" fn codux_protocol_relay_blocks_message(kind: *const c_char) -> bool {
    let Some(kind) = c_to_string(kind) else {
        return false;
    };
    relay_blocks_message_type(&kind)
}

#[unsafe(no_mangle)]
pub extern "C" fn codux_transport_server_url(base: *const c_char) -> *mut c_char {
    let Some(base) = c_to_string(base) else {
        return ptr::null_mut();
    };
    string_to_c(remote_server_url(&base))
}

#[unsafe(no_mangle)]
pub extern "C" fn codux_transport_pairing_ticket_url(
    base: *const c_char,
    ticket: *const c_char,
) -> *mut c_char {
    let Some(base) = c_to_string(base) else {
        return ptr::null_mut();
    };
    let Some(ticket) = c_to_string(ticket) else {
        return ptr::null_mut();
    };
    string_to_c(remote_pairing_ticket_url(&base, &ticket).unwrap_or_default())
}

#[unsafe(no_mangle)]
pub extern "C" fn codux_transport_pairing_websocket_url(
    base: *const c_char,
    host_id: *const c_char,
    device_public_key: *const c_char,
) -> *mut c_char {
    let Some(base) = c_to_string(base) else {
        return ptr::null_mut();
    };
    let Some(host_id) = c_to_string(host_id) else {
        return ptr::null_mut();
    };
    let Some(device_public_key) = c_to_string(device_public_key) else {
        return ptr::null_mut();
    };
    string_to_c(
        remote_pairing_websocket_url(&base, &host_id, &device_public_key).unwrap_or_default(),
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn codux_transport_client_websocket_url(
    base: *const c_char,
    host_id: *const c_char,
    device_id: *const c_char,
    token: *const c_char,
) -> *mut c_char {
    let Some(base) = c_to_string(base) else {
        return ptr::null_mut();
    };
    let Some(host_id) = c_to_string(host_id) else {
        return ptr::null_mut();
    };
    let Some(device_id) = c_to_string(device_id) else {
        return ptr::null_mut();
    };
    let token = c_to_string(token).filter(|value| !value.trim().is_empty());
    string_to_c(
        remote_client_websocket_url(&base, &host_id, &device_id, token.as_deref())
            .unwrap_or_default(),
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn codux_transport_default_ice_servers_json() -> *mut c_char {
    string_to_c(json!([{ "urls": remote_stun_urls() }]).to_string())
}

#[unsafe(no_mangle)]
pub extern "C" fn codux_transport_preferred_kind(
    transports_json: *const c_char,
    pairing: bool,
) -> *mut c_char {
    let Some(transports_json) = c_to_string(transports_json) else {
        return ptr::null_mut();
    };
    let transports = serde_json::from_str::<serde_json::Value>(&transports_json)
        .ok()
        .and_then(|value| value.as_array().cloned())
        .unwrap_or_default();
    let pairs = transports
        .iter()
        .map(|item| {
            (
                item.get("kind")
                    .or_else(|| item.get("transport"))
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default(),
                item.get("url")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default(),
            )
        })
        .collect::<Vec<_>>();
    let kind = if pairing {
        preferred_pairing_transport_kind(pairs.iter().copied())
    } else {
        preferred_controller_transport_kind(pairs.iter().copied())
    };
    string_to_c(kind)
}

#[unsafe(no_mangle)]
pub extern "C" fn codux_protocol_resource_subscribe_json(
    resource: *const c_char,
    project_id: *const c_char,
    session_id: *const c_char,
    baseline: bool,
    max_chars: i32,
    chunk_chars: i32,
) -> *mut c_char {
    let Some(resource) = c_to_string(resource) else {
        return ptr::null_mut();
    };
    let project_id = c_to_string(project_id).filter(|value| !value.trim().is_empty());
    let session_id = c_to_string(session_id).filter(|value| !value.trim().is_empty());
    let mut payload = json!({
        "resource": resource,
    });
    if let Some(project_id) = project_id {
        payload["projectId"] = json!(project_id);
    }
    if let Some(session_id) = session_id.as_deref() {
        payload["sessionId"] = json!(session_id);
    }
    if baseline {
        payload["baseline"] = json!(true);
    }
    if max_chars > 0 {
        payload["maxChars"] = json!(max_chars);
    }
    if chunk_chars > 0 {
        payload["chunkChars"] = json!(chunk_chars);
    }
    let envelope = json!({
        "type": REMOTE_RESOURCE_SUBSCRIBE,
        "sessionId": session_id,
        "payload": payload,
    });
    string_to_c(envelope.to_string())
}

#[unsafe(no_mangle)]
pub extern "C" fn codux_protocol_resource_unsubscribe_json(
    resource: *const c_char,
    project_id: *const c_char,
    session_id: *const c_char,
) -> *mut c_char {
    let Some(resource) = c_to_string(resource) else {
        return ptr::null_mut();
    };
    let project_id = c_to_string(project_id).filter(|value| !value.trim().is_empty());
    let session_id = c_to_string(session_id).filter(|value| !value.trim().is_empty());
    let mut payload = json!({
        "resource": resource,
    });
    if let Some(project_id) = project_id {
        payload["projectId"] = json!(project_id);
    }
    if let Some(session_id) = session_id.as_deref() {
        payload["sessionId"] = json!(session_id);
    }
    let envelope = json!({
        "type": REMOTE_RESOURCE_UNSUBSCRIBE,
        "sessionId": session_id,
        "payload": payload,
    });
    string_to_c(envelope.to_string())
}

#[unsafe(no_mangle)]
pub extern "C" fn codux_protocol_string_free(value: *mut c_char) {
    if value.is_null() {
        return;
    }
    unsafe {
        drop(CString::from_raw(value));
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn codux_terminal_session_new(
    session_id: *const c_char,
    max_cached_chars: i64,
) -> *mut FfiRemotePtySession {
    let Some(session_id) = c_to_string(session_id) else {
        return ptr::null_mut();
    };
    let max_cached_chars = usize::try_from(max_cached_chars.max(0)).unwrap_or(0);
    Box::into_raw(Box::new(RemotePtySession::new(
        session_id,
        max_cached_chars,
    )))
}

#[unsafe(no_mangle)]
pub extern "C" fn codux_terminal_session_free(session: *mut FfiRemotePtySession) {
    if session.is_null() {
        return;
    }
    unsafe {
        drop(Box::from_raw(session));
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn codux_terminal_session_snapshot_json(
    session: *const FfiRemotePtySession,
) -> *mut c_char {
    let Some(session) = terminal_session_ref(session) else {
        return ptr::null_mut();
    };
    let snapshot = session.snapshot();
    string_to_c(
        json!({
            "sessionId": snapshot.session_id,
            "content": snapshot.content,
            "bufferLength": snapshot.buffer_length,
            "sequence": snapshot.sequence,
        })
        .to_string(),
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn codux_terminal_session_content(
    session: *const FfiRemotePtySession,
) -> *mut c_char {
    let Some(session) = terminal_session_ref(session) else {
        return ptr::null_mut();
    };
    string_to_c(session.content())
}

#[unsafe(no_mangle)]
pub extern "C" fn codux_terminal_session_buffer_length(session: *const FfiRemotePtySession) -> i64 {
    terminal_session_ref(session)
        .map(|session| i64::try_from(session.buffer_length()).unwrap_or(i64::MAX))
        .unwrap_or(0)
}

#[unsafe(no_mangle)]
pub extern "C" fn codux_terminal_session_sequence(session: *const FfiRemotePtySession) -> i64 {
    terminal_session_ref(session)
        .map(|session| session.sequence())
        .unwrap_or(0)
}

#[unsafe(no_mangle)]
pub extern "C" fn codux_terminal_session_is_restoring_baseline(
    session: *const FfiRemotePtySession,
) -> bool {
    terminal_session_ref(session)
        .map(|session| session.is_restoring_baseline())
        .unwrap_or(false)
}

#[unsafe(no_mangle)]
pub extern "C" fn codux_terminal_session_is_restoring_snapshot(
    session: *const FfiRemotePtySession,
) -> bool {
    codux_terminal_session_is_restoring_baseline(session)
}

#[unsafe(no_mangle)]
pub extern "C" fn codux_terminal_session_require_baseline(session: *mut FfiRemotePtySession) {
    if let Some(session) = terminal_session_mut(session) {
        session.require_baseline();
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn codux_terminal_session_require_snapshot(session: *mut FfiRemotePtySession) {
    codux_terminal_session_require_baseline(session);
}

#[unsafe(no_mangle)]
pub extern "C" fn codux_terminal_session_reset_transient(
    session: *mut FfiRemotePtySession,
    reset_sequence: bool,
) {
    if let Some(session) = terminal_session_mut(session) {
        session.reset_transient(reset_sequence);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn codux_terminal_session_set_sequence(
    session: *mut FfiRemotePtySession,
    sequence: i64,
) {
    if let Some(session) = terminal_session_mut(session) {
        session.set_sequence(sequence);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn codux_terminal_session_hold_live_token(
    session: *mut FfiRemotePtySession,
    sequence: i64,
    token: i64,
) -> bool {
    let Some(session) = terminal_session_mut(session) else {
        return false;
    };
    session.hold_live(optional_sequence(sequence), token)
}

#[unsafe(no_mangle)]
pub extern "C" fn codux_terminal_session_accept_baseline_page_json(
    session: *mut FfiRemotePtySession,
    data: *const c_char,
    offset: i64,
    buffer_length: i64,
    truncated: bool,
) -> *mut c_char {
    let Some(session) = terminal_session_mut(session) else {
        return ptr::null_mut();
    };
    let Some(data) = c_to_string(data) else {
        return ptr::null_mut();
    };
    let offset = usize::try_from(offset.max(0)).unwrap_or(0);
    let buffer_length = optional_usize(buffer_length);
    let page = session.accept_baseline_page(&data, offset, buffer_length, truncated);
    string_to_c(
        json!({
            "accepted": page.accepted,
            "duplicate": page.duplicate,
            "ready": page.ready,
            "data": page.data,
            "nextOffset": page.next_offset,
            "progress": page.progress,
        })
        .to_string(),
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn codux_terminal_session_accept_snapshot_page_json(
    session: *mut FfiRemotePtySession,
    data: *const c_char,
    offset: i64,
    buffer_length: i64,
    truncated: bool,
) -> *mut c_char {
    codux_terminal_session_accept_baseline_page_json(
        session,
        data,
        offset,
        buffer_length,
        truncated,
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn codux_terminal_session_replace_from_baseline(
    session: *mut FfiRemotePtySession,
    content: *const c_char,
    buffer_length: i64,
    sequence: i64,
) {
    let Some(session) = terminal_session_mut(session) else {
        return;
    };
    let Some(content) = c_to_string(content) else {
        return;
    };
    session.replace_from_baseline(
        &content,
        optional_usize(buffer_length),
        optional_sequence(sequence),
    );
}

#[unsafe(no_mangle)]
pub extern "C" fn codux_terminal_session_replace_from_snapshot(
    session: *mut FfiRemotePtySession,
    content: *const c_char,
    buffer_length: i64,
    sequence: i64,
) {
    codux_terminal_session_replace_from_baseline(session, content, buffer_length, sequence);
}

#[unsafe(no_mangle)]
pub extern "C" fn codux_terminal_session_replace_from_baseline_json(
    session: *mut FfiRemotePtySession,
    content: *const c_char,
    buffer_length: i64,
    sequence: i64,
) -> *mut c_char {
    let Some(session) = terminal_session_mut(session) else {
        return ptr::null_mut();
    };
    let Some(content) = c_to_string(content) else {
        return ptr::null_mut();
    };
    let replay_tokens = session.replace_from_baseline(
        &content,
        optional_usize(buffer_length),
        optional_sequence(sequence),
    );
    string_to_c(
        json!({
            "replayTokens": replay_tokens,
        })
        .to_string(),
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn codux_terminal_session_replace_from_snapshot_json(
    session: *mut FfiRemotePtySession,
    content: *const c_char,
    buffer_length: i64,
    sequence: i64,
) -> *mut c_char {
    codux_terminal_session_replace_from_baseline_json(session, content, buffer_length, sequence)
}

#[unsafe(no_mangle)]
pub extern "C" fn codux_terminal_session_append_live(
    session: *mut FfiRemotePtySession,
    data: *const c_char,
    buffer_length: i64,
    sequence: i64,
) {
    let Some(session) = terminal_session_mut(session) else {
        return;
    };
    let Some(data) = c_to_string(data) else {
        return;
    };
    session.append_live(
        &data,
        optional_usize(buffer_length),
        optional_sequence(sequence),
    );
}

#[unsafe(no_mangle)]
pub extern "C" fn codux_terminal_session_clear(session: *mut FfiRemotePtySession) {
    if let Some(session) = terminal_session_mut(session) {
        session.clear();
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn codux_terminal_output_sequencer_new() -> *mut TerminalOutputSequencer {
    Box::into_raw(Box::new(TerminalOutputSequencer::new()))
}

#[unsafe(no_mangle)]
pub extern "C" fn codux_terminal_output_sequencer_free(sequencer: *mut TerminalOutputSequencer) {
    if sequencer.is_null() {
        return;
    }
    unsafe {
        drop(Box::from_raw(sequencer));
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn codux_terminal_output_sequencer_sequence_for(
    sequencer: *const TerminalOutputSequencer,
    session_id: *const c_char,
) -> i64 {
    let Some(sequencer) = terminal_output_sequencer_ref(sequencer) else {
        return 0;
    };
    let Some(session_id) = c_to_string(session_id) else {
        return 0;
    };
    sequencer.sequence_for(&session_id)
}

#[unsafe(no_mangle)]
pub extern "C" fn codux_terminal_output_sequencer_is_resyncing(
    sequencer: *const TerminalOutputSequencer,
    session_id: *const c_char,
) -> bool {
    let Some(sequencer) = terminal_output_sequencer_ref(sequencer) else {
        return false;
    };
    let Some(session_id) = c_to_string(session_id) else {
        return false;
    };
    sequencer.is_resyncing(&session_id)
}

#[unsafe(no_mangle)]
pub extern "C" fn codux_terminal_output_sequencer_observe_json(
    sequencer: *mut TerminalOutputSequencer,
    session_id: *const c_char,
    is_buffer: bool,
    output_seq: i64,
    offset: i64,
    resets_sequence: bool,
) -> *mut c_char {
    let Some(sequencer) = terminal_output_sequencer_mut(sequencer) else {
        return ptr::null_mut();
    };
    let Some(session_id) = c_to_string(session_id) else {
        return ptr::null_mut();
    };
    let result = sequencer.observe(
        &session_id,
        is_buffer,
        optional_sequence(output_seq),
        optional_usize(offset),
        resets_sequence,
    );
    string_to_c(
        json!({
            "action": result.action.as_str(),
            "previousSeq": result.previous_seq,
            "shouldRender": result.should_render(),
        })
        .to_string(),
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn codux_terminal_output_sequencer_remove(
    sequencer: *mut TerminalOutputSequencer,
    session_id: *const c_char,
) {
    let Some(sequencer) = terminal_output_sequencer_mut(sequencer) else {
        return;
    };
    let Some(session_id) = c_to_string(session_id) else {
        return;
    };
    sequencer.remove(&session_id);
}

#[unsafe(no_mangle)]
pub extern "C" fn codux_terminal_output_sequencer_reset(sequencer: *mut TerminalOutputSequencer) {
    if let Some(sequencer) = terminal_output_sequencer_mut(sequencer) {
        sequencer.reset();
    }
}

fn c_to_string(value: *const c_char) -> Option<String> {
    if value.is_null() {
        return None;
    }
    unsafe { CStr::from_ptr(value).to_str().ok().map(str::to_string) }
}

fn string_to_c(value: impl Into<String>) -> *mut c_char {
    CString::new(value.into())
        .map(CString::into_raw)
        .unwrap_or(ptr::null_mut())
}

fn optional_usize(value: i64) -> Option<usize> {
    if value < 0 {
        None
    } else {
        usize::try_from(value).ok()
    }
}

fn optional_sequence(value: i64) -> Option<i64> {
    if value < 0 { None } else { Some(value) }
}

fn terminal_session_ref<'a>(
    session: *const FfiRemotePtySession,
) -> Option<&'a FfiRemotePtySession> {
    if session.is_null() {
        return None;
    }
    unsafe { session.as_ref() }
}

fn terminal_session_mut<'a>(
    session: *mut FfiRemotePtySession,
) -> Option<&'a mut FfiRemotePtySession> {
    if session.is_null() {
        return None;
    }
    unsafe { session.as_mut() }
}

fn terminal_output_sequencer_ref<'a>(
    sequencer: *const TerminalOutputSequencer,
) -> Option<&'a TerminalOutputSequencer> {
    if sequencer.is_null() {
        return None;
    }
    unsafe { sequencer.as_ref() }
}

fn terminal_output_sequencer_mut<'a>(
    sequencer: *mut TerminalOutputSequencer,
) -> Option<&'a mut TerminalOutputSequencer> {
    if sequencer.is_null() {
        return None;
    }
    unsafe { sequencer.as_mut() }
}

fn message_type_by_name(name: &str) -> Option<&'static str> {
    Some(match name {
        "hello" => REMOTE_HELLO,
        "error" => REMOTE_ERROR,
        "relayError" => REMOTE_RELAY_ERROR,
        "secureMessage" => REMOTE_SECURE_MESSAGE,
        "hostInfo" => REMOTE_HOST_INFO,
        "hostOffline" => REMOTE_HOST_OFFLINE,
        "transportPing" => REMOTE_TRANSPORT_PING,
        "transportPong" => REMOTE_TRANSPORT_PONG,
        "resourceSubscribe" => REMOTE_RESOURCE_SUBSCRIBE,
        "resourceUnsubscribe" => REMOTE_RESOURCE_UNSUBSCRIBE,
        "projectList" => REMOTE_PROJECT_LIST,
        "projectSelect" => REMOTE_PROJECT_SELECT,
        "projectSelected" => REMOTE_PROJECT_SELECTED,
        "projectAdd" => REMOTE_PROJECT_ADD,
        "projectEdit" => REMOTE_PROJECT_EDIT,
        "projectRemove" => REMOTE_PROJECT_REMOVE,
        "projectUpdated" => REMOTE_PROJECT_UPDATED,
        "worktreeList" => REMOTE_WORKTREE_LIST,
        "worktreeSelect" => REMOTE_WORKTREE_SELECT,
        "worktreeCreate" => REMOTE_WORKTREE_CREATE,
        "worktreeMerge" => REMOTE_WORKTREE_MERGE,
        "worktreeDelete" => REMOTE_WORKTREE_DELETE,
        "worktreeUpdated" => REMOTE_WORKTREE_UPDATED,
        "terminalList" => REMOTE_TERMINAL_LIST,
        "terminalSubscribe" => REMOTE_TERMINAL_SUBSCRIBE,
        "terminalUnsubscribe" => REMOTE_TERMINAL_UNSUBSCRIBE,
        "terminalCreate" => REMOTE_TERMINAL_CREATE,
        "terminalCreated" => REMOTE_TERMINAL_CREATED,
        "terminalClose" => REMOTE_TERMINAL_CLOSE,
        "terminalClosed" => REMOTE_TERMINAL_CLOSED,
        "terminalBuffer" => REMOTE_TERMINAL_BUFFER,
        "terminalOutput" => REMOTE_TERMINAL_OUTPUT,
        "terminalOutputAck" => REMOTE_TERMINAL_OUTPUT_ACK,
        "terminalInput" => REMOTE_TERMINAL_INPUT,
        "terminalInputAck" => REMOTE_TERMINAL_INPUT_ACK,
        "terminalViewportClaim" => REMOTE_TERMINAL_VIEWPORT_CLAIM,
        "terminalViewportResize" => REMOTE_TERMINAL_VIEWPORT_RESIZE,
        "terminalViewportRelease" => REMOTE_TERMINAL_VIEWPORT_RELEASE,
        "terminalViewportState" => REMOTE_TERMINAL_VIEWPORT_STATE,
        "terminalUploadStart" => REMOTE_TERMINAL_UPLOAD_START,
        "terminalUploadChunk" => REMOTE_TERMINAL_UPLOAD_CHUNK,
        "terminalUploadFinish" => REMOTE_TERMINAL_UPLOAD_FINISH,
        "terminalUploadAck" => REMOTE_TERMINAL_UPLOAD_ACK,
        "terminalUploaded" => REMOTE_TERMINAL_UPLOADED,
        "fileList" => REMOTE_FILE_LIST,
        "fileRead" => REMOTE_FILE_READ,
        "fileWrite" => REMOTE_FILE_WRITE,
        "fileWritten" => REMOTE_FILE_WRITTEN,
        "fileRename" => REMOTE_FILE_RENAME,
        "fileRenamed" => REMOTE_FILE_RENAMED,
        "fileDelete" => REMOTE_FILE_DELETE,
        "fileDeleted" => REMOTE_FILE_DELETED,
        "gitStatus" => REMOTE_GIT_STATUS,
        "aiStats" => REMOTE_AI_STATS,
        _ => return None,
    })
}

fn resource_type_by_name(name: &str) -> Option<&'static str> {
    Some(match name {
        "projects" => REMOTE_RESOURCE_PROJECTS,
        "terminals" => REMOTE_RESOURCE_TERMINALS,
        "worktrees" => REMOTE_RESOURCE_WORKTREES,
        "gitStatus" => REMOTE_RESOURCE_GIT_STATUS,
        "aiStats" => REMOTE_RESOURCE_AI_STATS,
        "files" => REMOTE_RESOURCE_FILES,
        _ => return None,
    })
}

fn transport_kind_by_name(name: &str) -> Option<&'static str> {
    Some(match name {
        "websocketRelay" => REMOTE_TRANSPORT_WEBSOCKET_RELAY,
        "webRtc" => REMOTE_TRANSPORT_WEBRTC,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_protocol_names() {
        assert_eq!(
            message_type_by_name("terminalOutput"),
            Some("terminal.output")
        );
        assert_eq!(resource_type_by_name("gitStatus"), Some("git.status"));
        assert_eq!(
            transport_kind_by_name("websocketRelay"),
            Some("websocketRelay")
        );
        assert_eq!(transport_kind_by_name("webRtc"), Some("webRtc"));
    }
}
