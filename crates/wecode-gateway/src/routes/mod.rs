mod anthropic;
mod openai;

use std::sync::Arc;

use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::json;

use crate::accounts::AccountManager;
use crate::config::GatewayConfig;
use crate::model_catalog::GatewayModelCatalog;

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<GatewayConfig>,
    pub model_catalog: Arc<GatewayModelCatalog>,
    pub accounts: Arc<AccountManager>,
    pub truncation: Arc<crate::truncation::TruncationStore>,
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/", get(root))
        .route("/health", get(health))
        .route("/v1/models", get(models))
        .route("/v1/chat/completions", post(openai::chat_completions))
        .route("/v1/messages", post(anthropic::messages))
        .route("/v1/messages/count_tokens", post(anthropic::count_tokens))
        .with_state(state)
}

async fn root() -> impl IntoResponse {
    Json(json!({ "status": "ok", "service": "wecode-gateway" }))
}

async fn health() -> impl IntoResponse {
    Json(json!({ "status": "healthy" }))
}

/// Verify the client API key from `Authorization: Bearer` or `x-api-key`.
pub fn verify_api_key(headers: &HeaderMap, config: &GatewayConfig) -> Result<(), Response> {
    let expected = &config.api_key;
    if let Some(auth) = headers.get("authorization").and_then(|v| v.to_str().ok()) {
        if auth == format!("Bearer {expected}") {
            return Ok(());
        }
    }
    if let Some(key) = headers.get("x-api-key").and_then(|v| v.to_str().ok()) {
        if key == expected {
            return Ok(());
        }
    }
    Err((
        StatusCode::UNAUTHORIZED,
        Json(json!({
            "error": {
                "type": "authentication_error",
                "message": "Invalid or missing API key. Use x-api-key header or Authorization: Bearer."
            }
        })),
    )
        .into_response())
}

async fn models(State(state): State<AppState>, headers: HeaderMap) -> Response {
    if let Err(resp) = verify_api_key(&headers, &state.config) {
        return resp;
    }
    let mut models = state.model_catalog.models.clone();
    models.retain(|model| !state.config.hidden_from_list.contains(&model.id));
    let created = chrono::Utc::now().timestamp();
    let data: Vec<_> = models
        .into_iter()
        .map(|model| {
            json!({
                "id": model.id,
                "object": "model",
                "created": created,
                "owned_by": model.owned_by,
                "context_window_tokens": model.context_window_tokens,
                "rate_multiplier": model.rate_multiplier,
                "compatibility": model.compatibility,
            })
        })
        .collect();
    Json(json!({ "object": "list", "data": data })).into_response()
}

/// Save any detected truncations from a collected result (non-streaming).
pub fn save_truncations(
    result: &crate::upstream::StreamResult,
    store: &crate::truncation::TruncationStore,
) {
    for tc in &result.tool_calls {
        if tc.truncation_detected {
            store.save_tool(&tc.id);
        }
    }
    // Content truncation: stream ended without a completion signal, has content,
    // and produced no tool calls.
    if result.context_usage_percentage.is_none()
        && !result.content.is_empty()
        && result.tool_calls.is_empty()
    {
        store.save_content(&result.content);
    }
}

/// Shared token-from-context-usage calculation.
pub fn tokens_from_context_usage(
    context_usage_percentage: Option<f64>,
    completion_tokens: u64,
    max_input_tokens: u64,
) -> Option<u64> {
    match context_usage_percentage {
        Some(p) if p > 0.0 => {
            let total = ((p / 100.0) * max_input_tokens as f64) as u64;
            Some(total.saturating_sub(completion_tokens))
        }
        _ => None,
    }
}
