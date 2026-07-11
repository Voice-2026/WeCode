use super::*;
use crate::app::ui_helpers::wecode_tooltip_container;

impl WeCodeApp {
    /// Overlay descriptor for the terminal when the selected project's remote
    /// host link is not usable: `(icon, tint, message)`. `None` for a local
    /// project or a healthy connected link.
    fn selected_project_terminal_link_overlay(&self) -> Option<(HeroIconName, u32, String)> {
        let host = self
            .state
            .selected_project
            .as_ref()
            .and_then(|project| project.host_device_id.as_deref())?;
        match self.remote_link_states.get(host).copied() {
            Some(wecode_runtime::remote::ControllerLinkState::Disconnected) => Some((
                HeroIconName::LinkSlash,
                theme::RED,
                "远程主机已离线 · 正在自动重连…".to_string(),
            )),
            Some(wecode_runtime::remote::ControllerLinkState::Connecting) => Some((
                HeroIconName::Link,
                theme::ORANGE,
                "正在连接远程主机…".to_string(),
            )),
            _ => None,
        }
    }

    pub(in crate::app) fn terminal_workspace_body(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .flex_1()
            .flex_basis(px(0.0))
            .min_w_0()
            .min_h_0()
            .w_full()
            .h_full()
            .child(
                div()
                    .flex_1()
                    .flex_basis(px(0.0))
                    .min_w_0()
                    .min_h_0()
                    .w_full()
                    .child(self.terminal_main_split_area(cx)),
            )
    }

    fn terminal_main_split_area(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .flex_1()
            .size_full()
            .min_w_0()
            .min_h_0()
            .child(self.terminal_panes(cx))
    }

    pub(in crate::app) fn terminal_panes(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let Some(active) = self.main_terminal() else {
            return div().flex_1().size_full();
        };
        let pane_count = active.panes.len();
        let link_overlay = self.selected_project_terminal_link_overlay();
        div()
            .relative()
            .flex()
            .flex_1()
            .min_w_0()
            .overflow_hidden()
            .when_some(link_overlay, |this, overlay| {
                this.child(terminal_link_overlay(overlay))
            })
            .children(active.panes.iter().enumerate().map(|(index, slot)| {
                let close_id = SharedString::from(format!("terminal-pane-close-{index}"));
                let float_id = SharedString::from(format!("terminal-pane-float-{index}"));
                let add_id = SharedString::from(format!("terminal-pane-add-{index}"));
                div()
                    .relative()
                    .group("terminal-pane")
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_w_0()
                    .overflow_hidden()
                    .border_l_1()
                    .border_color(color(if index == 0 {
                        theme::BG_TERMINAL
                    } else {
                        theme::BORDER_SOFT
                    }))
                    .child(
                        div().flex_1().min_w_0().child(match &slot.pane {
                            Some(pane) => gpui::AnyView::from(pane.view.clone())
                                .cached(gpui::StyleRefinement::default().flex().size_full())
                                .into_any_element(),
                            None => div()
                                .id(SharedString::from(format!("terminal-pane-mount-{index}")))
                                .size_full()
                                .flex()
                                .items_center()
                                .justify_center()
                                .cursor_pointer()
                                .bg(theme::terminal_fill(color(theme::BG_TERMINAL)))
                                .text_color(color(theme::TEXT_DIM))
                                .on_click(cx.listener(move |app, _event, window, cx| {
                                    app.select_terminal_pane(index, window, cx);
                                }))
                                .child("Click to open terminal")
                                .into_any_element(),
                        }),
                    )
                    .child(
                        div()
                            .absolute()
                            .top_2()
                            .right_2()
                            .flex()
                            .items_center()
                            .gap_1()
                            .child(terminal_pane_agent_button(
                                SharedString::from(format!("terminal-pane-agent-{index}")),
                                has_project_context(&self.state),
                                cx,
                            ))
                            .child(terminal_pane_control_button(
                                float_id,
                                HeroIconName::ArrowTopRightOnSquare,
                                "浮窗",
                                true,
                                cx,
                                move |app, _event, window, cx| {
                                    app.float_terminal_pane(index, window, cx)
                                },
                            ))
                            .child(terminal_pane_control_button(
                                add_id,
                                HeroIconName::Plus,
                                "新建分屏",
                                true,
                                cx,
                                |app, _event, window, cx| app.split_terminal(window, cx),
                            ))
                            .child(terminal_pane_control_button(
                                close_id,
                                HeroIconName::XMark,
                                "关闭分屏",
                                pane_count > 1,
                                cx,
                                move |app, _event, window, cx| {
                                    app.close_terminal_pane(index, window, cx)
                                },
                            )),
                    )
                    .into_any_element()
            }))
    }
}

fn has_project_context(state: &RuntimeState) -> bool {
    state.selected_project.is_some()
}

/// A centered banner over the terminal area when the remote host link is down
/// or reconnecting, so a frozen remote shell reads as "offline, recovering"
/// instead of an unexplained blank pane.
fn terminal_link_overlay(overlay: (HeroIconName, u32, String)) -> impl IntoElement {
    let (icon, tint, message) = overlay;
    div()
        .absolute()
        .top_0()
        .left_0()
        .right_0()
        .flex()
        .items_center()
        .justify_center()
        .py_2()
        .gap_2()
        .bg(color(theme::BG_HEADER))
        .border_b_1()
        .border_color(color(theme::BORDER_SOFT))
        .child(Icon::new(icon).size_4().text_color(color(tint)))
        .child(
            div()
                .text_size(rems(0.8125))
                .text_color(color(theme::TEXT))
                .child(message),
        )
}

fn terminal_pane_agent_button(
    id: SharedString,
    enabled: bool,
    cx: &mut Context<WeCodeApp>,
) -> AnyElement {
    let app_entity = cx.entity();
    let gateway_status = GatewayService::global_status();
    let gateway_ready = gateway_status.addr.is_some() && gateway_status.error.is_none();
    let gateway_hint = if let Some(error) = gateway_status.error {
        format!("Gateway failed: {error}")
    } else if gateway_status.enabled {
        "Gateway starting".to_string()
    } else {
        "Gateway disabled".to_string()
    };
    let text_color = if enabled {
        cx.theme().secondary_foreground
    } else {
        color(theme::TEXT_DIM)
    };

    Button::new(id)
        .compact()
        .ghost()
        .h(px(28.0))
        .w(px(30.0))
        .disabled(!enabled)
        .tooltip("切换当前终端的 AI 工具")
        .text_color(text_color)
        .child(
            Icon::new(HeroIconName::Sparkles)
                .size_3p5()
                .text_color(text_color),
        )
        .dropdown_menu(move |menu, _window, _cx| {
            let mut menu = menu
                .item(quick_agent_item(
                    app_entity.clone(),
                    "Claude Code",
                    HeroIconName::CommandLine,
                    "claude",
                    !enabled,
                ))
                .item(quick_agent_item(
                    app_entity.clone(),
                    "Codex",
                    HeroIconName::CommandLine,
                    "codex",
                    !enabled,
                ))
                .item(quick_agent_item(
                    app_entity.clone(),
                    "Kiro",
                    HeroIconName::Sparkles,
                    "kiro",
                    !enabled,
                ))
                .separator();
            if gateway_ready {
                menu = menu
                    .item(quick_agent_item(
                        app_entity.clone(),
                        "Kiro Gateway · Claude · Opus 4.8",
                        HeroIconName::ServerStack,
                        "kiro-gateway-claude",
                        !enabled,
                    ))
                    .separator()
                    .item(quick_agent_item(
                        app_entity.clone(),
                        "Gateway · Haiku 4.5",
                        HeroIconName::ServerStack,
                        "kiro-gateway-claude-haiku-4-5",
                        !enabled,
                    ))
                    .item(quick_agent_item(
                        app_entity.clone(),
                        "Gateway · Sonnet 4.6",
                        HeroIconName::ServerStack,
                        "kiro-gateway-claude-sonnet-4-6",
                        !enabled,
                    ))
                    .item(quick_agent_item(
                        app_entity.clone(),
                        "Gateway · Opus 4.6",
                        HeroIconName::ServerStack,
                        "kiro-gateway-claude-opus-4-6",
                        !enabled,
                    ))
                    .item(quick_agent_item(
                        app_entity.clone(),
                        "Gateway · Opus 4.7",
                        HeroIconName::ServerStack,
                        "kiro-gateway-claude-opus-4-7",
                        !enabled,
                    ))
                    .item(quick_agent_item(
                        app_entity.clone(),
                        "Gateway · Opus 4.8",
                        HeroIconName::ServerStack,
                        "kiro-gateway-claude-opus-4-8",
                        !enabled,
                    ))
                    .item(quick_agent_item(
                        app_entity.clone(),
                        "Gateway · DeepSeek 3.2",
                        HeroIconName::ServerStack,
                        "kiro-gateway-claude-deepseek-3-2",
                        !enabled,
                    ))
                    .item(quick_agent_item(
                        app_entity.clone(),
                        "Gateway · GLM 5",
                        HeroIconName::ServerStack,
                        "kiro-gateway-claude-glm-5",
                        !enabled,
                    ))
                    .item(quick_agent_item(
                        app_entity.clone(),
                        "Gateway · MiniMax M2.5",
                        HeroIconName::ServerStack,
                        "kiro-gateway-claude-minimax-m2-5",
                        !enabled,
                    ))
                    .item(quick_agent_item(
                        app_entity.clone(),
                        "Gateway · Qwen3 Coder",
                        HeroIconName::ServerStack,
                        "kiro-gateway-claude-qwen3-coder-next",
                        !enabled,
                    ));
            } else {
                menu = menu.item(
                    PopupMenuItem::new(gateway_hint.clone())
                        .icon(HeroIconName::ServerStack)
                        .disabled(true),
                );
            }
            menu
        })
        .into_any_element()
}

fn quick_agent_item(
    app_entity: gpui::Entity<WeCodeApp>,
    label: &'static str,
    icon: HeroIconName,
    target: &'static str,
    disabled: bool,
) -> PopupMenuItem {
    PopupMenuItem::new(label)
        .icon(icon)
        .disabled(disabled)
        .on_click(move |_, window, cx| {
            cx.update_entity(&app_entity, |app, cx| {
                app.launch_quick_agent(target, window, cx);
            });
        })
}

fn terminal_pane_control_button(
    id: SharedString,
    icon: HeroIconName,
    tooltip: &'static str,
    enabled: bool,
    cx: &mut Context<WeCodeApp>,
    on_click: impl Fn(&mut WeCodeApp, &gpui::ClickEvent, &mut Window, &mut Context<WeCodeApp>) + 'static,
) -> AnyElement {
    let text_color = if enabled {
        cx.theme().secondary_foreground
    } else {
        color(theme::TEXT_DIM)
    };
    let button = wecode_tooltip_container(cx.entity(), id, tooltip)
        .size(px(28.0))
        .flex()
        .flex_none()
        .items_center()
        .justify_center()
        .rounded_sm()
        .text_color(text_color)
        .child(Icon::new(icon).size_3p5().text_color(text_color));

    if enabled {
        button
            .cursor_pointer()
            .hover(|style| style.bg(cx.theme().secondary_hover))
            .on_click(cx.listener(move |app, event, window, cx| {
                cx.stop_propagation();
                window.prevent_default();
                on_click(app, event, window, cx);
            }))
            .into_any_element()
    } else {
        button.opacity(0.45).into_any_element()
    }
}
