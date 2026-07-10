use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;

#[derive(Debug, thiserror::Error)]
pub enum GatewayError {
    #[error("authentication failed: {0}")]
    Auth(String),

    #[error("upstream API error ({status}): {body}")]
    Upstream { status: u16, body: String },

    #[error("network error: {0}")]
    Network(String),

    #[error("first token timeout: model did not respond within {timeout_secs}s after {attempts} attempts")]
    FirstTokenTimeout { timeout_secs: f64, attempts: u32 },

    #[error("invalid request: {0}")]
    InvalidRequest(String),

    #[error("{0}")]
    Internal(String),
}

impl GatewayError {
    pub fn status_code(&self) -> StatusCode {
        match self {
            GatewayError::Auth(_) => StatusCode::UNAUTHORIZED,
            GatewayError::Upstream { status, .. } => {
                StatusCode::from_u16(*status).unwrap_or(StatusCode::BAD_GATEWAY)
            }
            GatewayError::Network(_) => StatusCode::BAD_GATEWAY,
            GatewayError::FirstTokenTimeout { .. } => StatusCode::GATEWAY_TIMEOUT,
            GatewayError::InvalidRequest(_) => StatusCode::BAD_REQUEST,
            GatewayError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    /// OpenAI-style error body.
    pub fn to_openai_json(&self) -> serde_json::Value {
        json!({
            "error": {
                "message": self.to_string(),
                "type": "api_error",
                "code": self.status_code().as_u16(),
            }
        })
    }

    /// Anthropic-style error body.
    pub fn to_anthropic_json(&self) -> serde_json::Value {
        let err_type = match self {
            GatewayError::Auth(_) => "authentication_error",
            GatewayError::InvalidRequest(_) => "invalid_request_error",
            GatewayError::FirstTokenTimeout { .. } => "timeout_error",
            _ => "api_error",
        };
        json!({
            "type": "error",
            "error": { "type": err_type, "message": self.to_string() }
        })
    }
}

impl IntoResponse for GatewayError {
    fn into_response(self) -> Response {
        (self.status_code(), Json(self.to_openai_json())).into_response()
    }
}
