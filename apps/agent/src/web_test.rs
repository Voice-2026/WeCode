//! Built-in host-local HTTP page for validating WeCode Web Tunnel.

use std::{
    io::{Read, Write},
    net::{SocketAddr, TcpListener, TcpStream},
    sync::OnceLock,
    thread,
    time::{Duration, Instant, SystemTime},
};

pub const DEFAULT_BIND: &str = "127.0.0.1:8765";

static STARTED_AT: OnceLock<Instant> = OnceLock::new();

pub struct WebTestServer {
    pub address: SocketAddr,
}

pub fn start_background() -> Result<WebTestServer, String> {
    let listener = TcpListener::bind(DEFAULT_BIND).map_err(|error| {
        format!("failed to bind built-in web test page at {DEFAULT_BIND}: {error}")
    })?;
    let address = listener
        .local_addr()
        .map_err(|error| format!("failed to read web test listener address: {error}"))?;
    STARTED_AT.get_or_init(Instant::now);
    thread::Builder::new()
        .name("wecode-web-test".to_string())
        .spawn(move || serve(listener, address.port()))
        .map_err(|error| format!("failed to start web test thread: {error}"))?;
    Ok(WebTestServer { address })
}

fn serve(listener: TcpListener, port: u16) {
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                if let Err(error) = handle_stream(stream, port) {
                    eprintln!("web test request failed: {error}");
                }
            }
            Err(error) => eprintln!("web test accept failed: {error}"),
        }
    }
}

fn handle_stream(mut stream: TcpStream, port: u16) -> Result<(), String> {
    let mut buffer = [0u8; 2048];
    let read = stream
        .read(&mut buffer)
        .map_err(|error| format!("failed to read request: {error}"))?;
    let request = String::from_utf8_lossy(&buffer[..read]);
    let first_line = request.lines().next().unwrap_or_default();
    if first_line.starts_with("GET /health") {
        write_response(&mut stream, "text/plain; charset=utf-8", "ok\n")?;
        return Ok(());
    }
    if first_line.starts_with("GET /ping") {
        write_response(
            &mut stream,
            "application/json; charset=utf-8",
            &ping_payload(),
        )?;
        return Ok(());
    }
    if !first_line.starts_with("GET ") {
        write_status(
            &mut stream,
            405,
            "Method Not Allowed",
            "text/plain; charset=utf-8",
            "method not allowed\n",
        )?;
        return Ok(());
    }
    write_response(&mut stream, "text/html; charset=utf-8", &test_page(port))?;
    Ok(())
}

fn write_response(stream: &mut TcpStream, content_type: &str, body: &str) -> Result<(), String> {
    write_status(stream, 200, "OK", content_type, body)
}

fn write_status(
    stream: &mut TcpStream,
    status: u16,
    reason: &str,
    content_type: &str,
    body: &str,
) -> Result<(), String> {
    let head = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n",
        body.len()
    );
    stream
        .write_all(head.as_bytes())
        .and_then(|_| stream.write_all(body.as_bytes()))
        .map_err(|error| format!("failed to write response: {error}"))
}

fn ping_payload() -> String {
    let now_ms = unix_millis();
    let uptime_ms = STARTED_AT
        .get()
        .map(|started| started.elapsed().as_millis())
        .unwrap_or_default();
    format!(r#"{{"ok":true,"serverTimeMs":{now_ms},"uptimeMs":{uptime_ms},"agent":"wecode"}}"#)
}

fn unix_millis() -> u128 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_millis()
}

fn test_page(port: u16) -> String {
    let now = unix_millis();
    format!(
        r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>WeCode Web Tunnel Test</title>
  <style>
    :root {{ color-scheme: dark; }}
    * {{ box-sizing: border-box; }}
    body {{
      margin: 0;
      min-height: 100vh;
      display: grid;
      place-items: center;
      font: 14px/1.5 system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
      background: #101214;
      color: #f4f7fb;
    }}
    main {{
      width: min(680px, calc(100vw - 40px));
      border: 1px solid rgba(255, 255, 255, .12);
      border-radius: 18px;
      background: rgba(255, 255, 255, .06);
      padding: 28px;
      box-shadow: 0 24px 80px rgba(0, 0, 0, .32);
    }}
    h1 {{ margin: 0 0 12px; font-size: 24px; line-height: 1.2; }}
    p {{ margin: 8px 0; color: rgba(244, 247, 251, .76); }}
    code {{
      display: inline-block;
      padding: 2px 7px;
      border-radius: 7px;
      background: rgba(72, 135, 255, .16);
      color: #8bb7ff;
    }}
    .ok {{ color: #67e8a5; font-weight: 700; }}
    .grid {{ display: grid; grid-template-columns: repeat(3, minmax(0, 1fr)); gap: 10px; margin: 18px 0; }}
    .metric {{ border: 1px solid rgba(255,255,255,.10); border-radius: 12px; padding: 12px; background: rgba(0,0,0,.16); }}
    .label {{ color: rgba(244,247,251,.56); font-size: 12px; }}
    .value {{ margin-top: 4px; font-size: 20px; font-weight: 700; }}
    button {{
      margin-top: 10px;
      border: 0;
      border-radius: 10px;
      padding: 9px 13px;
      background: #4f8cff;
      color: white;
      font-weight: 700;
      cursor: pointer;
    }}
    @media (max-width: 560px) {{ .grid {{ grid-template-columns: 1fr; }} }}
  </style>
</head>
<body>
  <main>
    <h1><span class="ok">OK</span> WeCode Web Tunnel works</h1>
    <p>This diagnostic page is served by the running WeCode host.</p>
    <p>Open it through WeCode Desktop's Web Tunnel Browser at <code>http://127.0.0.1:{port}/</code>.</p>
    <div class="grid">
      <div class="metric"><div class="label">Last round trip</div><div class="value" id="rtt">—</div></div>
      <div class="metric"><div class="label">Server clock drift</div><div class="value" id="drift">—</div></div>
      <div class="metric"><div class="label">Agent uptime</div><div class="value" id="uptime">—</div></div>
    </div>
    <button id="run">Run latency check</button>
    <p>Generated at <code>{now}</code> ms since Unix epoch.</p>
  </main>
  <script>
    const rtt = document.getElementById('rtt');
    const drift = document.getElementById('drift');
    const uptime = document.getElementById('uptime');
    async function ping() {{
      const start = performance.now();
      const response = await fetch('/ping?ts=' + Date.now(), {{ cache: 'no-store' }});
      const data = await response.json();
      const elapsed = performance.now() - start;
      const clientNow = Date.now();
      rtt.textContent = Math.round(elapsed) + ' ms';
      drift.textContent = Math.round(data.serverTimeMs - clientNow) + ' ms';
      uptime.textContent = Math.round(data.uptimeMs / 1000) + ' s';
    }}
    document.getElementById('run').addEventListener('click', ping);
    ping();
    setInterval(ping, 3000);
  </script>
</body>
</html>
"#
    )
}
