pub fn show_native_notification_blocking(
    title: &str,
    body: &str,
    group: &str,
) -> Result<(), String> {
    show_native_notification_impl(title, body, group)
}

fn show_native_notification_impl(title: &str, body: &str, group: &str) -> Result<(), String> {
    let _ = group;
    let title = native_notification_text(title, "Codux");
    let body = native_notification_text(body, "");
    let mut notification = notify_rust::Notification::new();
    notification.summary(&title).body(&body);

    #[cfg(all(unix, not(target_os = "macos")))]
    notification.appname("Codux").icon("codux");

    #[cfg(target_os = "windows")]
    notification.app_id("com.duxweb.codux");

    notification
        .show()
        .map(|_| ())
        .map_err(|error| error.to_string())
}

fn native_notification_text(value: &str, fallback: &str) -> String {
    let value = value.trim();
    let text = if value.is_empty() { fallback } else { value };
    text.chars().take(512).collect()
}
