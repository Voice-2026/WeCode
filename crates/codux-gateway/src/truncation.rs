//! In-memory truncation recovery. When Kiro truncates a tool call or content
//! mid-stream, we record a stable identifier; on the next request we inject a
//! synthetic notice so the model can adapt. Port of truncation_state.py +
//! truncation_recovery.py.

use std::collections::HashSet;
use std::sync::Mutex;

use sha2::{Digest, Sha256};

pub const TOOL_NOTICE: &str = "[API Limitation] Your tool call was truncated by the upstream API due to output size limits.\n\nIf the tool result below shows an error or unexpected behavior, this is likely a CONSEQUENCE of the truncation, not the root cause. The tool call itself was cut off before it could be fully transmitted.\n\nRepeating the exact same operation will be truncated again. Consider adapting your approach.";

pub const CONTENT_NOTICE: &str = "[System Notice] Your previous response was truncated by the API due to output size limitations. This is not an error on your part. If you need to continue, please adapt your approach rather than repeating the same output.";

#[derive(Default)]
pub struct TruncationStore {
    tools: Mutex<HashSet<String>>,
    contents: Mutex<HashSet<String>>,
}

/// Hash of the first 500 chars, matching the Python content identifier.
pub fn content_hash(content: &str) -> String {
    let prefix: String = content.chars().take(500).collect();
    let mut hasher = Sha256::new();
    hasher.update(prefix.as_bytes());
    let digest = hasher.finalize();
    let mut s = String::new();
    for b in &digest[..8] {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

impl TruncationStore {
    pub fn save_tool(&self, tool_call_id: &str) {
        if let Ok(mut t) = self.tools.lock() {
            t.insert(tool_call_id.to_string());
        }
    }

    pub fn save_content(&self, content: &str) {
        if let Ok(mut c) = self.contents.lock() {
            c.insert(content_hash(content));
        }
    }

    /// One-time check: returns true and removes the entry if present.
    pub fn take_tool(&self, tool_call_id: &str) -> bool {
        self.tools
            .lock()
            .map(|mut t| t.remove(tool_call_id))
            .unwrap_or(false)
    }

    pub fn take_content(&self, content: &str) -> bool {
        let h = content_hash(content);
        self.contents
            .lock()
            .map(|mut c| c.remove(&h))
            .unwrap_or(false)
    }

    pub fn is_empty(&self) -> bool {
        let t = self.tools.lock().map(|t| t.is_empty()).unwrap_or(true);
        let c = self.contents.lock().map(|c| c.is_empty()).unwrap_or(true);
        t && c
    }
}

/// If any assistant message in the request references a recorded truncation,
/// append a synthetic user message with the appropriate notice(s) so the model
/// is informed. Works for both OpenAI and Anthropic message arrays. Returns true
/// if a notice was injected.
///
/// This is a simplification of the Python behavior (which injects per-tool
/// `is_error` tool_results); a single trailing user notice achieves the same
/// user-facing goal of telling the model its prior output was cut off.
pub fn inject_notices(request: &mut serde_json::Value, store: &TruncationStore) -> bool {
    use serde_json::Value;
    if store.is_empty() {
        return false;
    }
    let Some(messages) = request.get("messages").and_then(Value::as_array) else {
        return false;
    };

    let mut tool_hit = false;
    let mut content_hit = false;
    for msg in messages {
        if msg.get("role").and_then(Value::as_str) != Some("assistant") {
            continue;
        }
        // OpenAI tool_calls.
        if let Some(tcs) = msg.get("tool_calls").and_then(Value::as_array) {
            for tc in tcs {
                if let Some(id) = tc.get("id").and_then(Value::as_str) {
                    if store.take_tool(id) {
                        tool_hit = true;
                    }
                }
            }
        }
        // Anthropic tool_use blocks + text content.
        let mut assistant_text = String::new();
        match msg.get("content") {
            Some(Value::String(s)) => assistant_text.push_str(s),
            Some(Value::Array(blocks)) => {
                for b in blocks {
                    match b.get("type").and_then(Value::as_str) {
                        Some("tool_use") => {
                            if let Some(id) = b.get("id").and_then(Value::as_str) {
                                if store.take_tool(id) {
                                    tool_hit = true;
                                }
                            }
                        }
                        Some("text") => {
                            if let Some(t) = b.get("text").and_then(Value::as_str) {
                                assistant_text.push_str(t);
                            }
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
        if !assistant_text.is_empty() && store.take_content(&assistant_text) {
            content_hit = true;
        }
    }

    if !tool_hit && !content_hit {
        return false;
    }

    let mut notice = String::new();
    if tool_hit {
        notice.push_str(TOOL_NOTICE);
    }
    if content_hit {
        if !notice.is_empty() {
            notice.push_str("\n\n");
        }
        notice.push_str(CONTENT_NOTICE);
    }

    if let Some(arr) = request.get_mut("messages").and_then(Value::as_array_mut) {
        arr.push(serde_json::json!({ "role": "user", "content": notice }));
    }
    true
}
