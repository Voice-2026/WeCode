//! Minimal live probe for the WeChat iLink Bot protocol.
//!
//! Verifies the two pre-auth steps really work end to end against
//! `ilinkai.weixin.qq.com`:
//!   1. `get_bot_qrcode` returns a QR handle + scan URL.
//!   2. `get_qrcode_status` transitions wait -> scaned -> confirmed.
//!
//! Run: `cargo run -p codux-im-bridge --bin wechat-probe`
//! Then scan the printed URL with WeChat. Ctrl-C to stop.

use codux_im_bridge::wechat::{ILinkClient, QrScanOutcome, DEFAULT_BASE_URL};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let http = reqwest::Client::new();

    println!("[probe] fetching QR code from {DEFAULT_BASE_URL} ...");
    let qr = ILinkClient::fetch_qr_code(&http, DEFAULT_BASE_URL).await?;
    println!("[probe] qrcode handle: {}", qr.qrcode);
    println!(
        "[probe] scan this URL with WeChat:\n  {}",
        qr.qrcode_img_content
    );
    println!("[probe] polling scan status (Ctrl-C to stop) ...");

    let mut last = String::new();
    loop {
        match ILinkClient::poll_qr_status(&http, DEFAULT_BASE_URL, &qr.qrcode).await {
            Ok(QrScanOutcome::Waiting) => note(&mut last, "waiting"),
            Ok(QrScanOutcome::Scanned) => note(&mut last, "scanned"),
            Ok(QrScanOutcome::Expired) => {
                println!("[probe] QR expired; re-run to get a fresh code");
                break;
            }
            Ok(QrScanOutcome::Confirmed(creds)) => {
                println!("[probe] CONFIRMED. bot_id={}", creds.ilink_bot_id);
                println!("[probe] protocol verified end to end");
                break;
            }
            Err(e) => {
                eprintln!("[probe] poll error: {e}");
                break;
            }
        }
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    }
    Ok(())
}

fn note(last: &mut String, state: &str) {
    if last != state {
        println!("[probe] status: {state}");
        *last = state.to_string();
    }
}
