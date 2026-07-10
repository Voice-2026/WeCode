//! Web search via Kiro's MCP API (`POST {api_host}/mcp`, JSON-RPC 2.0).
//! Port of mcp_tools.py (Path B interception + summary generation).

use std::time::Duration;

use rand::Rng;
use serde_json::{json, Value};

use crate::auth::KiroAuth;

fn random_alnum(len: usize) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    let mut rng = rand::thread_rng();
    (0..len)
        .map(|_| CHARS[rng.gen_range(0..CHARS.len())] as char)
        .collect()
}

/// Call the Kiro MCP `web_search` tool. Returns (tool_use_id, results_json) or None.
pub async fn call_web_search(auth: &KiroAuth, query: &str) -> Option<(String, Value)> {
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

    let mcp_request = json!({
        "id": request_id,
        "jsonrpc": "2.0",
        "method": "tools/call",
        "params": { "name": "web_search", "arguments": { "query": query } }
    });

    let token = auth.get_access_token().await.ok()?;
    let url = format!("{}/mcp", auth.api_host());

    let resp = auth
        .http()
        .post(&url)
        .header("Authorization", format!("Bearer {token}"))
        .header("x-amzn-codewhisperer-optout", "false")
        .header("Content-Type", "application/json")
        .timeout(Duration::from_secs(60))
        .json(&mcp_request)
        .send()
        .await
        .ok()?;

    if !resp.status().is_success() {
        tracing::error!("MCP API error: {}", resp.status());
        return None;
    }
    let mcp_response: Value = resp.json().await.ok()?;
    if mcp_response
        .get("error")
        .map(|e| !e.is_null())
        .unwrap_or(false)
    {
        tracing::error!("MCP API returned error: {:?}", mcp_response.get("error"));
        return None;
    }
    // result.content[0].text is a JSON *string*.
    let text = mcp_response
        .get("result")?
        .get("content")?
        .get(0)?
        .get("text")?
        .as_str()?;
    let results: Value = serde_json::from_str(text).ok()?;
    Some((tool_use_id, results))
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
