//! Web search via Kiro's MCP API (`POST {api_host}/mcp`, JSON-RPC 2.0).
//! Port of mcp_tools.py (Path B interception + summary generation).

use std::time::Duration;

use rand::Rng;
use serde_json::{json, Value};

use crate::auth::KiroAuth;
use crate::error::GatewayError;

fn random_alnum(len: usize) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    let mut rng = rand::thread_rng();
    (0..len)
        .map(|_| CHARS[rng.gen_range(0..CHARS.len())] as char)
        .collect()
}

/// Call the Kiro MCP `web_search` tool.
pub async fn call_web_search(
    auth: &KiroAuth,
    query: &str,
) -> Result<(String, Value), GatewayError> {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let request_id = format!(
        "web_search_tooluse_{}_{}_{}",
        random_alnum(22),
        timestamp,
        random_alnum(8)
    );
    let tool_use_id = format!("srvtoolu_{}", uuid::Uuid::new_v4().simple());

    let profile_arn = auth
        .profile_arn()
        .await
        .ok_or_else(|| GatewayError::Auth("Kiro profile ARN is unavailable".into()))?;
    let mcp_request = json!({
        "id": request_id,
        "jsonrpc": "2.0",
        "method": "tools/call",
        "profileArn": profile_arn,
        "params": { "name": "web_search", "arguments": { "query": query } }
    });

    let token = auth.get_access_token().await?;
    let url = format!("{}/mcp", auth.api_host());

    let mut request = auth
        .http()
        .post(&url)
        .header("Content-Type", "application/json")
        .timeout(Duration::from_secs(60))
        .json(&mcp_request);
    for (name, value) in crate::util::kiro_headers(&token, auth.fingerprint()) {
        if name.eq_ignore_ascii_case("content-type") || name.eq_ignore_ascii_case("x-amz-target") {
            continue;
        }
        let value = if name.eq_ignore_ascii_case("x-amzn-codewhisperer-optout") {
            "true".to_string()
        } else {
            value
        };
        request = request.header(name, value);
    }
    let resp = request.send().await.map_err(|e| GatewayError::Upstream {
        status: 502,
        body: format!("Kiro MCP request failed: {e}"),
    })?;

    let status = resp.status();
    let body = resp.text().await.map_err(|e| GatewayError::Upstream {
        status: 502,
        body: format!("failed to read Kiro MCP response: {e}"),
    })?;
    if !status.is_success() {
        return Err(GatewayError::Upstream {
            status: status.as_u16(),
            body,
        });
    }
    let mcp_response: Value = serde_json::from_str(&body).map_err(|e| GatewayError::Upstream {
        status: 502,
        body: format!("invalid Kiro MCP response: {e}"),
    })?;
    if mcp_response
        .get("error")
        .map(|e| !e.is_null())
        .unwrap_or(false)
    {
        return Err(GatewayError::Upstream {
            status: 502,
            body: mcp_response["error"].to_string(),
        });
    }
    // result.content[0].text is a JSON *string*.
    let text = mcp_response
        .get("result")
        .and_then(|result| result.get("content"))
        .and_then(Value::as_array)
        .and_then(|content| content.first())
        .and_then(|item| item.get("text"))
        .and_then(Value::as_str)
        .ok_or_else(|| GatewayError::Upstream {
            status: 502,
            body: "Kiro MCP response is missing result.content[0].text".into(),
        })?;
    let results: Value = serde_json::from_str(text).map_err(|e| GatewayError::Upstream {
        status: 502,
        body: format!("invalid Kiro MCP search results: {e}"),
    })?;
    Ok((tool_use_id, results))
}

/// Build a human-readable summary of search results, wrapped in <web_search> tags.
pub fn generate_search_summary(query: &str, results: &Value) -> String {
    let mut summary = format!("\n<web_search>\nSearch results for \"{query}\":\n\n");
    if let Some(arr) = results.get("results").and_then(Value::as_array) {
        if arr.is_empty() {
            summary.push_str("No results found.\n");
        }
        for (i, r) in arr.iter().enumerate() {
            let title = r.get("title").and_then(Value::as_str).unwrap_or("Untitled");
            summary.push_str(&format!("{}. Title: **{}**\n", i + 1, title));
            if let Some(url) = r.get("url").and_then(Value::as_str) {
                if !url.is_empty() {
                    summary.push_str(&format!("   URL: {url}\n"));
                }
            }
            if let Some(snippet) = r.get("snippet").and_then(Value::as_str) {
                if !snippet.is_empty() {
                    summary.push_str(&format!("   {snippet}\n"));
                }
            }
            summary.push('\n');
        }
    } else {
        summary.push_str("No results found.\n");
    }
    summary.push_str("</web_search>\n");
    summary
}

/// The web_search tool definition (Anthropic shape). Injected when enabled.
pub fn web_search_tool_anthropic() -> Value {
    json!({
        "name": "web_search",
        "description": "Search the web for current information. Use when you need up-to-date facts, news, or documentation.",
        "input_schema": {
            "type": "object",
            "properties": { "query": { "type": "string", "description": "The search query" } },
            "required": ["query"]
        }
    })
}

/// The web_search tool definition (OpenAI shape).
pub fn web_search_tool_openai() -> Value {
    json!({
        "type": "function",
        "function": {
            "name": "web_search",
            "description": "Search the web for current information. Use when you need up-to-date facts, news, or documentation.",
            "parameters": {
                "type": "object",
                "properties": { "query": { "type": "string", "description": "The search query" } },
                "required": ["query"]
            }
        }
    })
}

/// Add the web_search tool to a request's `tools` array if not already present.
pub fn inject_web_search_tool(request: &mut Value, tool: Value) {
    let obj = match request.as_object_mut() {
        Some(o) => o,
        None => return,
    };
    let tools = obj
        .entry("tools")
        .or_insert_with(|| Value::Array(Vec::new()));
    let Some(arr) = tools.as_array_mut() else {
        return;
    };
    let already = arr.iter().any(|t| {
        t.get("name").and_then(Value::as_str) == Some("web_search")
            || t.get("function")
                .and_then(|f| f.get("name"))
                .and_then(Value::as_str)
                == Some("web_search")
    });
    if !already {
        arr.push(tool);
    }
}
