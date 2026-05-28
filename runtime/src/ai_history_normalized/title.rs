fn parse_iso8601_seconds(value: &str) -> Option<f64> {
    DateTime::parse_from_rfc3339(value).ok().map(|date| {
        date.timestamp() as f64 + f64::from(date.timestamp_subsec_micros()) / 1_000_000.0
    })
}

fn parse_opencode_timestamp(value: &str) -> Option<f64> {
    let value = value.trim();
    if value.is_empty() {
        return None;
    }
    if let Ok(milliseconds) = value.parse::<f64>() {
        return Some(milliseconds / 1000.0);
    }
    parse_iso8601_seconds(value)
}

fn value_to_string(value: &Value) -> Option<String> {
    value
        .as_str()
        .map(str::to_string)
        .or_else(|| value.as_i64().map(|value| value.to_string()))
        .or_else(|| value.as_f64().map(|value| value.to_string()))
}

fn claude_role(row_type: Option<&str>) -> Option<HistoryRole> {
    match row_type {
        Some("user") => Some(HistoryRole::User),
        Some("assistant") | Some("tool_use") | Some("tool_result") => Some(HistoryRole::Assistant),
        _ => None,
    }
}

fn codex_role(row_type: Option<&str>) -> HistoryRole {
    if matches!(row_type, Some("turn_context") | Some("session_meta")) {
        HistoryRole::User
    } else {
        HistoryRole::Assistant
    }
}

fn decode_checkpoint_payload(value: Option<&str>) -> Option<AIExternalFileCheckpointPayload> {
    value.and_then(|value| serde_json::from_str(value).ok())
}

fn encode_checkpoint_payload(payload: &AIExternalFileCheckpointPayload) -> Option<String> {
    serde_json::to_string(payload).ok()
}

fn claude_title(row: &Value) -> Option<String> {
    if row.get("type").and_then(|value| value.as_str()) != Some("user") {
        return row
            .get("slug")
            .and_then(|value| value.as_str())
            .and_then(normalized_string);
    }
    let message = row.get("message").unwrap_or(&Value::Null);
    if let Some(content) = message
        .get("content")
        .and_then(|value| value.as_str())
        .and_then(normalized_string)
    {
        return Some(truncate_title(&content));
    }
    if let Some(items) = message.get("content").and_then(|value| value.as_array()) {
        for item in items {
            if let Some(text) = item
                .get("text")
                .and_then(|value| value.as_str())
                .and_then(normalized_string)
            {
                return Some(truncate_title(&text));
            }
        }
    }
    row.get("slug")
        .and_then(|value| value.as_str())
        .and_then(normalized_string)
}

fn codex_response_title(payload: &Value) -> Option<String> {
    if payload.get("type").and_then(|value| value.as_str()) != Some("message")
        || payload.get("role").and_then(|value| value.as_str()) != Some("user")
    {
        return None;
    }
    let content = payload.get("content").and_then(|value| value.as_array())?;
    for item in content {
        let Some(text) = item
            .get("text")
            .and_then(|value| value.as_str())
            .and_then(normalized_string)
        else {
            continue;
        };
        if !text.contains("<environment_context>") {
            return Some(truncate_title(&text));
        }
    }
    None
}

fn parse_gemini_title(content: Option<&Value>) -> Option<String> {
    match content? {
        Value::String(text) => Some(truncate_title(text)),
        Value::Array(items) => items.iter().find_map(|item| {
            item.get("text")
                .and_then(|value| value.as_str())
                .and_then(normalized_string)
                .map(|text| truncate_title(&text))
                .or_else(|| parse_gemini_title(item.get("content")))
        }),
        _ => None,
    }
}

fn truncate_title(value: &str) -> String {
    value
        .replace('\n', " ")
        .chars()
        .take(80)
        .collect::<String>()
        .trim()
        .to_string()
}
