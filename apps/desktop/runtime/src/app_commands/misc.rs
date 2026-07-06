use super::*;

pub fn power_set_sleep_prevention(manager: &PowerManager, mode: String) -> Result<bool, String> {
    manager.set_sleep_prevention(mode)
}
pub fn notification_dispatch_channels(
    request: NotificationDispatchRequest,
) -> NotificationDispatchResult {
    crate::notification::dispatch_notification_channels_blocking(request)
}
