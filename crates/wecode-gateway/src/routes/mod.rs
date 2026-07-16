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
use crate::model_catalog::GatewayModel;
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
        .route("/v1/responses", post(openai::responses))
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
        .iter()
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
    // Codex custom providers fetch the same `/models` endpoint but use the
    // Codex model-catalog schema. Returning both wrappers keeps this endpoint
    // compatible with ordinary OpenAI clients and Codex CLI.
    let codex_models: Vec<_> = models
        .iter()
        .filter(|model| model.compatibility.codex_cli)
        .enumerate()
        .map(|(index, model)| codex_model_info(model, index))
        .collect();
    Json(json!({ "object": "list", "data": data, "models": codex_models })).into_response()
}

fn codex_model_info(model: &GatewayModel, index: usize) -> serde_json::Value {
    let context_window = (model.context_window_tokens > 0)
        .then_some(model.context_window_tokens)
        .unwrap_or(272_000);
    json!({
        "slug": model.id,
        "display_name": model.name,
        "description": model.description,
        "default_reasoning_level": "medium",
        "supported_reasoning_levels": [
            { "effort": "low", "description": "Faster responses" },
            { "effort": "medium", "description": "Balanced reasoning" },
            { "effort": "high", "description": "Deeper reasoning" }
        ],
        "shell_type": "unified_exec",
        "visibility": "list",
        "supported_in_api": true,
        "priority": 100 - index as i32,
        "availability_nux": null,
        "upgrade": null,
        "base_instructions": crate::convert::openai::CODEX_AGENT_BASE_INSTRUCTIONS,
        "support_verbosity": false,
        "default_verbosity": null,
        "apply_patch_tool_type": null,
        "truncation_policy": { "mode": "tokens", "limit": context_window },
        "supports_parallel_tool_calls": true,
        "supports_reasoning_summaries": true,
        "supports_image_detail_original": false,
        "context_window": context_window,
        "max_context_window": context_window,
        "experimental_supported_tools": [],
        "input_modalities": ["text"],
        "use_responses_lite": false
    })
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codex_model_catalog_entry_selects_function_based_unified_exec() {
        let catalog = GatewayModelCatalog::fallback();
        let model = catalog.model("gpt-5.6-luna").unwrap();
        let info = codex_model_info(model, 0);

        assert_eq!(info["slug"], "gpt-5.6-luna");
        assert_eq!(info["shell_type"], "unified_exec");
        assert_eq!(info["supports_reasoning_summaries"], true);
        assert!(info["base_instructions"]
            .as_str()
            .unwrap()
            .contains("You are Codex"));
        assert!(info["base_instructions"]
            .as_str()
            .unwrap()
            .contains("Kiro is only the model provider"));
        assert_eq!(info["use_responses_lite"], false);
        assert_eq!(info["context_window"], 272_000);
    }
}
