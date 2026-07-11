//! OpenAI Chat Completions request → unified → Kiro payload.

use serde_json::Value;

use super::anthropic::extract_images_from_content;
use super::kiro::{build_kiro_payload, BuildParams};
use super::{ThinkingConfig, ToolCallSpec, ToolResult, UnifiedMessage, UnifiedTool};
use crate::config::GatewayConfig;
use crate::model_resolver::resolve_model_id;

fn content_to_text(content: &Value) -> String {
    match content {
        Value::String(s) => s.clone(),
        Value::Array(blocks) => blocks
            .iter()
            .filter_map(|b| {
                let ty = b.get("type").and_then(Value::as_str);
                if ty == Some("text") {
                    b.get("text").and_then(Value::as_str).map(str::to_string)
                } else if ty.is_none() {
                    b.get("text").and_then(Value::as_str).map(str::to_string)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
            .join(""),
        _ => String::new(),
    }
}

fn extract_tool_calls(msg: &Value) -> Vec<ToolCallSpec> {
    let Some(tcs) = msg.get("tool_calls").and_then(Value::as_array) else {
        return Vec::new();
    };
    tcs.iter()
        .filter_map(|tc| {
            let func = tc.get("function")?;
            let name = func
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            let args_str = func
                .get("arguments")
                .and_then(Value::as_str)
                .unwrap_or("{}");
            let input = serde_json::from_str::<Value>(args_str)
                .unwrap_or_else(|_| Value::Object(Default::default()));
            Some(ToolCallSpec {
                id: tc
                    .get("id")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string(),
                name,
                input,
            })
        })
        .collect()
}

/// Returns (system_prompt, unified_messages).
fn convert_messages(messages: &[Value]) -> (String, Vec<UnifiedMessage>) {
    let mut system = String::new();
    let mut non_system: Vec<&Value> = Vec::new();
    for msg in messages {
        if msg.get("role").and_then(Value::as_str) == Some("system") {
            system.push_str(&content_to_text(msg.get("content").unwrap_or(&Value::Null)));
            system.push('\n');
        } else {
            non_system.push(msg);
        }
    }
    let system = system.trim().to_string();

    let mut processed: Vec<UnifiedMessage> = Vec::new();
    let mut pending_results: Vec<ToolResult> = Vec::new();

    for msg in non_system {
        let role = msg.get("role").and_then(Value::as_str).unwrap_or("user");
        if role == "tool" {
            let content = content_to_text(msg.get("content").unwrap_or(&Value::Null));
            pending_results.push(ToolResult {
                tool_use_id: msg
                    .get("tool_call_id")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string(),
                content: if content.is_empty() {
                    "(empty result)".to_string()
                } else {
                    content
                },
            });
            continue;
        }
        if !pending_results.is_empty() {
            processed.push(UnifiedMessage {
                role: "user".into(),
                content: String::new(),
                tool_results: Some(std::mem::take(&mut pending_results)),
                ..Default::default()
            });
        }
        let content_val = msg.get("content").cloned().unwrap_or(Value::Null);
        let mut tool_calls = None;
        let mut images = None;
        if role == "assistant" {
            let calls = extract_tool_calls(msg);
            if !calls.is_empty() {
                tool_calls = Some(calls);
            }
        } else if role == "user" {
            let imgs = extract_images_from_content(&content_val);
            if !imgs.is_empty() {
                images = Some(imgs);
            }
        }
        processed.push(UnifiedMessage {
            role: role.to_string(),
            content: content_to_text(&content_val),
            tool_calls,
            tool_results: None,
            images,
        });
    }
    if !pending_results.is_empty() {
        processed.push(UnifiedMessage {
            role: "user".into(),
            content: String::new(),
            tool_results: Some(pending_results),
            ..Default::default()
        });
    }
    (system, processed)
}

fn convert_tools(tools: Option<&Value>) -> Option<Vec<UnifiedTool>> {
    let arr = tools?.as_array()?;
    let mut out = Vec::new();
    for tool in arr {
        if tool.get("type").and_then(Value::as_str) != Some("function") {
            // Allow flat (Cursor-style) tools too.
            if let Some(name) = tool.get("name").and_then(Value::as_str) {
                out.push(UnifiedTool {
                    name: name.to_string(),
                    description: tool
                        .get("description")
                        .and_then(Value::as_str)
                        .map(str::to_string),
                    input_schema: tool.get("input_schema").cloned(),
                });
            }
            continue;
        }
        if let Some(func) = tool.get("function") {
            if let Some(name) = func.get("name").and_then(Value::as_str) {
                out.push(UnifiedTool {
                    name: name.to_string(),
                    description: func
                        .get("description")
                        .and_then(Value::as_str)
                        .map(str::to_string),
                    input_schema: func.get("parameters").cloned(),
                });
            }
        }
    }
    if out.is_empty() {
        None
    } else {
        Some(out)
    }
}

fn reasoning_effort_to_budget(max_tokens: u32, effort: &str) -> u32 {
    let percent = match effort {
        "minimal" => 0.10,
        "low" => 0.20,
        "medium" => 0.50,
        "high" => 0.80,
        "xhigh" => 0.95,
        _ => 0.0,
    };
    (max_tokens as f64 * percent) as u32
}

fn extract_thinking(request: &Value) -> ThinkingConfig {
    let Some(effort) = request.get("reasoning_effort").and_then(Value::as_str) else {
        return ThinkingConfig::default();
    };
    if effort == "none" {
        return ThinkingConfig {
            enabled: false,
            budget_tokens: None,
        };
    }
    let max_tokens = request
        .get("max_tokens")
        .and_then(Value::as_u64)
        .or_else(|| request.get("max_completion_tokens").and_then(Value::as_u64))
        .unwrap_or(4096) as u32;
    ThinkingConfig {
        enabled: true,
        budget_tokens: Some(reasoning_effort_to_budget(max_tokens, effort)),
    }
}

/// Convert an OpenAI `/v1/chat/completions` request JSON into a Kiro payload.
pub fn openai_to_kiro(
    request: &Value,
    conversation_id: String,
    profile_arn: String,
    config: &GatewayConfig,
) -> Result<Value, crate::error::GatewayError> {
    let messages = request
        .get("messages")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let (system, unified) = convert_messages(&messages);
    let tools = convert_tools(request.get("tools"));
    let model = request
        .get("model")
        .and_then(Value::as_str)
        .unwrap_or("auto");
    let model_id = resolve_model_id(model, &config.model_aliases, &config.hidden_models);
    let thinking = extract_thinking(request);

    let result = build_kiro_payload(
        unified,
        BuildParams {
            system_prompt: system,
            model_id,
            tools,
            conversation_id,
            profile_arn,
            thinking,
            config,
        },
    )?;
    Ok(result.payload)
}
