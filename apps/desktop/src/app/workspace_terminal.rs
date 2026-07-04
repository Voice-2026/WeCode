use super::*;
use crate::app::ui_helpers::codux_tooltip_container;
use super::agent_display::{humanize_tool_name, reduce_motion_enabled, shorten_model_name};
use super::ai_runtime_status::AgentLifecycleState;
use gpui::{Animation, AnimationExt as _, Transformation, ease_in_out, percentage};
use std::collections::HashMap;
use std::time::Duration;

impl CoduxApp {
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
            Some(codux_runtime::remote::ControllerLinkState::Disconnected) => Some((
                HeroIconName::LinkSlash,
                theme::RED,
                "远程主机已离线 · 正在自动重连…".to_string(),
            )),
            Some(codux_runtime::remote::ControllerLinkState::Connecting) => Some((
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
        let sessions_by_terminal: HashMap<
            &str,
            &codux_runtime::ai_runtime_state::AIRuntimeSessionSummary,
        > = self
            .state
            .ai_runtime_state
            .sessions
            .iter()
            .map(|session| (session.terminal_id.as_str(), session))
            .collect();
        let lifecycle_by_terminal: HashMap<&str, AgentLifecycleState> = self
            .pane_agent_lifecycle
            .iter()
            .map(|(terminal_id, lifecycle)| (terminal_id.as_str(), lifecycle.state))
            .collect();
        let collapsed_terminal_ids = self.pane_agent_chip_collapsed.clone();

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
                let terminal_id = Self::terminal_slot_terminal_id(active, index, slot);
                let session = terminal_id
                    .as_deref()
                    .and_then(|id| sessions_by_terminal.get(id).copied());
                let lifecycle_state = terminal_id
                    .as_deref()
                    .and_then(|id| lifecycle_by_terminal.get(id).copied())
                    .unwrap_or(AgentLifecycleState::Idle);
                let collapsed = terminal_id
                    .as_deref()
                    .is_some_and(|id| collapsed_terminal_ids.contains(id));
                let agent_chip = session.and_then(|session| {
                    let show = !collapsed || lifecycle_state == AgentLifecycleState::Waiting;
                    show.then(|| {
                        let terminal_id =
                            terminal_id.clone().expect("session binding requires terminal_id");
                        let collapse_terminal_id = terminal_id.clone();
                        terminal_pane_agent_chip_element(
                            &terminal_id,
                            lifecycle_state,
                            &session.tool,
                            session.model.as_deref(),
                        )
                        .cursor_pointer()
                        .on_click(cx.listener(move |app, _event, _window, cx| {
                            cx.stop_propagation();
                            let id = collapse_terminal_id.clone();
                            app.pane_agent_chip_collapsed.insert(id);
                            cx.notify();
                        }))
                        .into_any_element()
                    })
                });

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
                    .when_some(agent_chip, |pane, chip| pane.child(chip))
                    .child(
                        div()
                            .absolute()
                            .top_2()
                            .right_2()
                            .flex()
                            .items_center()
                            .gap_1()
                            .child(terminal_pane_control_button(
                                float_id,
                                HeroIconName::ArrowTopRightOnSquare,
                                "浮窗",
                                pane_count > 1,
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

#[derive(Clone, PartialEq, Eq)]
pub(in crate::app) struct AgentPaneChipSnapshot {
    pub state: AgentLifecycleState,
    pub tool: String,
    pub model: Option<String>,
    pub collapsed: bool,
}

pub(in crate::app) fn terminal_pane_agent_chip_element(
    terminal_id: &str,
    lifecycle_state: AgentLifecycleState,
    tool: &str,
    model: Option<&str>,
) -> gpui::Stateful<gpui::Div> {
    let chip_id = SharedString::from(format!("terminal-pane-agent-chip-{terminal_id}"));
    let label = agent_chip_label(tool, model);

    div()
        .id(chip_id)
        .absolute()
        .top_2()
        .left_2()
        .flex()
        .items_center()
        .gap_1()
        .px_2()
        .py_1()
        .rounded_md()
        .bg(theme::elevate(color(theme::BG_TERMINAL), 0.08).opacity(0.92))
        .text_size(rems(0.75))
        .text_color(color(theme::TEXT_MUTED))
        .child(agent_lifecycle_status_dot(lifecycle_state, terminal_id))
        .child(label)
}

fn agent_chip_label(tool: &str, model: Option<&str>) -> String {
    let name = humanize_tool_name(tool);
    match model.filter(|model| !model.trim().is_empty()) {
        Some(model) => format!("{} · {}", name, shorten_model_name(model)),
        None => name,
    }
}

fn agent_lifecycle_status_dot(
    lifecycle_state: AgentLifecycleState,
    terminal_id: &str,
) -> AnyElement {
    match lifecycle_state {
        AgentLifecycleState::Idle => div().into_any_element(),
        AgentLifecycleState::Working => {
            if reduce_motion_enabled() {
                return div()
                    .flex_none()
                    .size(px(6.0))
                    .rounded_full()
                    .bg(color(theme::ACCENT))
                    .into_any_element();
            }

            Icon::new(HeroIconName::ArrowPath)
                .size(px(8.0))
                .text_color(color(theme::ACCENT))
                .with_animation(
                    SharedString::from(format!("agent-chip-working-{terminal_id}")),
                    Animation::new(Duration::from_millis(900))
                        .repeat()
                        .with_easing(ease_in_out),
                    |icon, delta| icon.transform(Transformation::rotate(percentage(delta))),
                )
                .into_any_element()
        }
        AgentLifecycleState::Waiting => div()
            .flex_none()
            .size(px(6.0))
            .rounded_full()
            .bg(color(theme::ORANGE))
            .into_any_element(),
        AgentLifecycleState::Completed => Icon::new(HeroIconName::Check)
            .size(px(10.0))
            .text_color(color(theme::GREEN))
            .into_any_element(),
    }
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

fn terminal_pane_control_button(
    id: SharedString,
    icon: HeroIconName,
    tooltip: &'static str,
    enabled: bool,
    cx: &mut Context<CoduxApp>,
    on_click: impl Fn(&mut CoduxApp, &gpui::ClickEvent, &mut Window, &mut Context<CoduxApp>) + 'static,
) -> AnyElement {
    let text_color = if enabled {
        cx.theme().secondary_foreground
    } else {
        color(theme::TEXT_DIM)
    };
    let button = codux_tooltip_container(cx.entity(), id, tooltip)
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
