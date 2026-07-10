use async_stream::stream;
use axum::body::Body;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use futures::StreamExt;
use serde_json::{json, Value};

use super::{tokens_from_context_usage, verify_api_key, AppState};
use crate::convert::anthropic::anthropic_to_kiro;
use crate::tokens::{count_tokens as count_text_tokens, estimate_request_tokens};
use crate::upstream::{collect_stream, kiro_event_stream, KiroEvent};
use crate::util::{generate_message_id, generate_tool_use_id};

fn sse_event(event_type: &str, data: &Value) -> String {
    format!(
        "event: {event_type}\ndata: {}\n\n",
        serde_json::to_string(data).unwrap_or_default()
    )
}

fn conversation_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

pub async fn messages(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Json<Value>,
) -> Response {
    if let Err(resp) = verify_api_key(&headers, &state.config) {
        return resp;
    }
    let mut request = body.0;
    if state.config.truncation_recovery {
        crate::truncation::inject_notices(&mut request, &state.truncation);
    }
    if state.config.web_search_enabled {
        crate::mcp::inject_web_search_tool(&mut request, crate::mcp::web_search_tool_anthropic());
    }
    let stream_requested = request
        .get("stream")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let model = request
        .get("model")
        .and_then(Value::as_str)
        .unwrap_or("auto")
        .to_string();

    let conv_id = conversation_id();
    let (auth, resp) = match state
        .accounts
        .request_with_failover(
            |profile_arn| {
                anthropic_to_kiro(
                    &request,
                    conv_id.clone(),
                    profile_arn.unwrap_or_default(),
                    &state.config,
                )
            },
            &state.config,
        )
        .await
    {
        Ok(r) => r,
        Err(e) => return (e.status_code(), Json(e.to_anthropic_json())).into_response(),
    };

    // Fallback input-token estimate (streaming spec needs it up front).
    let input_estimate = estimate_input_tokens(&request);
    let max_input_tokens = state.config.default_max_input_tokens;
    let ft = state.config.first_token_timeout_secs;
    let rt = state.config.streaming_read_timeout_secs;
    let thinking = thinking_handling(&state.config);

    let recovery = state.config.truncation_recovery;
    let web_search = state.config.web_search_enabled;
    let store = state.truncation.clone();

    if stream_requested {
        let body = anthropic_sse_body(
            resp,
            model,
            input_estimate,
            ft,
            rt,
            thinking,
            store,
            recovery,
            auth,
            web_search,
        );
        Response::builder()
            .header("content-type", "text/event-stream")
            .header("cache-control", "no-cache")
            .body(body)
            .unwrap()
    } else {
        match collect_stream(resp, ft, rt, thinking.clone()).await {
            Ok(mut result) => {
                if recovery {
                    super::save_truncations(&result, &store);
                }
                if web_search {
                    run_web_search(&mut result, &auth).await;
                }
                let msg = build_anthropic_message(
                    &model,
                    &result,
                    input_estimate,
                    max_input_tokens,
                    &thinking,
                );
                Json(msg).into_response()
            }
            Err(e) => (e.status_code(), Json(e.to_anthropic_json())).into_response(),
        }
    }
}

/// Non-streaming Path B: run a web_search tool call and fold the summary into content.
async fn run_web_search(result: &mut crate::upstream::StreamResult, auth: &crate::auth::KiroAuth) {
    let Some(idx) = result
        .tool_calls
        .iter()
        .position(|tc| tc.name == "web_search")
    else {
        return;
    };
    let query = serde_json::from_str::<Value>(&result.tool_calls[idx].arguments)
        .ok()
        .and_then(|v| v.get("query").and_then(Value::as_str).map(str::to_string))
        .unwrap_or_default();
    if query.is_empty() {
        return;
    }
    if let Some((_, results)) = crate::mcp::call_web_search(auth, &query).await {
        let summary = crate::mcp::generate_search_summary(&query, &results);
        result.tool_calls.remove(idx);
        result.content.push_str(&summary);
    }
}

/// The thinking handling mode, or None when fake reasoning is disabled.
fn thinking_handling(config: &crate::config::GatewayConfig) -> Option<String> {
    if config.fake_reasoning {
        Some(config.fake_reasoning_handling.clone())
    } else {
        None
    }
}

fn estimate_input_tokens(request: &Value) -> u64 {
    let messages = request
        .get("messages")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let tools = request.get("tools").and_then(Value::as_array).cloned();
    let system = request.get("system").cloned();
    estimate_request_tokens(&messages, tools.as_deref(), system.as_ref(), false)
}

fn tool_input_value(tc: &crate::upstream::event_stream::ToolCall) -> Value {
    serde_json::from_str::<Value>(&tc.arguments).unwrap_or_else(|_| json!({}))
}

fn build_anthropic_message(
    model: &str,
    result: &crate::upstream::StreamResult,
    input_estimate: u64,
    max_input_tokens: u64,
    thinking: &Option<String>,
) -> Value {
    let mut content_blocks = Vec::new();
    let as_reasoning = thinking.as_deref() == Some("as_reasoning_content");
    if !result.thinking_content.is_empty() && as_reasoning {
        content_blocks.push(json!({
            "type": "thinking",
            "thinking": result.thinking_content,
            "signature": crate::util::generate_thinking_signature(),
        }));
    }
    // For non-reasoning handling modes, thinking text is prepended to content.
    let text = if !result.thinking_content.is_empty() && !as_reasoning {
        format!("{}{}", result.thinking_content, result.content)
    } else {
        result.content.clone()
    };
    if !text.is_empty() {
        content_blocks.push(json!({ "type": "text", "text": text }));
    }
    for tc in &result.tool_calls {
        content_blocks.push(json!({
            "type": "tool_use",
            "id": if tc.id.is_empty() { generate_tool_use_id() } else { tc.id.clone() },
            "name": tc.name,
            "input": tool_input_value(tc),
        }));
    }

    let output_tokens = count_text_tokens(&result.content, true);
    let input_tokens = tokens_from_context_usage(
        result.context_usage_percentage,
        output_tokens,
        max_input_tokens,
    )
    .unwrap_or(input_estimate);

    let stop_reason = if !result.tool_calls.is_empty() {
        "tool_use"
    } else {
        "end_turn"
    };

    json!({
        "id": generate_message_id(),
        "type": "message",
        "role": "assistant",
        "content": content_blocks,
        "model": model,
        "stop_reason": stop_reason,
        "stop_sequence": null,
        "usage": { "input_tokens": input_tokens, "output_tokens": output_tokens }
    })
}

#[allow(clippy::too_many_arguments)]
fn anthropic_sse_body(
    resp: reqwest::Response,
    model: String,
    input_estimate: u64,
    ft: f64,
    rt: f64,
    thinking: Option<String>,
    store: std::sync::Arc<crate::truncation::TruncationStore>,
    recovery: bool,
    auth: std::sync::Arc<crate::auth::KiroAuth>,
    web_search: bool,
) -> Body {
    let s = stream! {
        let as_reasoning = thinking.as_deref() == Some("as_reasoning_content");
        let message_id = generate_message_id();
        let mut truncated_tool_ids: Vec<String> = Vec::new();

        yield Ok::<_, std::convert::Infallible>(sse_event("message_start", &json!({
            "type": "message_start",
            "message": {
                "id": message_id,
                "type": "message",
                "role": "assistant",
                "content": [],
                "model": model,
                "stop_reason": null,
                "stop_sequence": null,
                "usage": { "input_tokens": input_estimate, "output_tokens": 0 }
            }
        })));

        let mut index = 0usize;
        let mut text_started = false;
        let mut thinking_started = false;
        let thinking_signature = crate::util::generate_thinking_signature();
        let mut full_content = String::new();
        let mut tool_count = 0usize;
        let mut context_usage: Option<f64> = None;

        let events = kiro_event_stream(resp, ft, rt, thinking.clone());
        futures::pin_mut!(events);

        while let Some(ev) = events.next().await {
            match ev {
                Ok(KiroEvent::Thinking { content, .. }) => {
                    if as_reasoning {
                        if !thinking_started {
                            yield Ok(sse_event("content_block_start", &json!({
                                "type": "content_block_start",
                                "index": index,
                                "content_block": { "type": "thinking", "thinking": "", "signature": thinking_signature }
                            })));
                            thinking_started = true;
                        }
                        yield Ok(sse_event("content_block_delta", &json!({
                            "type": "content_block_delta",
                            "index": index,
                            "delta": { "type": "thinking_delta", "thinking": content }
                        })));
                    } else {
                        // Treat as regular text.
                        full_content.push_str(&content);
                        if !text_started {
                            yield Ok(sse_event("content_block_start", &json!({
                                "type": "content_block_start",
                                "index": index,
                                "content_block": { "type": "text", "text": "" }
                            })));
                            text_started = true;
                        }
                        yield Ok(sse_event("content_block_delta", &json!({
                            "type": "content_block_delta",
                            "index": index,
                            "delta": { "type": "text_delta", "text": content }
                        })));
                    }
                }
                Ok(KiroEvent::Content(c)) => {
                    if thinking_started {
                        yield Ok(sse_event("content_block_stop", &json!({
                            "type": "content_block_stop", "index": index
                        })));
                        thinking_started = false;
                        index += 1;
                    }
                    full_content.push_str(&c);
                    if !text_started {
                        yield Ok(sse_event("content_block_start", &json!({
                            "type": "content_block_start",
                            "index": index,
                            "content_block": { "type": "text", "text": "" }
                        })));
                        text_started = true;
                    }
                    yield Ok(sse_event("content_block_delta", &json!({
                        "type": "content_block_delta",
                        "index": index,
                        "delta": { "type": "text_delta", "text": c }
                    })));
                }
                Ok(KiroEvent::ToolUse(tc)) => {
                    if thinking_started {
                        yield Ok(sse_event("content_block_stop", &json!({
                            "type": "content_block_stop", "index": index
                        })));
                        thinking_started = false;
                        index += 1;
                    }
                    if text_started {
                        yield Ok(sse_event("content_block_stop", &json!({
                            "type": "content_block_stop", "index": index
                        })));
                        text_started = false;
                        index += 1;
                    }
                    // Path B: intercept web_search, stream the summary as a text block.
                    if web_search && tc.name == "web_search" {
                        let query = serde_json::from_str::<Value>(&tc.arguments)
                            .ok()
                            .and_then(|v| v.get("query").and_then(Value::as_str).map(str::to_string))
                            .unwrap_or_default();
                        if !query.is_empty() {
                            if let Some((_, results)) = crate::mcp::call_web_search(&auth, &query).await {
                                let summary = crate::mcp::generate_search_summary(&query, &results);
                                full_content.push_str(&summary);
                                yield Ok(sse_event("content_block_start", &json!({
                                    "type": "content_block_start",
                                    "index": index,
                                    "content_block": { "type": "text", "text": "" }
                                })));
                                yield Ok(sse_event("content_block_delta", &json!({
                                    "type": "content_block_delta",
                                    "index": index,
                                    "delta": { "type": "text_delta", "text": summary }
                                })));
                                yield Ok(sse_event("content_block_stop", &json!({
                                    "type": "content_block_stop", "index": index
                                })));
                                index += 1;
                                continue;
                            }
                        }
                    }
                    let tool_id = if tc.id.is_empty() { generate_tool_use_id() } else { tc.id.clone() };
                    if tc.truncation_detected {
                        truncated_tool_ids.push(tool_id.clone());
                    }
                    let input = tool_input_value(&tc);
                    yield Ok(sse_event("content_block_start", &json!({
                        "type": "content_block_start",
                        "index": index,
                        "content_block": { "type": "tool_use", "id": tool_id, "name": tc.name, "input": {} }
                    })));
                    yield Ok(sse_event("content_block_delta", &json!({
                        "type": "content_block_delta",
                        "index": index,
                        "delta": { "type": "input_json_delta", "partial_json": serde_json::to_string(&input).unwrap_or_default() }
                    })));
                    yield Ok(sse_event("content_block_stop", &json!({
                        "type": "content_block_stop", "index": index
                    })));
                    index += 1;
                    tool_count += 1;
                }
                Ok(KiroEvent::ContextUsage(p)) => context_usage = Some(p),
                Ok(KiroEvent::Usage(_)) => {}
                Err(e) => {
                    yield Ok(sse_event("error", &e.to_anthropic_json()));
                    return;
                }
            }
        }

        if thinking_started {
            yield Ok(sse_event("content_block_stop", &json!({
                "type": "content_block_stop", "index": index
            })));
            index += 1;
        }
        if text_started {
            yield Ok(sse_event("content_block_stop", &json!({
                "type": "content_block_stop", "index": index
            })));
        }

        let output_tokens = count_text_tokens(&full_content, true);
        let stop_reason = if tool_count > 0 { "tool_use" } else { "end_turn" };

        if recovery {
            for id in &truncated_tool_ids {
                store.save_tool(id);
            }
            // Content truncation: no completion signal, has content, no tools.
            if context_usage.is_none() && !full_content.is_empty() && tool_count == 0 {
                store.save_content(&full_content);
            }
        }

        yield Ok(sse_event("message_delta", &json!({
            "type": "message_delta",
            "delta": { "stop_reason": stop_reason, "stop_sequence": null },
            "usage": { "output_tokens": output_tokens }
        })));
        yield Ok(sse_event("message_stop", &json!({ "type": "message_stop" })));
    };
    Body::from_stream(s)
}

pub async fn count_tokens(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Json<Value>,
) -> Response {
    if let Err(resp) = verify_api_key(&headers, &state.config) {
        return resp;
    }
    let request = body.0;
    let messages = request
        .get("messages")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let tools = request.get("tools").and_then(Value::as_array).cloned();
    let system = request.get("system").cloned();
    let input_tokens = estimate_request_tokens(&messages, tools.as_deref(), system.as_ref(), true);
    (
        StatusCode::OK,
        Json(json!({ "input_tokens": input_tokens })),
    )
        .into_response()
}
