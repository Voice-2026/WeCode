pub mod event_stream;

use std::sync::Arc;
use std::time::Duration;

use async_stream::stream;
use futures::{Stream, StreamExt};
use serde_json::Value;
use tokio::time::timeout;

use crate::auth::KiroAuth;
use crate::config::GatewayConfig;
use crate::error::GatewayError;
use crate::upstream::event_stream::{
    deduplicate_tool_calls, parse_bracket_tool_calls, AwsEventStreamParser, ParsedEvent, ToolCall,
};
use crate::util::kiro_headers;

/// Unified event from the Kiro stream.
#[derive(Debug, Clone)]
pub enum KiroEvent {
    Content(String),
    Thinking {
        content: String,
        is_first: bool,
        is_last: bool,
    },
    ToolUse(ToolCall),
    Usage(Value),
    ContextUsage(f64),
}

/// Fully-collected stream result (non-streaming mode).
#[derive(Debug, Default)]
pub struct StreamResult {
    pub content: String,
    pub thinking_content: String,
    pub tool_calls: Vec<ToolCall>,
    pub usage: Option<Value>,
    pub context_usage_percentage: Option<f64>,
}

/// POST generateAssistantResponse with retry (403→refresh, 429/5xx→backoff).
/// Returns the streaming response once a 200 is received.
pub async fn request_kiro(
    auth: &Arc<KiroAuth>,
    payload: &Value,
    config: &GatewayConfig,
) -> Result<reqwest::Response, GatewayError> {
    let url = format!("{}/generateAssistantResponse", auth.api_host());
    let body = serde_json::to_vec(payload)
        .map_err(|e| GatewayError::Internal(format!("failed to serialize payload: {e}")))?;

    let max_retries = config.first_token_max_retries.max(1);
    let mut last_status = None;
    let mut last_body = String::new();

    for attempt in 0..max_retries {
        let token = auth.get_access_token().await?;
        let mut req = auth
            .http()
            .post(&url)
            .body(body.clone())
            .header("Connection", "close");
        for (k, v) in kiro_headers(&token, auth.fingerprint()) {
            req = req.header(k, v);
        }

        let resp = match req.send().await {
            Ok(r) => r,
            Err(e) => {
                let delay = config.base_retry_delay_secs * 2f64.powi(attempt as i32);
                tracing::warn!("request error: {e}, retry in {delay}s");
                tokio::time::sleep(Duration::from_secs_f64(delay)).await;
                last_body = e.to_string();
                continue;
            }
        };

        let status = resp.status();
        if status.is_success() {
            return Ok(resp);
        }
        if status.as_u16() == 403 {
            tracing::warn!("received 403, forcing token refresh");
            auth.force_refresh().await?;
            continue;
        }
        if status.as_u16() == 429 || status.is_server_error() {
            last_status = Some(status.as_u16());
            last_body = resp.text().await.unwrap_or_default();
            let delay = config.base_retry_delay_secs * 2f64.powi(attempt as i32);
            tracing::warn!("received {}, waiting {delay}s", status.as_u16());
            tokio::time::sleep(Duration::from_secs_f64(delay)).await;
            continue;
        }
        // Other error: return to caller.
        let s = status.as_u16();
        let body = resp.text().await.unwrap_or_default();
        return Err(GatewayError::Upstream { status: s, body });
    }

    Err(GatewayError::Upstream {
        status: last_status.unwrap_or(502),
        body: if last_body.is_empty() {
            "request failed after retries".into()
        } else {
            last_body
        },
    })
}

/// Turn a Kiro streaming response into a stream of unified events.
/// Applies a first-token timeout and a per-chunk read timeout.
pub fn kiro_event_stream(
    resp: reqwest::Response,
    first_token_timeout_secs: f64,
    read_timeout_secs: f64,
    thinking_handling: Option<String>,
) -> impl Stream<Item = Result<KiroEvent, GatewayError>> {
    stream! {
        let mut parser = AwsEventStreamParser::new();
        let mut thinking = thinking_handling
            .map(|mode| crate::thinking::ThinkingParser::new(&mode));
        let mut bytes = resp.bytes_stream();
        let mut first = true;

        loop {
            let to = if first { first_token_timeout_secs } else { read_timeout_secs };
            let next = timeout(Duration::from_secs_f64(to), bytes.next()).await;
            let chunk = match next {
                Err(_) if first => {
                    yield Err(GatewayError::FirstTokenTimeout {
                        timeout_secs: first_token_timeout_secs,
                        attempts: 1,
                    });
                    return;
                }
                Err(_) => {
                    yield Err(GatewayError::Network("read timeout between chunks".into()));
                    return;
                }
                Ok(None) => break,
                Ok(Some(Err(e))) => {
                    yield Err(GatewayError::Network(format!("stream error: {e}")));
                    return;
                }
                Ok(Some(Ok(c))) => c,
            };
            first = false;

            for ev in parser.feed(&chunk) {
                match ev {
                    ParsedEvent::Content(c) => {
                        if let Some(tp) = thinking.as_mut() {
                            let r = tp.feed(&c);
                            if let Some(tc) = r.thinking_content {
                                if let Some(out) = tp.process_for_output(&tc, r.is_first_thinking_chunk, r.is_last_thinking_chunk) {
                                    yield Ok(KiroEvent::Thinking {
                                        content: out,
                                        is_first: r.is_first_thinking_chunk,
                                        is_last: r.is_last_thinking_chunk,
                                    });
                                }
                            }
                            if let Some(reg) = r.regular_content {
                                yield Ok(KiroEvent::Content(reg));
                            }
                        } else {
                            yield Ok(KiroEvent::Content(c));
                        }
                    }
                    ParsedEvent::Usage(u) => yield Ok(KiroEvent::Usage(u)),
                    ParsedEvent::ContextUsage(p) => yield Ok(KiroEvent::ContextUsage(p)),
                }
            }
        }

        if let Some(tp) = thinking.as_mut() {
            let r = tp.finalize();
            if let Some(tc) = r.thinking_content {
                if let Some(out) = tp.process_for_output(&tc, r.is_first_thinking_chunk, r.is_last_thinking_chunk) {
                    yield Ok(KiroEvent::Thinking {
                        content: out,
                        is_first: r.is_first_thinking_chunk,
                        is_last: r.is_last_thinking_chunk,
                    });
                }
            }
            if let Some(reg) = r.regular_content {
                yield Ok(KiroEvent::Content(reg));
            }
        }

        for tc in parser.take_tool_calls() {
            yield Ok(KiroEvent::ToolUse(tc));
        }
    }
}

/// Collect the full stream (non-streaming mode).
pub async fn collect_stream(
    resp: reqwest::Response,
    first_token_timeout_secs: f64,
    read_timeout_secs: f64,
    thinking_handling: Option<String>,
) -> Result<StreamResult, GatewayError> {
    let mut result = StreamResult::default();
    let events = kiro_event_stream(
        resp,
        first_token_timeout_secs,
        read_timeout_secs,
        thinking_handling,
    );
    futures::pin_mut!(events);
    while let Some(ev) = events.next().await {
        match ev? {
            KiroEvent::Content(c) => result.content.push_str(&c),
            KiroEvent::Thinking { content, .. } => result.thinking_content.push_str(&content),
            KiroEvent::ToolUse(tc) => result.tool_calls.push(tc),
            KiroEvent::Usage(u) => result.usage = Some(u),
            KiroEvent::ContextUsage(p) => result.context_usage_percentage = Some(p),
        }
    }
    // Bracket-style tool calls in the accumulated content.
    let bracket = parse_bracket_tool_calls(&result.content);
    if !bracket.is_empty() {
        let mut all = std::mem::take(&mut result.tool_calls);
        all.extend(bracket);
        result.tool_calls = deduplicate_tool_calls(all);
    }
    Ok(result)
}
