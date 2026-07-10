//! Standalone codux-gateway binary. Also embeddable via the library `start()`.

use std::path::PathBuf;

use codux_gateway::{start, GatewayConfig};

fn parse_args() -> (Option<PathBuf>, Option<String>, Option<u16>) {
    let mut config_path = None;
    let mut host = None;
    let mut port = None;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--config" | "-c" => config_path = args.next().map(PathBuf::from),
            "--host" => host = args.next(),
            "--port" | "-p" => port = args.next().and_then(|p| p.parse().ok()),
            "--help" | "-h" => {
                println!("Usage: codux-gateway [--config <file>] [--host <host>] [--port <port>]");
                std::process::exit(0);
            }
            _ => {}
        }
    }
    (config_path, host, port)
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "codux_gateway=info,tower_http=info".into()),
        )
        .init();

    let (config_path, host, port) = parse_args();

    let mut config = match config_path {
        Some(path) => match GatewayConfig::load_from_file(&path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("failed to load config: {e}");
                std::process::exit(1);
            }
        },
        None => GatewayConfig::default(),
    };
    if let Some(h) = host {
        config.host = h;
    }
    if let Some(p) = port {
        config.port = p;
    }

    let handle = match start(config).await {
        Ok(h) => h,
        Err(e) => {
            eprintln!("failed to start gateway: {e}");
            std::process::exit(1);
        }
    };

    // Run until Ctrl-C.
    let _ = tokio::signal::ctrl_c().await;
    tracing::info!("shutting down");
    handle.shutdown().await;
}
