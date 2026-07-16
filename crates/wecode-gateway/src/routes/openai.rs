use async_stream::stream;
use axum::body::Body;
use axum::extract::State;
use axum::http::HeaderMap;
use axum::response::{IntoResponse, Response};
use axum::Json;
use futures::StreamExt;
use serde_json::{json, Value};

use super::{verify_api_key, AppState};
use crate::convert::openai::{openai_to_kiro, responses_to_kiro};
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

fn response_id() -> String {
    format!("resp_{}", uuid::Uuid::new_v4().simple())
}

fn response_message_id() -> String {
    format!("msg_{}", uuid::Uuid::new_v4().simple())
}

fn response_text_item(id: &str, text: &str, status: &str) -> Value {
    json!({
        "type": "message",
        "id": id,
        "status": status,
        "role": "assistant",
        "content": [{
            "type": "output_text",
            "text": text,
            "annotations": [],
            "logprobs": []
        }]
    })
}

fn response_function_item(tc: &ToolCall, status: &str) -> Value {
    json!({
        "type": "function_call",
        "id": tc.id,
        "call_id": tc.id,
        "name": tc.name,
        "arguments": tool_input_string(tc),
        "status": status
    })
}

#[allow(clippy::too_many_arguments)]
fn response_object(
    id: &str,
    model: &str,
    status: &str,
    output: Vec<Value>,
    input_tokens: u64,
    output_tokens: u64,
    reasoning_tokens: u64,
    error: Option<Value>,
) -> Value {
    let completed_at = (status == "completed").then(|| chrono::Utc::now().timestamp());
    json!({
        "id": id,
        "object": "response",
        "created_at": chrono::Utc::now().timestamp(),
        "completed_at": completed_at,
        "status": status,
        "error": error,
        "incomplete_details": null,
        "instructions": null,
        "max_output_tokens": null,
        "model": model,
        "output": output,
        "parallel_tool_calls": true,
        "previous_response_id": null,
        "reasoning": { "effort": null, "summary": null },
        "store": false,
        "temperature": null,
        "text": { "format": { "type": "text" } },
        "tool_choice": "auto",
        "tools": [],
        "top_p": null,
        "truncation": "disabled",
        "usage": {
            "input_tokens": input_tokens,
            "input_tokens_details": { "cached_tokens": 0 },
            "output_tokens": output_tokens,
            "output_tokens_details": { "reasoning_tokens": reasoning_tokens },
            "total_tokens": input_tokens + output_tokens
        }
    })
}

fn responses_sse(event_type: &str, sequence_number: u64, value: Value) -> String {
    let mut value = value;
    value["type"] = json!(event_type);
    value["sequence_number"] = json!(sequence_number);
    format!(
        "event: {event_type}\ndata: {}\n\n",
        serde_json::to_string(&value).unwrap_or_default()
    )
}

pub async fn responses(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Json<Value>,
) -> Response {
    if let Err(resp) = verify_api_key(&headers, &state.config) {
        return resp;
    }
    let request = body.0;
    let model = request
        .get("model")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let model_available = state
        .model_catalog
        .model(&model)
        .is_some_and(|entry| entry.compatibility.codex_cli);
    if !model_available {
        return crate::error::GatewayError::InvalidRequest(format!(
            "model '{model}' is not available for Codex CLI"
        ))
        .into_response();
    }
    let stream_requested = request
        .get("stream")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let input_tokens = count_tokens(&request.to_string(), false);
    let conv_id = conversation_id();
    let (_auth, resp) = match state
        .accounts
        .request_with_failover(
            |profile_arn| {
                responses_to_kiro(
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
        Ok(result) => result,
        Err(error) => return error.into_response(),
    };

    let ft = state.config.first_token_timeout_secs;
    let rt = state.config.streaming_read_timeout_secs;
    let thinking = if state.config.gateway_agent_features_enabled() && state.config.fake_reasoning {
        Some(state.config.fake_reasoning_handling.clone())
    } else {
        None
    };
    let recovery =
        state.config.gateway_agent_features_enabled() && state.config.truncation_recovery;
    let store = state.truncation.clone();

    if stream_requested {
        let body = responses_sse_body(resp, model, input_tokens, ft, rt, thinking, store, recovery);
        Response::builder()
            .header("content-type", "text/event-stream")
            .header("cache-control", "no-cache")
            .header("x-accel-buffering", "no")
            .body(body)
            .unwrap()
    } else {
        match collect_stream(resp, ft, rt, thinking).await {
            Ok(result) => {
                if recovery {
                    super::save_truncations(&result, &store);
                }
                Json(build_responses_completion(&model, input_tokens, &result)).into_response()
            }
            Err(error) => error.into_response(),
        }
    }
}

fn build_responses_completion(
    model: &str,
    input_tokens: u64,
    result: &crate::upstream::StreamResult,
) -> Value {
    let mut output = Vec::new();
    let content = format!("{}{}", result.thinking_content, result.content);
    if !content.is_empty() {
        output.push(response_text_item(
            &response_message_id(),
            &content,
            "completed",
        ));
    }
    output.extend(
        result
            .tool_calls
            .iter()
            .map(|tool_call| response_function_item(tool_call, "completed")),
    );
    let output_tokens = count_tokens(&content, false)
        + result
            .tool_calls
            .iter()
            .map(|tool_call| count_tokens(&tool_call.arguments, false))
            .sum::<u64>();
    response_object(
        &response_id(),
        model,
        "completed",
        output,
        input_tokens,
        output_tokens,
        count_tokens(&result.thinking_content, false),
        None,
    )
}

#[allow(clippy::too_many_arguments)]
fn responses_sse_body(
    resp: reqwest::Response,
    model: String,
    input_tokens: u64,
    ft: f64,
    rt: f64,
    thinking: Option<String>,
    store: std::sync::Arc<crate::truncation::TruncationStore>,
    recovery: bool,
) -> Body {
    let s = stream! {
        let id = response_id();
        let message_id = response_message_id();
        let mut sequence = 0u64;
        let mut full_content = String::new();
        let mut thinking_content = String::new();
        let mut text_started = false;
        let mut output: Vec<Value> = Vec::new();
        let mut tool_calls = Vec::new();
        let mut context_usage = None;

        let initial = response_object(&id, &model, "in_progress", Vec::new(), input_tokens, 0, 0, None);
        yield Ok::<_, std::convert::Infallible>(responses_sse("response.created", sequence, json!({ "response": initial })));
        sequence += 1;
        let in_progress = response_object(&id, &model, "in_progress", Vec::new(), input_tokens, 0, 0, None);
        yield Ok(responses_sse("response.in_progress", sequence, json!({ "response": in_progress })));
        sequence += 1;

        let events = kiro_event_stream(resp, ft, rt, thinking);
        futures::pin_mut!(events);
        while let Some(event) = events.next().await {
            let delta = match event {
                Ok(KiroEvent::Thinking { content, .. }) => {
                    thinking_content.push_str(&content);
                    content
                }
                Ok(KiroEvent::Content(content)) => content,
                Ok(KiroEvent::ToolUse(tool_call)) => {
                    tool_calls.push(tool_call);
                    continue;
                }
                Ok(KiroEvent::ContextUsage(usage)) => {
                    context_usage = Some(usage);
                    continue;
                }
                Ok(KiroEvent::Usage(_)) => continue,
                Err(error) => {
                    let failed = response_object(
                        &id,
                        &model,
                        "failed",
                        output,
                        input_tokens,
                        count_tokens(&full_content, false),
                        count_tokens(&thinking_content, false),
                        Some(json!({ "code": "api_error", "message": error.to_string() })),
                    );
                    yield Ok(responses_sse("response.failed", sequence, json!({ "response": failed })));
                    return;
                }
            };
            if delta.is_empty() {
                continue;
            }
            if !text_started {
                let item = json!({
                    "type": "message",
                    "id": message_id,
                    "status": "in_progress",
                    "role": "assistant",
                    "content": []
                });
                yield Ok(responses_sse("response.output_item.added", sequence, json!({ "output_index": 0, "item": item })));
                sequence += 1;
                yield Ok(responses_sse("response.content_part.added", sequence, json!({
                    "item_id": message_id,
                    "output_index": 0,
                    "content_index": 0,
                    "part": { "type": "output_text", "text": "", "annotations": [], "logprobs": [] }
                })));
                sequence += 1;
                text_started = true;
            }
            full_content.push_str(&delta);
            yield Ok(responses_sse("response.output_text.delta", sequence, json!({
                "item_id": message_id,
                "output_index": 0,
                "content_index": 0,
                "delta": delta,
                "logprobs": []
            })));
            sequence += 1;
        }

        if text_started {
            yield Ok(responses_sse("response.output_text.done", sequence, json!({
                "item_id": message_id,
                "output_index": 0,
                "content_index": 0,
                "text": full_content,
                "logprobs": []
            })));
            sequence += 1;
            let part = json!({ "type": "output_text", "text": full_content, "annotations": [], "logprobs": [] });
            yield Ok(responses_sse("response.content_part.done", sequence, json!({
                "item_id": message_id,
                "output_index": 0,
                "content_index": 0,
                "part": part
            })));
            sequence += 1;
            let item = response_text_item(&message_id, &full_content, "completed");
            yield Ok(responses_sse("response.output_item.done", sequence, json!({ "output_index": 0, "item": item })));
            sequence += 1;
            output.push(item);
        }

        for tool_call in &tool_calls {
            let output_index = output.len();
            let mut item = response_function_item(tool_call, "in_progress");
            item["arguments"] = json!("");
            yield Ok(responses_sse("response.output_item.added", sequence, json!({ "output_index": output_index, "item": item })));
            sequence += 1;
            if !tool_call.arguments.is_empty() {
                yield Ok(responses_sse("response.function_call_arguments.delta", sequence, json!({
                    "item_id": tool_call.id,
                    "output_index": output_index,
                    "delta": tool_call.arguments
                })));
                sequence += 1;
            }
            yield Ok(responses_sse("response.function_call_arguments.done", sequence, json!({
                "item_id": tool_call.id,
                "output_index": output_index,
                "arguments": tool_call.arguments
            })));
            sequence += 1;
            let item = response_function_item(tool_call, "completed");
            yield Ok(responses_sse("response.output_item.done", sequence, json!({ "output_index": output_index, "item": item })));
            sequence += 1;
            output.push(item);
        }

        if recovery {
            for tool_call in &tool_calls {
                if tool_call.truncation_detected {
                    store.save_tool(&tool_call.id);
                }
            }
            if context_usage.is_none() && !full_content.is_empty() && tool_calls.is_empty() {
                store.save_content(&full_content);
            }
        }
        let output_tokens = count_tokens(&full_content, false)
            + tool_calls.iter().map(|tool_call| count_tokens(&tool_call.arguments, false)).sum::<u64>();
        let completed = response_object(
            &id,
            &model,
            "completed",
            output,
            input_tokens,
            output_tokens,
            count_tokens(&thinking_content, false),
            None,
        );
        yield Ok(responses_sse("response.completed", sequence, json!({ "response": completed })));
    };
    Body::from_stream(s)
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
    if state.config.gateway_agent_features_enabled() && state.config.truncation_recovery {
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
    let thinking = if state.config.gateway_agent_features_enabled() && state.config.fake_reasoning {
        Some(state.config.fake_reasoning_handling.clone())
    } else {
        None
    };

    let recovery =
        state.config.gateway_agent_features_enabled() && state.config.truncation_recovery;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::upstream::StreamResult;

    #[test]
    fn responses_completion_contains_text_tools_and_usage() {
        let result = StreamResult {
            content: "done".into(),
            thinking_content: String::new(),
            tool_calls: vec![ToolCall {
                id: "call_1".into(),
                name: "read_file".into(),
                arguments: "{\"path\":\"a.txt\"}".into(),
                truncation_detected: false,
            }],
            ..Default::default()
        };
        let response = build_responses_completion("gpt-5.6-terra", 12, &result);
        assert_eq!(response["object"], "response");
        assert_eq!(response["status"], "completed");
        assert_eq!(response["output"][0]["content"][0]["text"], "done");
        assert_eq!(response["output"][1]["type"], "function_call");
        assert_eq!(response["output"][1]["call_id"], "call_1");
        assert!(response["usage"]["total_tokens"].as_u64().unwrap() >= 12);
    }

    #[test]
    fn responses_sse_uses_named_event_and_sequence_number() {
        let event = responses_sse("response.output_text.delta", 4, json!({ "delta": "hello" }));
        assert!(event.starts_with("event: response.output_text.delta\n"));
        assert!(event.contains("\"type\":\"response.output_text.delta\""));
        assert!(event.contains("\"sequence_number\":4"));
    }
}
