//! FFI surface for `RemoteTerminalOutputRouter`. The mobile app drives the
//! terminal output orchestration + render-path screen ops through this single
//! opaque handle instead of re-implementing the state machine in Dart.

use crate::common::{c_to_string, string_to_c};
use serde_json::Value;
use std::ffi::c_char;
use std::ptr;
use wecode_terminal_core::RemoteTerminalOutputRouter;

pub type FfiOutputRouter = RemoteTerminalOutputRouter;

fn router_ref<'a>(router: *const FfiOutputRouter) -> Option<&'a FfiOutputRouter> {
    if router.is_null() {
        return None;
    }
    unsafe { router.as_ref() }
}

fn router_mut<'a>(router: *mut FfiOutputRouter) -> Option<&'a mut FfiOutputRouter> {
    if router.is_null() {
        return None;
    }
    unsafe { router.as_mut() }
}

#[unsafe(no_mangle)]
pub extern "C" fn wecode_output_router_new(
    max_buffer_chars: i64,
    max_cached_chars: i64,
) -> *mut FfiOutputRouter {
    let max_buffer = usize::try_from(max_buffer_chars).unwrap_or(200_000);
    let max_cached = usize::try_from(max_cached_chars).unwrap_or(2_000_000);
    Box::into_raw(Box::new(RemoteTerminalOutputRouter::new(
        max_buffer, max_cached,
    )))
}

#[unsafe(no_mangle)]
pub extern "C" fn wecode_output_router_free(router: *mut FfiOutputRouter) {
    if router.is_null() {
        return;
    }
    unsafe {
        drop(Box::from_raw(router));
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn wecode_output_router_accept(
    router: *mut FfiOutputRouter,
    message_json: *const c_char,
    active_session_id: *const c_char,
) -> *mut c_char {
    let Some(router) = router_mut(router) else {
        return string_to_c("[]");
    };
    let Some(message_json) = c_to_string(message_json) else {
        return string_to_c("[]");
    };
    let Ok(message) = serde_json::from_str::<Value>(&message_json) else {
        return string_to_c("[]");
    };
    let active = c_to_string(active_session_id);
    let effects = router.accept(&message, active.as_deref());
    let array: Vec<Value> = effects.iter().map(|effect| effect.to_json()).collect();
    string_to_c(Value::Array(array).to_string())
}

#[unsafe(no_mangle)]
pub extern "C" fn wecode_output_router_bind_session(
    router: *mut FfiOutputRouter,
    session_id: *const c_char,
    require_baseline: bool,
) {
    if let (Some(router), Some(session_id)) = (router_mut(router), c_to_string(session_id)) {
        router.bind_session(&session_id, require_baseline);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn wecode_output_router_remove_session(
    router: *mut FfiOutputRouter,
    session_id: *const c_char,
) {
    if let (Some(router), Some(session_id)) = (router_mut(router), c_to_string(session_id)) {
        router.remove_session(&session_id);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn wecode_output_router_start_buffer_request(
    router: *mut FfiOutputRouter,
    session_id: *const c_char,
    request_id: *const c_char,
    require_baseline: bool,
    reset_assembler: bool,
    replace_active: bool,
) -> bool {
    let (Some(router), Some(session_id), Some(request_id)) = (
        router_mut(router),
        c_to_string(session_id),
        c_to_string(request_id),
    ) else {
        return false;
    };
    router.start_buffer_request(
        &session_id,
        &request_id,
        require_baseline,
        reset_assembler,
        replace_active,
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn wecode_output_router_evict_inactive(
    router: *mut FfiOutputRouter,
    active_session_id: *const c_char,
    max_sessions: i64,
) -> *mut c_char {
    let (Some(router), Some(active)) = (router_mut(router), c_to_string(active_session_id)) else {
        return string_to_c("[]");
    };
    let max = usize::try_from(max_sessions).unwrap_or(8);
    let evicted = router.evict_inactive_sessions(&active, max);
    string_to_c(Value::Array(evicted.into_iter().map(Value::String).collect()).to_string())
}

#[unsafe(no_mangle)]
pub extern "C" fn wecode_output_router_reset_transient(router: *mut FfiOutputRouter) {
    if let Some(router) = router_mut(router) {
        router.reset_transient();
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn wecode_output_router_reset_session_transient(
    router: *mut FfiOutputRouter,
    session_id: *const c_char,
    reset_sequence: bool,
) {
    if let (Some(router), Some(session_id)) = (router_mut(router), c_to_string(session_id)) {
        router.reset_session_transient(&session_id, reset_sequence);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn wecode_output_router_reset_all(router: *mut FfiOutputRouter) {
    if let Some(router) = router_mut(router) {
        router.reset_all();
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn wecode_output_router_content(
    router: *const FfiOutputRouter,
    session_id: *const c_char,
) -> *mut c_char {
    let (Some(router), Some(session_id)) = (router_ref(router), c_to_string(session_id)) else {
        return ptr::null_mut();
    };
    match router.content(&session_id) {
        Some(content) => string_to_c(content),
        None => ptr::null_mut(),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn wecode_output_router_has_cached_output(
    router: *const FfiOutputRouter,
    session_id: *const c_char,
) -> bool {
    match (router_ref(router), c_to_string(session_id)) {
        (Some(router), Some(session_id)) => router.has_cached_output(&session_id),
        _ => false,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn wecode_output_router_buffer_offset(
    router: *const FfiOutputRouter,
    session_id: *const c_char,
) -> i64 {
    match (router_ref(router), c_to_string(session_id)) {
        (Some(router), Some(session_id)) => router.buffer_offset(&session_id) as i64,
        _ => 0,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn wecode_output_router_has_sequence_gap(
    router: *const FfiOutputRouter,
    session_id: *const c_char,
) -> bool {
    match (router_ref(router), c_to_string(session_id)) {
        (Some(router), Some(session_id)) => router.has_sequence_gap(&session_id),
        _ => false,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn wecode_output_router_output_sequence(
    router: *const FfiOutputRouter,
    session_id: *const c_char,
) -> i64 {
    match (router_ref(router), c_to_string(session_id)) {
        (Some(router), Some(session_id)) => router.output_sequence(&session_id),
        _ => 0,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn wecode_output_router_active_buffer_request_id(
    router: *const FfiOutputRouter,
    session_id: *const c_char,
) -> *mut c_char {
    let (Some(router), Some(session_id)) = (router_ref(router), c_to_string(session_id)) else {
        return ptr::null_mut();
    };
    match router.active_buffer_request_id(&session_id) {
        Some(request_id) => string_to_c(request_id),
        None => ptr::null_mut(),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn wecode_output_router_has_active_buffer_request(
    router: *const FfiOutputRouter,
    session_id: *const c_char,
) -> bool {
    match (router_ref(router), c_to_string(session_id)) {
        (Some(router), Some(session_id)) => router.has_active_buffer_request(&session_id),
        _ => false,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn wecode_output_router_render_generation(
    router: *const FfiOutputRouter,
    session_id: *const c_char,
) -> i64 {
    match (router_ref(router), c_to_string(session_id)) {
        (Some(router), Some(session_id)) => router.render_generation(&session_id) as i64,
        _ => 0,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn wecode_output_router_screen_snapshot_json(
    router: *const FfiOutputRouter,
    session_id: *const c_char,
) -> *mut c_char {
    let (Some(router), Some(session_id)) = (router_ref(router), c_to_string(session_id)) else {
        return ptr::null_mut();
    };
    match router.screen_snapshot_json(&session_id) {
        Some(json) => string_to_c(json),
        None => ptr::null_mut(),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn wecode_output_router_resize_screen(
    router: *mut FfiOutputRouter,
    session_id: *const c_char,
    cols: i64,
    rows: i64,
) {
    if let (Some(router), Some(session_id)) = (router_mut(router), c_to_string(session_id)) {
        let cols = usize::try_from(cols).unwrap_or(0);
        let rows = usize::try_from(rows).unwrap_or(0);
        router.resize_screen(&session_id, cols, rows);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn wecode_output_router_scroll_screen_pixels(
    router: *mut FfiOutputRouter,
    session_id: *const c_char,
    pixels: f64,
    cell_height: f64,
) {
    if let (Some(router), Some(session_id)) = (router_mut(router), c_to_string(session_id)) {
        router.scroll_screen_pixels(&session_id, pixels, cell_height);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn wecode_output_router_settle_screen_pixel_scroll(
    router: *mut FfiOutputRouter,
    session_id: *const c_char,
) {
    if let (Some(router), Some(session_id)) = (router_mut(router), c_to_string(session_id)) {
        router.settle_screen_pixel_scroll(&session_id);
    }
}
