//! WeCode Gateway — OpenAI/Anthropic-compatible API backed by Kiro
//! (Amazon Q Developer / AWS CodeWhisperer). Rust rewrite of kiro-gateway.

pub mod accounts;
pub mod auth;
pub mod config;
pub mod convert;
pub mod error;
pub mod mcp;
pub mod model_resolver;
pub mod routes;
pub mod thinking;
pub mod tokens;
pub mod truncation;
pub mod upstream;
pub mod util;

use std::sync::Arc;

use tokio::sync::oneshot;

pub use config::GatewayConfig;
pub use error::GatewayError;

/// A running gateway server; drop or call `shutdown()` to stop it.
pub struct GatewayHandle {
    addr: std::net::SocketAddr,
    shutdown: Option<oneshot::Sender<()>>,
    join: tokio::task::JoinHandle<()>,
}

impl GatewayHandle {
    pub fn local_addr(&self) -> std::net::SocketAddr {
        self.addr
    }

    /// Signal shutdown and wait for the server task to finish.
    pub async fn shutdown(mut self) {
        if let Some(tx) = self.shutdown.take() {
            let _ = tx.send(());
        }
        let _ = self.join.await;
    }
}

fn build_http_client() -> reqwest::Client {
    reqwest::Client::builder()
        .build()
        .expect("failed to build HTTP client")
}

/// Start the gateway server bound to the configured host/port.
pub async fn start(config: GatewayConfig) -> Result<GatewayHandle, GatewayError> {
    let config = Arc::new(config);
    let http = build_http_client();
    let accounts = accounts::AccountManager::from_config(&config, http)?;

    let state = routes::AppState {
        config: config.clone(),
        accounts,
        truncation: Arc::new(truncation::TruncationStore::default()),
    };
    let app = routes::router(state);

    let addr = format!("{}:{}", config.host, config.port);
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .map_err(|e| GatewayError::Internal(format!("failed to bind {addr}: {e}")))?;
    let local_addr = listener
        .local_addr()
        .map_err(|e| GatewayError::Internal(format!("failed to read local addr: {e}")))?;

    let (tx, rx) = oneshot::channel();
    let join = tokio::spawn(async move {
        let server = axum::serve(listener, app).with_graceful_shutdown(async {
            let _ = rx.await;
        });
        if let Err(e) = server.await {
            tracing::error!("gateway server error: {e}");
        }
    });

    tracing::info!("wecode-gateway listening on http://{local_addr}");

    Ok(GatewayHandle {
        addr: local_addr,
        shutdown: Some(tx),
        join,
    })
}
