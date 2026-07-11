//! `wecode device` / `device:del` / `device:rename` / `device:clear`.

use dialoguer::theme::ColorfulTheme;
use dialoguer::{Confirm, Input};

use crate::device_store;

pub fn list() -> Result<(), String> {
    let devices = device_store::list();
    if devices.is_empty() {
        println!("No paired devices. Run `wecode qrcode` to pair one.");
        return Ok(());
    }
    let rows: Vec<[String; 4]> = devices
        .iter()
        .map(|device| {
            [
                device.id.clone(),
                empty_dash(&device.name),
                device_type_label(&device.platform),
                short_time(&device.last_seen),
            ]
        })
        .collect();
    print_table(["DEVICE ID", "NAME", "TYPE", "LAST SEEN"], &rows);
    Ok(())
}

pub fn del(id: &str) -> Result<(), String> {
    if device_store::remove(id)? {
        println!("Removed device {id}.");
    } else {
        println!("No device with id {id}.");
    }
    Ok(())
}

pub fn rename(id: &str) -> Result<(), String> {
    let current = device_store::list()
        .into_iter()
        .find(|device| device.id == id);
    let Some(current) = current else {
        return Err(format!("no device with id {id}"));
    };
    let name: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("New device name")
        .with_initial_text(current.name.clone())
        .interact_text()
        .map_err(|error| error.to_string())?;
    let name = name.trim();
    if name.is_empty() {
        return Err("device name cannot be empty".to_string());
    }
    device_store::rename(id, name)?;
    println!("Renamed {id} to “{name}”.");
    Ok(())
}

pub fn clear() -> Result<(), String> {
    let count = device_store::list().len();
    if count == 0 {
        println!("No paired devices.");
        return Ok(());
    }
    let confirmed = Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt(format!("Remove all {count} paired device(s)?"))
        .default(false)
        .interact()
        .map_err(|error| error.to_string())?;
    if !confirmed {
        println!("Cancelled.");
        return Ok(());
    }
    let removed = device_store::clear()?;
    println!("Cleared {removed} device(s).");
    Ok(())
}

fn empty_dash(value: &str) -> String {
    if value.trim().is_empty() {
        "—".to_string()
    } else {
        value.to_string()
    }
}

/// Friendly OS label from a reported platform id.
pub fn device_type_label(platform: &str) -> String {
    match platform.trim().to_ascii_lowercase().as_str() {
        "macos" | "darwin" | "mac" => "macOS".to_string(),
        "ios" | "ipados" => "iOS".to_string(),
        "android" => "Android".to_string(),
        "linux" => "Linux".to_string(),
        "windows" => "Windows".to_string(),
        "" => "—".to_string(),
        other => other.to_string(),
    }
}

/// Trim an RFC3339 timestamp to `YYYY-MM-DD HH:MM`.
fn short_time(value: &str) -> String {
    if value.len() >= 16 {
        value[..16].replace('T', " ")
    } else {
        empty_dash(value)
    }
}

fn print_table(headers: [&str; 4], rows: &[[String; 4]]) {
    let mut widths = headers.map(|header| header.len());
    for row in rows {
        for (index, cell) in row.iter().enumerate() {
            widths[index] = widths[index].max(cell.chars().count());
        }
    }
    let render = |cells: &[String; 4]| {
        cells
            .iter()
            .enumerate()
            .map(|(index, cell)| format!("{cell:<width$}", width = widths[index]))
            .collect::<Vec<_>>()
            .join("   ")
    };
    let header_row: [String; 4] = headers.map(|header| header.to_string());
    println!("{}", render(&header_row));
    println!("{}", "─".repeat(widths.iter().sum::<usize>() + 9));
    for row in rows {
        println!("{}", render(row));
    }
}
