//! OpenAI Chat Completions request → unified → Kiro payload.

use serde_json::Value;

use super::anthropic::extract_images_from_content;
use super::kiro::{build_kiro_payload, BuildParams};
use super::{ThinkingConfig, ToolCallSpec, ToolResult, UnifiedMessage, UnifiedTool};
use crate::config::GatewayConfig;

/// Codex is the agent runtime. Kiro supplies the model behind the gateway but
/// must not replace the agent identity exposed to the user.
pub const CODEX_AGENT_BASE_INSTRUCTIONS: &str = "You are Codex, a coding agent running in the Codex CLI, a terminal-based coding assistant. Codex is the agent runtime. The selected model is supplied through the Kiro provider; Kiro is only the model provider, not the agent. When asked who you are, identify yourself as Codex and do not identify yourself as Kiro or as a Kiro agent.";
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
                    input_schema: tool
                        .get("parameters")
                        .or_else(|| tool.get("input_schema"))
                        .cloned(),
                });
            }
            continue;
        }
        let func = tool.get("function").unwrap_or(tool);
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
    if out.is_empty() {
        None
    } else {
        Some(out)
    }
}

fn responses_content_to_text(content: &Value) -> Result<String, crate::error::GatewayError> {
    match content {
        Value::String(text) => Ok(text.clone()),
        Value::Array(parts) => {
            let mut text = String::new();
            for part in parts {
                let part_type = part.get("type").and_then(Value::as_str).unwrap_or("text");
                match part_type {
                    "input_text" | "output_text" | "text" => {
                        text.push_str(part.get("text").and_then(Value::as_str).unwrap_or(""));
                    }
                    other => {
                        return Err(crate::error::GatewayError::InvalidRequest(format!(
                            "unsupported Responses content part: {other}"
                        )));
                    }
                }
            }
            Ok(text)
        }
        Value::Null => Ok(String::new()),
        _ => Err(crate::error::GatewayError::InvalidRequest(
            "Responses message content must be a string or text part array".into(),
        )),
    }
}

fn append_response_function_call(
    messages: &mut Vec<Value>,
    item: &Value,
) -> Result<(), crate::error::GatewayError> {
    let call_id = item
        .get("call_id")
        .or_else(|| item.get("id"))
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            crate::error::GatewayError::InvalidRequest(
                "Responses function_call requires call_id".into(),
            )
        })?;
    let name = item
        .get("name")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            crate::error::GatewayError::InvalidRequest(
                "Responses function_call requires name".into(),
            )
        })?;
    let arguments = match item.get("arguments") {
        Some(Value::String(arguments)) => arguments.clone(),
        Some(arguments) => serde_json::to_string(arguments).unwrap_or_else(|_| "{}".into()),
        None => "{}".to_string(),
    };
    let tool_call = serde_json::json!({
        "id": call_id,
        "type": "function",
        "function": { "name": name, "arguments": arguments }
    });
    if let Some(last) = messages.last_mut().filter(|message| {
        message.get("role").and_then(Value::as_str) == Some("assistant")
            && message
                .get("content")
                .and_then(Value::as_str)
                .unwrap_or("")
                .is_empty()
    }) {
        if let Some(calls) = last.get_mut("tool_calls").and_then(Value::as_array_mut) {
            calls.push(tool_call);
            return Ok(());
        }
    }
    messages.push(serde_json::json!({
        "role": "assistant",
        "content": "",
        "tool_calls": [tool_call]
    }));
    Ok(())
}

/// Convert an OpenAI Responses request into the existing Chat Completions
/// adapter shape, then reuse the shared Kiro payload pipeline.
pub fn responses_to_kiro(
    request: &Value,
    conversation_id: String,
    profile_arn: String,
    config: &GatewayConfig,
) -> Result<Value, crate::error::GatewayError> {
    if request
        .get("previous_response_id")
        .is_some_and(|value| !value.is_null())
    {
        return Err(crate::error::GatewayError::InvalidRequest(
            "previous_response_id is not supported; send the prior output items in input".into(),
        ));
    }

    let mut messages = Vec::new();
    let mut tools = request
        .get("tools")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut system_instructions = CODEX_AGENT_BASE_INSTRUCTIONS.to_string();
    if let Some(instructions) = request.get("instructions") {
        let instructions = instructions.as_str().ok_or_else(|| {
            crate::error::GatewayError::InvalidRequest(
                "Responses instructions must be a string".into(),
            )
        })?;
        if !instructions.is_empty() && instructions != CODEX_AGENT_BASE_INSTRUCTIONS {
            system_instructions.push_str("\n\n");
            system_instructions.push_str(instructions);
        }
    }
    messages.push(serde_json::json!({
        "role": "system",
        "content": system_instructions
    }));

    match request.get("input") {
        Some(Value::String(text)) => {
            messages.push(serde_json::json!({ "role": "user", "content": text }));
        }
        Some(Value::Array(items)) => {
            for item in items {
                let item_type = item
                    .get("type")
                    .and_then(Value::as_str)
                    .unwrap_or("message");
                match item_type {
                    "additional_tools" => {
                        let additional =
                            item.get("tools").and_then(Value::as_array).ok_or_else(|| {
                                crate::error::GatewayError::InvalidRequest(
                                    "Responses additional_tools requires a tools array".into(),
                                )
                            })?;
                        tools.extend(additional.iter().cloned());
                    }
                    "message" => {
                        let mut role = item.get("role").and_then(Value::as_str).unwrap_or("user");
                        if role == "developer" {
                            role = "system";
                        }
                        if !matches!(role, "system" | "user" | "assistant") {
                            return Err(crate::error::GatewayError::InvalidRequest(format!(
                                "unsupported Responses message role: {role}"
                            )));
                        }
                        let content =
                            responses_content_to_text(item.get("content").unwrap_or(&Value::Null))?;
                        messages.push(serde_json::json!({ "role": role, "content": content }));
                    }
                    "function_call" => append_response_function_call(&mut messages, item)?,
                    "function_call_output" => {
                        let call_id = item
                            .get("call_id")
                            .and_then(Value::as_str)
                            .filter(|value| !value.is_empty())
                            .ok_or_else(|| {
                                crate::error::GatewayError::InvalidRequest(
                                    "Responses function_call_output requires call_id".into(),
                                )
                            })?;
                        let output =
                            responses_content_to_text(item.get("output").unwrap_or(&Value::Null))?;
                        messages.push(serde_json::json!({
                            "role": "tool",
                            "tool_call_id": call_id,
                            "content": output
                        }));
                    }
                    other => {
                        return Err(crate::error::GatewayError::InvalidRequest(format!(
                            "unsupported Responses input item: {other}"
                        )));
                    }
                }
            }
        }
        None => {
            return Err(crate::error::GatewayError::InvalidRequest(
                "Responses input is required".into(),
            ));
        }
        _ => {
            return Err(crate::error::GatewayError::InvalidRequest(
                "Responses input must be a string or item array".into(),
            ));
        }
    }

    // Kiro's stateless runtime can ignore a turn whose current message only
    // contains structured toolResults. Keep the structured result and add a
    // small continuation message so the model resumes the original request.
    if messages
        .last()
        .and_then(|message| message.get("role"))
        .and_then(Value::as_str)
        == Some("tool")
    {
        messages.push(serde_json::json!({
            "role": "user",
            "content": "Continue the original request using the tool result above."
        }));
    }

    let reasoning_effort = request
        .get("reasoning")
        .and_then(|reasoning| reasoning.get("effort"))
        .cloned();
    let mut chat_request = serde_json::json!({
        "model": request.get("model").cloned().unwrap_or_else(|| Value::String("auto".into())),
        "messages": messages,
        "tools": tools,
        "max_completion_tokens": request
            .get("max_output_tokens")
            .cloned()
            .unwrap_or_else(|| Value::from(4096)),
    });
    if let Some(effort) = reasoning_effort {
        chat_request["reasoning_effort"] = effort;
    }
    openai_to_kiro(&chat_request, conversation_id, profile_arn, config)
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn config() -> GatewayConfig {
        GatewayConfig::default()
    }

    #[test]
    fn responses_maps_instructions_messages_and_flat_function_tools() {
        let payload = responses_to_kiro(
            &json!({
                "model": "gpt-5.6-terra",
                "instructions": "Be precise.",
                "input": [{
                    "type": "message",
                    "role": "user",
                    "content": [{ "type": "input_text", "text": "hello" }]
                }],
                "tools": [{
                    "type": "function",
                    "name": "read_file",
                    "description": "Read a file",
                    "parameters": {
                        "type": "object",
                        "properties": { "path": { "type": "string" } },
                        "required": ["path"]
                    }
                }],
                "reasoning": { "effort": "high" },
                "max_output_tokens": 1000
            }),
            "conversation-1".into(),
            String::new(),
            &config(),
        )
        .unwrap();

        let state = &payload["conversationState"];
        assert_eq!(state["conversationId"], "conversation-1");
        assert_eq!(
            state["currentMessage"]["userInputMessage"]["modelId"],
            "gpt-5.6-terra"
        );
        assert!(state["currentMessage"]["userInputMessage"]["content"]
            .as_str()
            .unwrap()
            .contains("You are Codex"));
        assert!(state["currentMessage"]["userInputMessage"]["content"]
            .as_str()
            .unwrap()
            .contains("Kiro is only the model provider"));
        assert!(state["currentMessage"]["userInputMessage"]["content"]
            .as_str()
            .unwrap()
            .contains("Be precise."));
        assert_eq!(
            state["currentMessage"]["userInputMessage"]["userInputMessageContext"]["tools"][0]
                ["toolSpecification"]["name"],
            "read_file"
        );
    }

    #[test]
    fn responses_maps_function_call_and_output_into_tool_history() {
        let payload = responses_to_kiro(
            &json!({
                "model": "gpt-5.6-sol",
                "input": [
                    { "type": "message", "role": "user", "content": "read it" },
                    {
                        "type": "function_call",
                        "call_id": "call_1",
                        "name": "read_file",
                        "arguments": "{\"path\":\"a.txt\"}"
                    },
                    {
                        "type": "function_call_output",
                        "call_id": "call_1",
                        "output": "contents"
                    }
                ],
                "tools": [{
                    "type": "function",
                    "name": "read_file",
                    "parameters": { "type": "object" }
                }]
            }),
            "conversation-2".into(),
            String::new(),
            &config(),
        )
        .unwrap();

        let state = &payload["conversationState"];
        let history = state["history"].as_array().unwrap();
        assert!(history
            .iter()
            .any(|message| message.to_string().contains("call_1")));
        assert_eq!(
            state["currentMessage"]["userInputMessage"]["userInputMessageContext"]["toolResults"]
                [0]["toolUseId"],
            "call_1"
        );
        assert_eq!(
            state["currentMessage"]["userInputMessage"]["userInputMessageContext"]["toolResults"]
                [0]["content"][0]["text"],
            "contents"
        );
        assert!(state["currentMessage"]["userInputMessage"]["content"]
            .as_str()
            .unwrap()
            .contains("Continue the original request"));
    }

    #[test]
    fn responses_rejects_unsupported_items_and_server_side_history_ids() {
        let unsupported = json!({
            "model": "gpt-5.6-luna",
            "input": [{ "type": "computer_call" }]
        });
        assert!(responses_to_kiro(
            &unsupported,
            "conversation-3".into(),
            String::new(),
            &config(),
        )
        .is_err());

        let previous = json!({
            "model": "gpt-5.6-luna",
            "previous_response_id": "resp_previous",
            "input": "continue"
        });
        assert!(
            responses_to_kiro(&previous, "conversation-4".into(), String::new(), &config(),)
                .is_err()
        );
    }

    #[test]
    fn responses_lite_additional_tools_are_merged_into_the_kiro_context() {
        let payload = responses_to_kiro(
            &json!({
                "model": "gpt-5.6-luna",
                "input": [
                    {
                        "type": "additional_tools",
                        "role": "developer",
                        "tools": [{
                            "type": "function",
                            "name": "shell_command",
                            "description": "Run a shell command",
                            "parameters": { "type": "object" }
                        }]
                    },
                    {
                        "type": "message",
                        "role": "developer",
                        "content": [{ "type": "input_text", "text": "Use tools when needed." }]
                    },
                    { "type": "message", "role": "user", "content": "pwd" }
                ]
            }),
            "conversation-lite".into(),
            String::new(),
            &config(),
        )
        .unwrap();

        assert_eq!(
            payload["conversationState"]["currentMessage"]["userInputMessage"]
                ["userInputMessageContext"]["tools"][0]["toolSpecification"]["name"],
            "shell_command"
        );
    }
}
