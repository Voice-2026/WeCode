use crate::{
    config::ServerConfig,
    store::{Host, Store},
};
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use codux_protocol::REMOTE_PROTOCOL_VERSION;
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use tower_http::cors::CorsLayer;

#[derive(Debug)]
pub struct Hub {
    store: Mutex<Store>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RegisterHostRequest {
    host_id: Option<String>,
    name: Option<String>,
    token: Option<String>,
    public_key: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RevokeDeviceRequest {
    host_id: String,
    token: String,
    device_id: String,
}

impl Hub {
    pub fn open(config: ServerConfig) -> anyhow::Result<Self> {
        let store = Store::open(&config.db_path)?;
        Ok(Self {
            store: Mutex::new(store),
        })
    }

    pub fn router(self: Arc<Self>) -> Router {
        Router::new()
            .route("/healthz", get(health))
            .route("/api/hosts/register", post(register_host))
            .route("/api/hosts/{host_id}/devices", get(list_devices))
            .route("/api/devices/revoke", post(revoke_device))
            .route("/v3/healthz", get(v3_health))
            .layer(CorsLayer::permissive())
            .with_state(self)
    }

    fn with_store<T>(&self, f: impl FnOnce(&mut Store) -> anyhow::Result<T>) -> anyhow::Result<T> {
        let mut store = self.store.lock().expect("store lock");
        f(&mut store)
    }

    fn authenticate_host(&self, host_id: &str, token: &str) -> Option<Host> {
        if host_id.trim().is_empty() || token.trim().is_empty() {
            return None;
        }
        self.with_store(|store| Ok(store.host_by_token(token)?))
            .ok()
            .filter(|host| host.id == host_id)
    }
}

async fn health() -> impl IntoResponse {
    Json(json!({ "ok": true }))
}

async fn v3_health() -> impl IntoResponse {
    Json(json!({ "ok": true, "protocolVersion": REMOTE_PROTOCOL_VERSION }))
}

async fn register_host(
    State(hub): State<Arc<Hub>>,
    Json(request): Json<RegisterHostRequest>,
) -> Response {
    match hub.with_store(|store| {
        Ok(store.upsert_host(
            request.host_id,
            request.name,
            request.token,
            request.public_key,
        )?)
    }) {
        Ok(host) => json_ok(json!({ "hostId": host.id, "token": host.token })),
        Err(error) => json_error(StatusCode::INTERNAL_SERVER_ERROR, error.to_string()),
    }
}

async fn list_devices(
    State(hub): State<Arc<Hub>>,
    Path(host_id): Path<String>,
    Query(query): Query<HashMap<String, String>>,
    headers: HeaderMap,
) -> Response {
    let token = query
        .get("token")
        .cloned()
        .or_else(|| bearer_token(&headers))
        .unwrap_or_default();
    if hub.authenticate_host(&host_id, &token).is_none() {
        return json_error(StatusCode::UNAUTHORIZED, "invalid host token");
    }
    match hub.with_store(|store| Ok(store.devices_for_host(&host_id)?)) {
        Ok(devices) => json_ok(json!({ "devices": devices })),
        Err(error) => json_error(StatusCode::INTERNAL_SERVER_ERROR, error.to_string()),
    }
}

async fn revoke_device(
    State(hub): State<Arc<Hub>>,
    Json(request): Json<RevokeDeviceRequest>,
) -> Response {
    if hub
        .authenticate_host(&request.host_id, &request.token)
        .is_none()
    {
        return json_error(StatusCode::UNAUTHORIZED, "invalid host token");
    }
    match hub.with_store(|store| Ok(store.revoke_device(&request.host_id, &request.device_id)?)) {
        Ok(()) | Err(_) => json_ok(json!({ "ok": true })),
    }
}

fn bearer_token(headers: &HeaderMap) -> Option<String> {
    headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .map(ToOwned::to_owned)
}

fn json_ok(value: Value) -> Response {
    (StatusCode::OK, Json(value)).into_response()
}

fn json_error(status: StatusCode, message: impl Into<String>) -> Response {
    (status, Json(json!({ "error": message.into() }))).into_response()
}
