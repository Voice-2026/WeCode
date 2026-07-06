use super::options::*;
use super::widgets::*;
use super::*;

pub(super) fn settings_notifications_pane(
    notifications: &NotificationSummary,
    _selected_channel_id: Option<&str>,
    testing_channel_id: Option<&str>,
    language: &str,
    window: &mut Window,
    cx: &mut Context<CoduxApp>,
) -> AnyElement {
    settings_form(
        notifications
            .channels
            .iter()
            .cloned()
            .map(|channel| {
                settings_notification_card(channel, testing_channel_id, language, window, cx)
                    .into_any_element()
            })
            .collect::<Vec<_>>(),
    )
    .into_any_element()
}
pub(super) fn settings_notification_card(
    channel: codux_runtime::notification::NotificationChannelSummary,
    testing_channel_id: Option<&str>,
    language: &str,
    window: &mut Window,
    cx: &mut Context<CoduxApp>,
) -> AnyElement {
    let enabled_id = channel.id.clone();
    let endpoint_id = channel.id.clone();
    let token_id = channel.id.clone();
    let testing = testing_channel_id
        .map(|id| id == channel.id)
        .unwrap_or(false);
    let test_disabled = testing_channel_id.is_some() || channel.endpoint.trim().is_empty();
    settings_card(
        None,
        None,
        {
            let mut rows = vec![
                div()
                    .flex()
                    .items_start()
                    .justify_between()
                    .gap(px(16.0))
                    .child(
                        div()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .text_size(rems(0.875))
                                    .line_height(rems(1.125))
                                    .text_color(color(theme::TEXT))
                                    .child(channel.label.clone()),
                            )
                            .child(
                                div()
                                    .mt(px(4.0))
                                    .text_size(rems(0.75))
                                    .line_height(rems(1.0))
                                    .text_color(color(theme::TEXT_DIM))
                                    .child(notification_channel_description(&channel.id, language)),
                            ),
                    )
                    .child(settings_toggle(
                        format!("settings-notification-enabled-{}", channel.id),
                        channel.enabled,
                        cx,
                        move |app, window, cx| {
                            let next = !app
                                .state
                                .notifications
                                .channels
                                .iter()
                                .find(|item| item.id == enabled_id)
                                .map(|item| item.enabled)
                                .unwrap_or(false);
                            app.set_notification_channel_enabled(
                                enabled_id.clone(),
                                next,
                                window,
                                cx,
                            )
                        },
                    ))
                    .into_any_element(),
            ];
            if channel.enabled {
                rows.extend([
                    settings_row(
                        notification_endpoint_label(&channel.id, language),
                        None,
                        settings_text_input(
                            SharedString::from(format!(
                                "settings-notification-endpoint-{}",
                                channel.id
                            )),
                            channel.endpoint.clone(),
                            notification_endpoint_label(&channel.id, language),
                            false,
                            window,
                            cx,
                            move |app, value, window, cx| {
                                app.update_notification_channel_string(
                                    endpoint_id.clone(),
                                    "endpoint",
                                    value,
                                    window,
                                    cx,
                                )
                            },
                        ),
                    )
                    .into_any_element(),
                    settings_row(
                        notification_token_label(&channel.id, language),
                        None,
                        settings_text_input(
                            SharedString::from(format!(
                                "settings-notification-token-{}",
                                channel.id
                            )),
                            channel.token.clone(),
                            notification_token_label(&channel.id, language),
                            true,
                            window,
                            cx,
                            move |app, value, window, cx| {
                                app.update_notification_channel_string(
                                    token_id.clone(),
                                    "token",
                                    value,
                                    window,
                                    cx,
                                )
                            },
                        ),
                    )
                    .into_any_element(),
                    div()
                        .flex()
                        .justify_end()
                        .child(settings_small_button_state(
                            format!("settings-notification-test-{}", channel.id),
                            if testing {
                                settings_text(
                                    language,
                                    "settings.ai.provider.test.running",
                                    "Testing...",
                                )
                            } else {
                                settings_text(language, "common.test", "Test")
                            },
                            testing,
                            test_disabled,
                            cx,
                            move |app, _event, window, cx| {
                                app.test_notification_channel(channel.id.clone(), window, cx)
                            },
                        ))
                        .into_any_element(),
                ]);
            }
            rows
        },
        cx,
    )
    .into_any_element()
}
