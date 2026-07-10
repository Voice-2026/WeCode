use async_stream::stream;
use axum::body::Body;
use axum::extract::State;
use axum::http::HeaderMap;
use axum::response::{IntoResponse, Response};
use axum::Json;
use futures::StreamExt;
use serde_json::{json, Value};

use super::{verify_api_key, AppState};
use crate::convert::openai::openai_to_kiro;
use crate::tokens::count_tokens;
use crate::upstream::event_stream::ToolCall;
use crate::upstream::{collect_stream, kiro_event_stream, KiroEvent};
use crate::util::generate_completion_id;

fn conversation_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

fn tool_input_string(tc: &ToolCall) -> String {
    // arguments is already canonical JSON (or "{}").
    tc.arguments.clone()
}

pub async fn chat_completions(
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
    let (_auth, resp) = match state
        .accounts
        .request_with_failover(
            |profile_arn| {
                openai_to_kiro(
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
        Err(e) => return e.into_response(),
    };

    let ft = state.config.first_token_timeout_secs;
    let rt = state.config.streaming_read_timeout_secs;
    let thinking = if state.config.fake_reasoning {
        Some(state.config.fake_reasoning_handling.clone())
    } else {
        None
    };

    let recovery = state.config.truncation_recovery;
    let store = state.truncation.clone();

    if stream_requested {
        let body = openai_sse_body(resp, model, ft, rt, thinking, store, recovery);
        Response::builder()
            .header("content-type", "text/event-stream")
            .header("cache-control", "no-cache")
            .body(body)
            .unwrap()
    } else {
        match collect_stream(resp, ft, rt, thinking.clone()).await {
            Ok(result) => {
                if recovery {
                    super::save_truncations(&result, &store);
                }
                Json(build_openai_completion(&model, &result, &thinking)).into_response()
            }
            Err(e) => e.into_response(),
        }
    }
}

fn build_openai_completion(
    model: &str,
    result: &crate::upstream::StreamResult,
    thinking: &Option<String>,
) -> Value {
    let as_reasoning = thinking.as_deref() == Some("as_reasoning_content");
    let content = if !result.thinking_content.is_empty() && !as_reasoning {
        format!("{}{}", result.thinking_content, result.content)
    } else {
        result.content.clone()
    };
    let mut message = json!({ "role": "assistant", "content": content });
    if !result.thinking_content.is_empty() && as_reasoning {
        message["reasoning_content"] = json!(result.thinking_content);
    }
    if !result.tool_calls.is_empty() {
        let tcs: Vec<Value> = result
            .tool_calls
            .iter()
            .map(|tc| {
                json!({
                    "id": tc.id,
                    "type": "function",
                    "function": { "name": tc.name, "arguments": tool_input_string(tc) }
                })
            })
            .collect();
        message["tool_calls"] = Value::Array(tcs);
    }
    let finish_reason = if !result.tool_calls.is_empty() {
        "tool_calls"
    } else {
        "stop"
    };
    let completion_tokens = count_tokens(&result.content, true);

    json!({
        "id": generate_completion_id(),
        "object": "chat.completion",
        "created": chrono::Utc::now().timestamp(),
        "model": model,
        "choices": [{
            "index": 0,
            "message": message,
            "finish_reason": finish_reason
        }],
        "usage": {
            "prompt_tokens": 0,
            "completion_tokens": completion_tokens,
            "total_tokens": completion_tokens
        }
    })
}

#[allow(clippy::too_many_arguments)]
fn openai_sse_body(
    resp: reqwest::Response,
    model: String,
    ft: f64,
    rt: f64,
    thinking: Option<String>,
    store: std::sync::Arc<crate::truncation::TruncationStore>,
    recovery: bool,
) -> Body {
    let s = stream! {
        let as_reasoning = thinking.as_deref() == Some("as_reasoning_content");
        let id = generate_completion_id();
        let created = chrono::Utc::now().timestamp();
        let mut first_chunk = true;
        let mut full_content = String::new();
        let mut tool_calls: Vec<ToolCall> = Vec::new();
        let mut context_usage: Option<f64> = None;

        let chunk = |delta: Value, finish: Option<&str>| -> String {
            let obj = json!({
                "id": id,
                "object": "chat.completion.chunk",
                "created": created,
                "model": model,
                "choices": [{ "index": 0, "delta": delta, "finish_reason": finish }]
            });
            format!("data: {}\n\n", serde_json::to_string(&obj).unwrap_or_default())
        };

        let events = kiro_event_stream(resp, ft, rt, thinking.clone());
        futures::pin_mut!(events);

        while let Some(ev) = events.next().await {
            match ev {
                Ok(KiroEvent::Thinking { content, .. }) => {
                    let mut delta = if as_reasoning {
                        json!({ "reasoning_content": content })
                    } else {
                        full_content.push_str(&content);
                        json!({ "content": content })
                    };
                    if first_chunk {
                        delta["role"] = json!("assistant");
                        first_chunk = false;
                    }
                    yield Ok::<_, std::convert::Infallible>(chunk(delta, None));
                }
                Ok(KiroEvent::Content(c)) => {
                    full_content.push_str(&c);
                    let mut delta = json!({ "content": c });
                    if first_chunk {
                        delta["role"] = json!("assistant");
                        first_chunk = false;
                    }
                    yield Ok::<_, std::convert::Infallible>(chunk(delta, None));
                }
                Ok(KiroEvent::ToolUse(tc)) => {
                    tool_calls.push(tc);
                }
                Ok(KiroEvent::ContextUsage(p)) => context_usage = Some(p),
                Ok(KiroEvent::Usage(_)) => {}
                Err(e) => {
                    let err = json!({ "error": e.to_openai_json()["error"] });
                    yield Ok(format!("data: {}\n\n", serde_json::to_string(&err).unwrap_or_default()));
                    yield Ok("data: [DONE]\n\n".to_string());
                    return;
                }
            }
        }

        let finish_reason = if !tool_calls.is_empty() { "tool_calls" } else { "stop" };

        if recovery {
            for tc in &tool_calls {
                if tc.truncation_detected {
                    store.save_tool(&tc.id);
                }
            }
            if context_usage.is_none() && !full_content.is_empty() && tool_calls.is_empty() {
                store.save_content(&full_content);
            }
        }

        if !tool_calls.is_empty() {
            let indexed: Vec<Value> = tool_calls.iter().enumerate().map(|(i, tc)| {
                json!({
                    "index": i,
                    "id": tc.id,
                    "type": "function",
                    "function": { "name": tc.name, "arguments": tc.arguments }
                })
            }).collect();
            yield Ok(chunk(json!({ "tool_calls": indexed }), None));
        }

        let completion_tokens = count_tokens(&full_content, true);
        let _ = context_usage;

        let final_obj = json!({
            "id": id,
            "object": "chat.completion.chunk",
            "created": created,
            "model": model,
            "choices": [{ "index": 0, "delta": {}, "finish_reason": finish_reason }],
            "usage": {
                "prompt_tokens": 0,
                "completion_tokens": completion_tokens,
                "total_tokens": completion_tokens
            }
        });
        yield Ok(format!("data: {}\n\n", serde_json::to_string(&final_obj).unwrap_or_default()));
        yield Ok("data: [DONE]\n\n".to_string());
    };
    Body::from_stream(s)
}
