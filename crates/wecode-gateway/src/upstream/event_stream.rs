//! Heuristic parser for Kiro's AWS event-stream responses.
//!
//! Faithful port of the Python `AwsEventStreamParser`: instead of decoding AWS
//! event-stream framing, it scans the decoded UTF-8 buffer for known JSON event
//! prefixes and extracts each object with brace matching. See parsers.py.

use serde_json::Value;

use crate::util::generate_tool_call_id;

/// Find the index of the matching closing brace for the `{` at `start`,
/// accounting for quoted strings and escapes. Operates on chars via byte
/// indices of a &str. Returns None if unbalanced.
pub fn find_matching_brace(text: &str, start: usize) -> Option<usize> {
    let bytes = text.as_bytes();
    if start >= bytes.len() || bytes[start] != b'{' {
        return None;
    }
    let mut brace_count = 0i32;
    let mut in_string = false;
    let mut escape_next = false;
    let mut i = start;
    while i < bytes.len() {
        let c = bytes[i];
        if escape_next {
            escape_next = false;
            i += 1;
            continue;
        }
        if c == b'\\' && in_string {
            escape_next = true;
            i += 1;
            continue;
        }
        if c == b'"' {
            in_string = !in_string;
            i += 1;
            continue;
        }
        if !in_string {
            if c == b'{' {
                brace_count += 1;
            } else if c == b'}' {
                brace_count -= 1;
                if brace_count == 0 {
                    return Some(i);
                }
            }
        }
        i += 1;
    }
    None
}

const EVENT_PATTERNS: &[(&str, EventKind)] = &[
    ("{\"content\":", EventKind::Content),
    ("{\"name\":", EventKind::ToolStart),
    ("{\"input\":", EventKind::ToolInput),
    ("{\"stop\":", EventKind::ToolStop),
    ("{\"followupPrompt\":", EventKind::Followup),
    ("{\"usage\":", EventKind::Usage),
    ("{\"contextUsagePercentage\":", EventKind::ContextUsage),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EventKind {
    Content,
    ToolStart,
    ToolInput,
    ToolStop,
    Followup,
    Usage,
    ContextUsage,
}

/// A parsed event emitted to the streaming layer.
#[derive(Debug, Clone)]
pub enum ParsedEvent {
    Content(String),
    Usage(Value),
    ContextUsage(f64),
}

/// An accumulated tool call.
#[derive(Debug, Clone)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    /// JSON string of arguments (normalized to "{}" on failure).
    pub arguments: String,
    pub truncation_detected: bool,
}

#[derive(Default)]
pub struct AwsEventStreamParser {
    buffer: String,
    last_content: Option<String>,
    current_tool_call: Option<PartialToolCall>,
    tool_calls: Vec<ToolCall>,
}

#[derive(Clone)]
struct PartialToolCall {
    id: String,
    name: String,
    arguments: String,
}

impl AwsEventStreamParser {
    pub fn new() -> Self {
        Self::default()
    }

    /// Feed a chunk of bytes, returning any complete events.
    pub fn feed(&mut self, chunk: &[u8]) -> Vec<ParsedEvent> {
        self.buffer.push_str(&String::from_utf8_lossy(chunk));
        let mut events = Vec::new();

        loop {
            let mut earliest: Option<(usize, EventKind)> = None;
            for (pattern, kind) in EVENT_PATTERNS {
                if let Some(pos) = self.buffer.find(pattern) {
                    match earliest {
                        Some((e, _)) if pos >= e => {}
                        _ => earliest = Some((pos, *kind)),
                    }
                }
            }
            let Some((pos, kind)) = earliest else { break };
            let Some(end) = find_matching_brace(&self.buffer, pos) else {
                break; // incomplete JSON, wait for more
            };
            let json_str = self.buffer[pos..=end].to_string();
            self.buffer = self.buffer[end + 1..].to_string();

            if let Ok(data) = serde_json::from_str::<Value>(&json_str) {
                if let Some(ev) = self.process_event(&data, kind) {
                    events.push(ev);
                }
            }
        }
        events
    }

    fn process_event(&mut self, data: &Value, kind: EventKind) -> Option<ParsedEvent> {
        match kind {
            EventKind::Content => self.process_content(data),
            EventKind::ToolStart => {
                self.process_tool_start(data);
                None
            }
            EventKind::ToolInput => {
                self.process_tool_input(data);
                None
            }
            EventKind::ToolStop => {
                if self.current_tool_call.is_some() && data.get("stop").is_some() {
                    self.finalize_tool_call();
                }
                None
            }
            EventKind::Followup => None,
            EventKind::Usage => Some(ParsedEvent::Usage(
                data.get("usage").cloned().unwrap_or(Value::from(0)),
            )),
            EventKind::ContextUsage => Some(ParsedEvent::ContextUsage(
                data.get("contextUsagePercentage")
                    .and_then(Value::as_f64)
                    .unwrap_or(0.0),
            )),
        }
    }

    fn process_content(&mut self, data: &Value) -> Option<ParsedEvent> {
        if data.get("followupPrompt").is_some() {
            return None;
        }
        let content = data
            .get("content")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        if Some(&content) == self.last_content.as_ref() {
            return None;
        }
        self.last_content = Some(content.clone());
        Some(ParsedEvent::Content(content))
    }

    fn input_to_string(input: Option<&Value>) -> String {
        match input {
            Some(Value::Object(map)) => {
                if map.is_empty() {
                    String::new()
                } else {
                    serde_json::to_string(&Value::Object(map.clone())).unwrap_or_default()
                }
            }
            Some(Value::String(s)) => s.clone(),
            Some(Value::Null) | None => String::new(),
            Some(other) => other.to_string(),
        }
    }

    fn process_tool_start(&mut self, data: &Value) {
        if self.current_tool_call.is_some() {
            self.finalize_tool_call();
        }
        let id = data
            .get("toolUseId")
            .and_then(Value::as_str)
            .map(str::to_string)
            .unwrap_or_else(generate_tool_call_id);
        let name = data
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let arguments = Self::input_to_string(data.get("input"));
        self.current_tool_call = Some(PartialToolCall {
            id,
            name,
            arguments,
        });

        if data.get("stop").is_some() {
            self.finalize_tool_call();
        }
    }

    fn process_tool_input(&mut self, data: &Value) {
        let frag = Self::input_to_string(data.get("input"));
        if let Some(tc) = &mut self.current_tool_call {
            tc.arguments.push_str(&frag);
        }
    }

    fn finalize_tool_call(&mut self) {
        let Some(tc) = self.current_tool_call.take() else {
            return;
        };
        let (arguments, truncation_detected) = normalize_arguments(&tc.arguments);
        self.tool_calls.push(ToolCall {
            id: tc.id,
            name: tc.name,
            arguments,
            truncation_detected,
        });
    }

    /// Finalize and return all deduplicated tool calls.
    pub fn take_tool_calls(&mut self) -> Vec<ToolCall> {
        if self.current_tool_call.is_some() {
            self.finalize_tool_call();
        }
        deduplicate_tool_calls(std::mem::take(&mut self.tool_calls))
    }
}

/// Normalize an arguments string to canonical JSON; returns ("{}", true) when
/// it looks truncated, ("{}", false) on empty, or the reserialized JSON.
fn normalize_arguments(args: &str) -> (String, bool) {
    let trimmed = args.trim();
    if trimmed.is_empty() {
        return ("{}".to_string(), false);
    }
    match serde_json::from_str::<Value>(trimmed) {
        Ok(v) => (
            serde_json::to_string(&v).unwrap_or_else(|_| "{}".into()),
            false,
        ),
        Err(_) => {
            let truncated = diagnose_truncation(trimmed);
            ("{}".to_string(), truncated)
        }
    }
}

fn diagnose_truncation(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    let open_braces = s.matches('{').count();
    let close_braces = s.matches('}').count();
    let open_brackets = s.matches('[').count();
    let close_brackets = s.matches(']').count();

    if s.starts_with('{') && !s.ends_with('}') {
        return true;
    }
    if s.starts_with('[') && !s.ends_with(']') {
        return true;
    }
    if open_braces != close_braces || open_brackets != close_brackets {
        return true;
    }
    // Unclosed string literal: odd count of unescaped quotes.
    let mut quote_count = 0;
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\\' && i + 1 < bytes.len() {
            i += 2;
            continue;
        }
        if bytes[i] == b'"' {
            quote_count += 1;
        }
        i += 1;
    }
    quote_count % 2 != 0
}

/// Parse `[Called func with args: {...}]` text-format tool calls.
pub fn parse_bracket_tool_calls(text: &str) -> Vec<ToolCall> {
    if text.is_empty() || !text.contains("[Called") {
        return Vec::new();
    }
    let re = regex::Regex::new(r"(?i)\[Called\s+(\w+)\s+with\s+args:\s*").unwrap();
    let mut calls = Vec::new();
    for m in re.captures_iter(text) {
        let func_name = m[1].to_string();
        let args_start = m.get(0).unwrap().end();
        let Some(rel) = text[args_start..].find('{') else {
            continue;
        };
        let json_start = args_start + rel;
        let Some(json_end) = find_matching_brace(text, json_start) else {
            continue;
        };
        let json_str = &text[json_start..=json_end];
        if let Ok(v) = serde_json::from_str::<Value>(json_str) {
            calls.push(ToolCall {
                id: generate_tool_call_id(),
                name: func_name,
                arguments: serde_json::to_string(&v).unwrap_or_else(|_| "{}".into()),
                truncation_detected: false,
            });
        }
    }
    calls
}

/// Deduplicate by id (keep non-empty args) then by name+arguments.
pub fn deduplicate_tool_calls(tool_calls: Vec<ToolCall>) -> Vec<ToolCall> {
    use std::collections::HashMap;
    let mut by_id: HashMap<String, ToolCall> = HashMap::new();
    let mut without_id: Vec<ToolCall> = Vec::new();

    for tc in &tool_calls {
        if tc.id.is_empty() {
            continue;
        }
        match by_id.get(&tc.id) {
            None => {
                by_id.insert(tc.id.clone(), tc.clone());
            }
            Some(existing) => {
                let cur = &tc.arguments;
                let ex = &existing.arguments;
                if cur != "{}" && (ex == "{}" || cur.len() > ex.len()) {
                    by_id.insert(tc.id.clone(), tc.clone());
                }
            }
        }
    }
    for tc in &tool_calls {
        if tc.id.is_empty() {
            without_id.push(tc.clone());
        }
    }

    // Preserve original ordering of id'd calls by first appearance.
    let mut ordered_with_id: Vec<ToolCall> = Vec::new();
    let mut seen_ids = std::collections::HashSet::new();
    for tc in &tool_calls {
        if tc.id.is_empty() {
            continue;
        }
        if seen_ids.insert(tc.id.clone()) {
            if let Some(best) = by_id.get(&tc.id) {
                ordered_with_id.push(best.clone());
            }
        }
    }

    let mut seen = std::collections::HashSet::new();
    let mut unique = Vec::new();
    for tc in ordered_with_id.into_iter().chain(without_id.into_iter()) {
        let key = format!("{}-{}", tc.name, tc.arguments);
        if seen.insert(key) {
            unique.push(tc);
        }
    }
    unique
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_nested_braces() {
        assert_eq!(find_matching_brace(r#"{"a": {"b": 1}}"#, 0), Some(14));
        assert_eq!(find_matching_brace(r#"{"a": "{}"}"#, 0), Some(10));
    }

    #[test]
    fn parses_content_events() {
        let mut p = AwsEventStreamParser::new();
        let evs = p.feed(br#"garbage{"content":"hello"}more{"content":"world"}"#);
        let texts: Vec<String> = evs
            .into_iter()
            .filter_map(|e| match e {
                ParsedEvent::Content(c) => Some(c),
                _ => None,
            })
            .collect();
        assert_eq!(texts, vec!["hello", "world"]);
    }

    #[test]
    fn accumulates_tool_call() {
        let mut p = AwsEventStreamParser::new();
        p.feed(br#"{"name":"search","toolUseId":"call_1","input":{}}"#);
        p.feed(br#"{"input":"{\"q\":"}"#);
        p.feed(br#"{"input":"\"rust\"}"}"#);
        p.feed(br#"{"stop":true}"#);
        let tcs = p.take_tool_calls();
        assert_eq!(tcs.len(), 1);
        assert_eq!(tcs[0].name, "search");
        assert_eq!(tcs[0].arguments, r#"{"q":"rust"}"#);
    }

    #[test]
    fn parses_bracket_tool_calls() {
        let calls =
            parse_bracket_tool_calls(r#"[Called get_weather with args: {"city": "London"}]"#);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].name, "get_weather");
    }
}
