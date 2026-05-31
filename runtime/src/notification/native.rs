pub fn show_native_notification_blocking(
    title: &str,
    body: &str,
    group: &str,
) -> Result<(), String> {
    show_native_notification_impl(title, body, group)
}

#[cfg(target_os = "macos")]
fn show_native_notification_impl(title: &str, body: &str, group: &str) -> Result<(), String> {
    let _ = (title, body, group);
    Err("macOS native notifications are unavailable in the current dev launch mode".to_string())
}

#[cfg(not(target_os = "macos"))]
fn show_native_notification_impl(title: &str, body: &str, group: &str) -> Result<(), String> {
    let _ = (title, body, group);
    Ok(())
}
