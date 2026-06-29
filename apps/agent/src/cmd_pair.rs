//! `codux link` and `codux qrcode` — surface the pairing ticket the running host
//! publishes. If the host isn't up yet, start it in the background first. The
//! headless host auto-confirms pairing (holding the one-time ticket is the gate),
//! so no second confirmation is needed.

use qrcode::QrCode;
use qrcode::render::unicode;
use std::time::Duration;

use crate::{cmd_start, runstate};

/// Print the pasteable pairing ticket for the desktop to connect.
pub fn link() -> Result<(), String> {
    let ticket = ensure_ticket()?;
    println!("{ticket}");
    Ok(())
}

/// Render the pairing QR code in the terminal, with the host status above it.
pub fn qrcode() -> Result<(), String> {
    let ticket = ensure_ticket()?;
    if let Some(status) = runstate::read_status() {
        println!(
            "Host running · started {} · device “{}”",
            status.started_at, status.device_name
        );
    }
    println!();
    let code = QrCode::new(ticket.as_bytes()).map_err(|error| error.to_string())?;
    let rendered = code.render::<unicode::Dense1x2>().quiet_zone(true).build();
    println!("{rendered}");
    println!("Scan with the Codux mobile app, or paste the link below on desktop:");
    println!("{ticket}");
    Ok(())
}

/// Return the published pairing ticket, starting the host in the background if
/// it isn't already running.
fn ensure_ticket() -> Result<String, String> {
    if !runstate::is_running() {
        println!("Codux host is not running — starting it…");
        cmd_start::run(true)?;
    }
    for _ in 0..40 {
        if let Some(ticket) = runstate::read_ticket() {
            return Ok(ticket);
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    Err("the host is up but has not published a pairing ticket yet".to_string())
}
