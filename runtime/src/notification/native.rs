pub fn show_native_notification_blocking(
    title: &str,
    body: &str,
    group: &str,
) -> Result<(), String> {
    show_native_notification_impl(title, body, group)
}

#[cfg(target_os = "macos")]
fn show_native_notification_impl(title: &str, body: &str, group: &str) -> Result<(), String> {
    let script = format!(
        "display notification {} with title {} subtitle {}",
        applescript_string(body),
        applescript_string(title),
        applescript_string(group),
    );
    let status = std::process::Command::new("osascript")
        .arg("-e")
        .arg(script)
        .status()
        .map_err(|error| error.to_string())?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("native notification failed: {status}"))
    }
}

#[cfg(not(target_os = "macos"))]
fn show_native_notification_impl(title: &str, body: &str, group: &str) -> Result<(), String> {
    let _ = (title, body, group);
    Ok(())
}

#[cfg(target_os = "macos")]
fn applescript_string(value: &str) -> String {
    let escaped = value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', " ");
    format!("\"{escaped}\"")
}
