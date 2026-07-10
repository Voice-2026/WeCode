//! Approximate token counting. The Python gateway uses tiktoken cl100k_base ×1.15;
//! we ship a chars/4 ×1.15 heuristic (tiktoken-rs can be added behind a feature later
//! for closer parity — count_tokens is inherently approximate).

use serde_json::Value;

pub const CLAUDE_CORRECTION_FACTOR: f64 = 1.15;

pub fn count_tokens(text: &str, apply_correction: bool) -> u64 {
    if text.is_empty() {
        return 0;
    }
    let base = base_token_count(text) as f64;
    if apply_correction {
        (base * CLAUDE_CORRECTION_FACTOR) as u64
    } else {
        base as u64
    }
}

#[cfg(feature = "tiktoken")]
fn base_token_count(text: &str) -> usize {
    use std::sync::OnceLock;
    use tiktoken_rs::CoreBPE;
    static BPE: OnceLock<Option<CoreBPE>> = OnceLock::new();
    let bpe = BPE.get_or_init(|| tiktoken_rs::cl100k_base().ok());
    match bpe {
        Some(bpe) => bpe.encode_with_special_tokens(text).len(),
        None => text.chars().count() / 4 + 1,
    }
}

#[cfg(not(feature = "tiktoken"))]
fn base_token_count(text: &str) -> usize {
    text.chars().count() / 4 + 1
}

/// Estimate input tokens for a request (messages + tools + system), matching the
/// Python `estimate_request_tokens` structure closely enough for Claude Code's
/// compaction heuristic.
pub fn estimate_request_tokens(
    messages: &[Value],
    tools: Option<&[Value]>,
    system: Option<&Value>,
    apply_correction: bool,
) -> u64 {
    let mut total = count_message_tokens(messages);
    if let Some(tools) = tools {
        total += count_tools_tokens(tools);
    }
    if let Some(system) = system {
        total += count_system_tokens(system);
    }
    if apply_correction {
        (total as f64 * CLAUDE_CORRECTION_FACTOR) as u64
    } else {
        total
    }
}

fn count_message_tokens(messages: &[Value]) -> u64 {
    let mut total = 0u64;
    for msg in messages {
        total += 4;
        if let Some(role) = msg.get("role").and_then(Value::as_str) {
            total += count_tokens(role, false);
        }
        match msg.get("content") {
            Some(Value::String(s)) => total += count_tokens(s, false),
            Some(Value::Array(items)) => {
                for item in items {
                    total += count_content_block(item);
                }
            }
            _ => {}
        }
        if let Some(tcs) = msg.get("tool_calls").and_then(Value::as_array) {
            for tc in tcs {
                total += 4;
                if let Some(f) = tc.get("function") {
                    total +=
                        count_tokens(f.get("name").and_then(Value::as_str).unwrap_or(""), false);
                    total += count_tokens(
                        f.get("arguments").and_then(Value::as_str).unwrap_or(""),
                        false,
                    );
                }
            }
        }
        if let Some(id) = msg.get("tool_call_id").and_then(Value::as_str) {
            total += count_tokens(id, false);
        }
    }
    total + 3
}

fn count_content_block(item: &Value) -> u64 {
    let ty = item.get("type").and_then(Value::as_str).unwrap_or("");
    match ty {
        "text" => count_tokens(
            item.get("text").and_then(Value::as_str).unwrap_or(""),
            false,
        ),
        "image_url" | "image" => 100,
        "tool_use" => {
            let mut t = count_tokens(item.get("id").and_then(Value::as_str).unwrap_or(""), false);
            t += count_tokens(
                item.get("name").and_then(Value::as_str).unwrap_or(""),
                false,
            );
            if let Some(input) = item.get("input") {
                t += count_tokens(&input.to_string(), false);
            }
            t
        }
        "tool_result" => {
            let mut t = count_tokens(
                item.get("tool_use_id")
                    .and_then(Value::as_str)
                    .unwrap_or(""),
                false,
            );
            match item.get("content") {
                Some(Value::String(s)) => t += count_tokens(s, false),
                Some(Value::Array(items)) => {
                    for b in items {
                        t += count_content_block(b);
                    }
                }
                _ => {}
            }
            t
        }
        _ => count_tokens(&item.to_string(), false),
    }
}

fn count_tools_tokens(tools: &[Value]) -> u64 {
    let mut total = 0u64;
    for tool in tools {
        total += 4;
        let payload = if tool.get("type").and_then(Value::as_str) == Some("function") {
            tool.get("function").unwrap_or(tool)
        } else {
            tool
        };
        total += count_tokens(
            payload.get("name").and_then(Value::as_str).unwrap_or(""),
            false,
        );
        total += count_tokens(
            payload
                .get("description")
                .and_then(Value::as_str)
                .unwrap_or(""),
            false,
        );
        let params = payload
            .get("input_schema")
            .or_else(|| payload.get("parameters"));
        if let Some(p) = params {
            total += count_tokens(&p.to_string(), false);
        }
    }
    total
}

fn count_system_tokens(system: &Value) -> u64 {
    match system {
        Value::String(s) => count_tokens(s, false),
        Value::Array(blocks) => blocks
            .iter()
            .map(|b| count_tokens(b.get("text").and_then(Value::as_str).unwrap_or(""), false))
            .sum(),
        _ => 0,
    }
}
