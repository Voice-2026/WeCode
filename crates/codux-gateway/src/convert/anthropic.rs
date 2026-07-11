//! Anthropic Messages API request → unified → Kiro payload.

use serde_json::Value;

use super::kiro::{build_kiro_payload, BuildParams};
use super::{ThinkingConfig, ToolCallSpec, ToolResult, UnifiedImage, UnifiedMessage, UnifiedTool};
use crate::config::GatewayConfig;
use crate::model_resolver::resolve_model_id;

pub fn extract_system_prompt(system: Option<&Value>) -> String {
    match system {
        None | Some(Value::Null) => String::new(),
        Some(Value::String(s)) => s.clone(),
        Some(Value::Array(blocks)) => blocks
            .iter()
            .filter_map(|b| {
                if b.get("type").and_then(Value::as_str) == Some("text") {
                    b.get("text").and_then(Value::as_str).map(str::to_string)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
            .join("\n"),
        Some(other) => other.to_string(),
    }
}

fn content_to_text(content: &Value) -> String {
    match content {
        Value::String(s) => s.clone(),
        Value::Array(blocks) => blocks
            .iter()
            .filter_map(|b| {
                if b.get("type").and_then(Value::as_str) == Some("text") {
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

pub fn extract_images_from_content(content: &Value) -> Vec<UnifiedImage> {
    let Value::Array(blocks) = content else {
        return Vec::new();
    };
    let mut images = Vec::new();
    for b in blocks {
        let ty = b.get("type").and_then(Value::as_str);
        match ty {
            Some("image") => {
                if let Some(source) = b.get("source") {
                    if source.get("type").and_then(Value::as_str) == Some("base64") {
                        let media_type = source
                            .get("media_type")
                            .and_then(Value::as_str)
                            .unwrap_or("image/jpeg")
                            .to_string();
                        let data = source.get("data").and_then(Value::as_str).unwrap_or("");
                        if !data.is_empty() {
                            images.push(UnifiedImage {
                                media_type,
                                data: data.to_string(),
                            });
                        }
                    }
                }
            }
            Some("image_url") => {
                if let Some(url) = b
                    .get("image_url")
                    .and_then(|u| u.get("url"))
                    .and_then(Value::as_str)
                {
                    if let Some(img) = parse_data_url(url) {
                        images.push(img);
                    }
                }
            }
            _ => {}
        }
    }
    images
}

fn parse_data_url(url: &str) -> Option<UnifiedImage> {
    let rest = url.strip_prefix("data:")?;
    let (header, data) = rest.split_once(',')?;
    if data.is_empty() {
        return None;
    }
    let media_type = header.split(';').next().unwrap_or("image/jpeg").to_string();
    Some(UnifiedImage {
        media_type,
        data: data.to_string(),
    })
}

fn extract_tool_results(content: &Value) -> Vec<ToolResult> {
    let Value::Array(blocks) = content else {
        return Vec::new();
    };
    let mut results = Vec::new();
    for b in blocks {
        if b.get("type").and_then(Value::as_str) == Some("tool_result") {
            if let Some(id) = b.get("tool_use_id").and_then(Value::as_str) {
                let content = match b.get("content") {
                    Some(Value::String(s)) => s.clone(),
                    Some(arr @ Value::Array(_)) => content_to_text(arr),
                    _ => String::new(),
                };
                results.push(ToolResult {
                    tool_use_id: id.to_string(),
                    content: if content.is_empty() {
                        "(empty result)".to_string()
                    } else {
                        content
                    },
                });
            }
        }
    }
    results
}

fn extract_images_from_tool_results(content: &Value) -> Vec<UnifiedImage> {
    let Value::Array(blocks) = content else {
        return Vec::new();
    };
    let mut images = Vec::new();
    for b in blocks {
        if b.get("type").and_then(Value::as_str) == Some("tool_result") {
            if let Some(inner @ Value::Array(_)) = b.get("content") {
                images.extend(extract_images_from_content(inner));
            }
        }
    }
    images
}

fn extract_tool_uses(content: &Value) -> Vec<ToolCallSpec> {
    let Value::Array(blocks) = content else {
        return Vec::new();
    };
    let mut calls = Vec::new();
    for b in blocks {
        if b.get("type").and_then(Value::as_str) == Some("tool_use") {
            let id = b.get("id").and_then(Value::as_str);
            let name = b.get("name").and_then(Value::as_str);
            if let (Some(id), Some(name)) = (id, name) {
                calls.push(ToolCallSpec {
                    id: id.to_string(),
                    name: name.to_string(),
                    input: b
                        .get("input")
                        .cloned()
                        .unwrap_or_else(|| Value::Object(Default::default())),
                });
            }
        }
    }
    calls
}

fn convert_messages(messages: &[Value]) -> Vec<UnifiedMessage> {
    let mut out = Vec::new();
    for msg in messages {
        let role = msg
            .get("role")
            .and_then(Value::as_str)
            .unwrap_or("user")
            .to_string();
        let content = msg.get("content").cloned().unwrap_or(Value::Null);
        let text = content_to_text(&content);

        let mut tool_calls = None;
        let mut tool_results = None;
        let mut images = None;

        if role == "assistant" {
            let calls = extract_tool_uses(&content);
            if !calls.is_empty() {
                tool_calls = Some(calls);
            }
        } else if role == "user" {
            let results = extract_tool_results(&content);
            if !results.is_empty() {
                tool_results = Some(results);
            }
            let mut imgs = extract_images_from_content(&content);
            imgs.extend(extract_images_from_tool_results(&content));
            if !imgs.is_empty() {
                images = Some(imgs);
            }
        }

        out.push(UnifiedMessage {
            role,
            content: text,
            tool_calls,
            tool_results,
            images,
        });
    }
    out
}

fn convert_tools(tools: Option<&Value>, provider_only: bool) -> Option<Vec<UnifiedTool>> {
    let arr = tools?.as_array()?;
    let mut out = Vec::new();
    for tool in arr {
        if provider_only && is_provider_managed_tool(tool) {
            continue;
        }
        let name = tool.get("name").and_then(Value::as_str)?.to_string();
        out.push(UnifiedTool {
            name,
            description: tool
                .get("description")
                .and_then(Value::as_str)
                .map(str::to_string),
            input_schema: tool.get("input_schema").cloned(),
        });
    }
    if out.is_empty() {
        None
    } else {
        Some(out)
    }
}

fn is_provider_managed_tool(tool: &Value) -> bool {
    tool.get("type")
        .and_then(Value::as_str)
        .is_some_and(|kind| kind.starts_with("web_search"))
}

fn extract_thinking(request: &Value) -> ThinkingConfig {
    let Some(thinking) = request.get("thinking") else {
        return ThinkingConfig::default();
    };
    match thinking.get("type").and_then(Value::as_str) {
        Some("disabled") => ThinkingConfig {
            enabled: false,
            budget_tokens: None,
        },
        Some("enabled") => ThinkingConfig {
            enabled: true,
            budget_tokens: thinking
                .get("budget_tokens")
                .and_then(Value::as_u64)
                .map(|b| b as u32),
        },
        _ => ThinkingConfig::default(),
    }
}

/// Convert an Anthropic `/v1/messages` request JSON into a Kiro payload.
pub fn anthropic_to_kiro(
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
    let unified = convert_messages(&messages);
    let tools = convert_tools(request.get("tools"), config.provider_only);
    let system = extract_system_prompt(request.get("system"));
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

    #[test]
    fn provider_only_drops_server_web_search_but_keeps_claude_code_tools() {
        let tools = json!([
            { "type": "web_search_20250305", "name": "web_search" },
            {
                "name": "Bash",
                "description": "Run a shell command",
                "input_schema": { "type": "object" }
            },
            {
                "name": "mcp__search__query",
                "description": "Search through a local MCP server",
                "input_schema": { "type": "object" }
            }
        ]);

        let converted = convert_tools(Some(&tools), true).expect("local tools");
        assert_eq!(converted.len(), 2);
        assert_eq!(converted[0].name, "Bash");
        assert_eq!(converted[1].name, "mcp__search__query");
    }

    #[test]
    fn legacy_gateway_mode_can_still_convert_server_web_search() {
        let tools = json!([{ "type": "web_search_20250305", "name": "web_search" }]);
        let converted = convert_tools(Some(&tools), false).expect("web search tool");
        assert_eq!(converted[0].name, "web_search");
    }
}
