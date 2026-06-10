use codux_protocol::REMOTE_PROTOCOL_VERSION;
use codux_remote_transport::{
    LocalMemoryTransportHub, RemoteHostTransportConfig, RemoteTransport, RemoteTransportKind,
    remote_server_url, remote_stun_urls, remote_url,
};
use codux_runtime_core::terminal::terminal_snapshot_payload;
use codux_terminal_core::{TerminalDriver, TerminalLaunchConfig, TerminalSessionHandle};
use codux_terminal_pty::LocalPtyDriver;
use std::{
    env,
    sync::{Arc, Mutex},
    thread, time,
};

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.iter().any(|arg| arg == "--version" || arg == "-V") {
        println!("codux-agent {}", env!("CARGO_PKG_VERSION"));
        println!("protocol {}", REMOTE_PROTOCOL_VERSION);
        return;
    }
    if args.iter().any(|arg| arg == "--pty-smoke") {
        match run_pty_smoke() {
            Ok(output) => {
                println!("{output}");
                return;
            }
            Err(error) => {
                eprintln!("{error}");
                std::process::exit(1);
            }
        }
    }
    if args.iter().any(|arg| arg == "--transport-smoke") {
        match run_transport_smoke() {
            Ok(output) => {
                println!("{output}");
                return;
            }
            Err(error) => {
                eprintln!("{error}");
                std::process::exit(1);
            }
        }
    }

    println!("codux-agent {}", env!("CARGO_PKG_VERSION"));
    println!("protocol {}", REMOTE_PROTOCOL_VERSION);
    println!("usage: codux-agent [--version] [--pty-smoke] [--transport-smoke]");
}

fn run_pty_smoke() -> Result<String, String> {
    let driver = LocalPtyDriver::new();
    let session = driver.create(
        TerminalLaunchConfig {
            command: Some("printf codux-agent-pty-ok".to_string()),
            title: Some("Codux Agent PTY Smoke".to_string()),
            ..Default::default()
        },
        Box::new(|_| true),
    )?;

    let deadline = time::Instant::now() + time::Duration::from_secs(3);
    while time::Instant::now() < deadline {
        let snapshot = session.snapshot();
        if snapshot.contains("codux-agent-pty-ok") {
            let terminal = terminal_snapshot_payload(session.info(), "headless");
            let _ = session.kill();
            return Ok(format!(
                "{snapshot}\nterminal={}",
                terminal["id"].as_str().unwrap_or_default()
            ));
        }
        thread::sleep(time::Duration::from_millis(20));
    }
    let snapshot = session.snapshot();
    let _ = session.kill();
    Err(format!("PTY smoke output not observed: {snapshot:?}"))
}

fn run_transport_smoke() -> Result<String, String> {
    let config = RemoteHostTransportConfig {
        server_url: "https://relay.example".to_string(),
        host_id: "host-smoke".to_string(),
        host_token: "token-smoke".to_string(),
        stun_urls: remote_stun_urls(),
    };
    let relay = remote_server_url(&config.server_url);
    let url = remote_url(
        &relay,
        "/ws/host",
        &[
            ("hostId", config.host_id.as_str()),
            ("token", config.host_token.as_str()),
        ],
        true,
    )?;
    let hub = LocalMemoryTransportHub::new();
    let received = Arc::new(Mutex::new(Vec::<String>::new()));
    let host = hub.connect(
        "host-smoke",
        RemoteTransportKind::WebSocketRelay,
        {
            let received = Arc::clone(&received);
            Arc::new(move |source, data| {
                let text = String::from_utf8(data).unwrap_or_default();
                received.lock().unwrap().push(format!("{source}:{text}"));
            })
        },
        Arc::new(|_, _| {}),
    );
    let controller = hub.connect(
        "device-smoke",
        RemoteTransportKind::WebSocketRelay,
        Arc::new(|_, _| {}),
        Arc::new(|_, _| {}),
    );
    if !controller.send(b"codux-agent-transport-ok".to_vec(), Some(host.id())) {
        return Err("local memory transport send failed".to_string());
    }
    let observed = received
        .lock()
        .unwrap()
        .iter()
        .any(|line| line == "device-smoke:codux-agent-transport-ok");
    if !observed {
        return Err("local memory transport message not observed".to_string());
    }
    Ok(format!(
        "codux-agent-transport-ok\nrelay={relay}\nurl={url}\nstun={}\nlocal=ok",
        config.stun_urls.len()
    ))
}
