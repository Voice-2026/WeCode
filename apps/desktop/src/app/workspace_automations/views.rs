use super::*;

pub(super) fn automation_list_row(
    definition: wecode_runtime::automation::AutomationDefinition,
    latest_run: Option<AutomationRun>,
    active: bool,
    app_entity: gpui::Entity<WeCodeApp>,
    cx: &mut Context<WeCodeApp>,
) -> AnyElement {
    let id = definition.id.clone();
    let state = latest_run.as_ref().map(|run| run.state);
    let enabled = definition.enabled;
    div()
        .id(format!("automation-row-{id}"))
        .w_full()
        .h(px(AUTOMATION_LIST_ROW_HEIGHT))
        .flex_none()
        .overflow_hidden()
        .mb(px(AUTOMATION_LIST_ROW_GAP))
        .px(px(11.0))
        .py(px(9.0))
        .flex()
        .flex_col()
        .gap(px(4.0))
        .rounded(px(8.0))
        .border_1()
        .border_color(if active {
            cx.theme().primary.opacity(0.65)
        } else {
            cx.theme().border
        })
        .bg(if active {
            cx.theme().secondary
        } else {
            cx.theme().background.opacity(0.5)
        })
        .cursor_pointer()
        .hover(|this| this.bg(cx.theme().secondary.opacity(0.78)))
        .on_click(move |_, _window, cx| {
            cx.update_entity(&app_entity, |app, cx| {
                app.automation_selected_id = Some(id.clone());
                app.automation_editor_open = false;
                app.automation_detail_tab = AutomationDetailTab::Overview;
                app.invalidate_ui(cx, [UiRegion::WorkspaceBody]);
            });
        })
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(7.0))
                .child(div().size(px(7.0)).rounded(px(999.0)).bg(if enabled {
                    cx.theme().success
                } else {
                    cx.theme().muted_foreground
                }))
                .child(
                    div()
                        .min_w_0()
                        .flex_1()
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(cx.theme().foreground)
                        .child(definition.name),
                )
                .child(automation_enabled_badge(enabled, cx)),
        )
        .child(
            div()
                .w_full()
                .min_w_0()
                .text_size(rems(0.68))
                .text_color(cx.theme().muted_foreground)
                .child(format!(
                    "{} · {}",
                    automation_list_schedule_label(&definition.schedule),
                    automation_list_agent_label(definition.agent)
                )),
        )
        .child(
            div()
                .w_full()
                .min_w_0()
                .text_size(rems(0.68))
                .font_weight(if enabled {
                    FontWeight::NORMAL
                } else {
                    FontWeight::MEDIUM
                })
                .text_color(if enabled {
                    automation_state_color(state, cx)
                } else {
                    cx.theme().muted_foreground
                })
                .child(if enabled {
                    automation_state_label(state, definition.next_run_at)
                } else {
                    "已暂停 · 不会自动运行".to_string()
                }),
        )
        .into_any_element()
}

pub(super) fn automation_enabled_badge(enabled: bool, cx: &mut Context<WeCodeApp>) -> AnyElement {
    div()
        .flex_none()
        .px(px(7.0))
        .py(px(2.0))
        .rounded(px(999.0))
        .bg(if enabled {
            cx.theme().success.opacity(0.12)
        } else {
            cx.theme().secondary
        })
        .text_size(rems(0.62))
        .font_weight(FontWeight::MEDIUM)
        .text_color(if enabled {
            cx.theme().success
        } else {
            cx.theme().muted_foreground
        })
        .child(if enabled { "已启用" } else { "已暂停" })
        .into_any_element()
}

pub(super) fn automation_list_schedule_label(schedule: &AutomationSchedule) -> String {
    match schedule {
        AutomationSchedule::Weekly {
            weekdays,
            hour,
            minute,
        } if weekdays == &[1, 2, 3, 4, 5] => format!("工作日 {hour:02}:{minute:02}"),
        AutomationSchedule::Weekly {
            weekdays,
            hour,
            minute,
        } => {
            let days = weekdays
                .iter()
                .filter_map(|day| {
                    day.checked_sub(1).and_then(|index| {
                        ["一", "二", "三", "四", "五", "六", "日"].get(index as usize)
                    })
                })
                .copied()
                .collect::<Vec<_>>()
                .join("/");
            format!("周{days} {hour:02}:{minute:02}")
        }
        _ => schedule.display(),
    }
}

fn automation_list_agent_label(agent: AutomationAgent) -> &'static str {
    match agent {
        AutomationAgent::Claude => "Claude",
        AutomationAgent::KiroGatewayClaude => "Claude + Kiro",
        AutomationAgent::KiroCodex => "Codex + Kiro",
        AutomationAgent::Codex => "Codex",
        AutomationAgent::Kiro => "Kiro",
    }
}

pub(super) fn automation_template_card(
    template: AutomationTemplate,
    app_entity: gpui::Entity<WeCodeApp>,
    cx: &mut Context<WeCodeApp>,
) -> AnyElement {
    div()
        .id(format!("automation-template-{}", template.id))
        .min_h(px(92.0))
        .p(px(12.0))
        .flex()
        .flex_col()
        .gap(px(6.0))
        .rounded(px(9.0))
        .border_1()
        .border_color(cx.theme().border)
        .bg(cx.theme().background)
        .cursor_pointer()
        .hover(|this| this.bg(cx.theme().secondary))
        .on_click(move |_, window, cx| {
            cx.update_entity(&app_entity, |app, cx| {
                app.apply_automation_template(template, window, cx);
            });
        })
        .child(
            div()
                .font_weight(FontWeight::MEDIUM)
                .text_color(cx.theme().foreground)
                .child(template.title),
        )
        .child(
            div()
                .text_size(rems(0.7))
                .text_color(cx.theme().muted_foreground)
                .whitespace_normal()
                .child(template.description),
        )
        .into_any_element()
}

pub(super) fn automation_template_button(
    template: AutomationTemplate,
    app_entity: gpui::Entity<WeCodeApp>,
) -> AnyElement {
    Button::new(format!("automation-template-compact-{}", template.id))
        .secondary()
        .compact()
        .with_size(Size::Small)
        .label(template.title)
        .on_click(move |_, window, cx| {
            cx.update_entity(&app_entity, |app, cx| {
                app.apply_automation_template(template, window, cx);
            });
        })
        .into_any_element()
}

pub(super) fn automation_agent_value(agent: AutomationAgent) -> &'static str {
    match agent {
        AutomationAgent::Claude => "claude",
        AutomationAgent::KiroGatewayClaude => "kiro_gateway_claude",
        AutomationAgent::KiroCodex => "kiro_gateway_codex",
        AutomationAgent::Codex => "codex",
        AutomationAgent::Kiro => "kiro",
    }
}

pub(super) fn automation_workspace_mode_value(mode: AutomationWorkspaceMode) -> &'static str {
    match mode {
        AutomationWorkspaceMode::Existing => "existing",
        AutomationWorkspaceMode::NewPerRun => "new",
    }
}

pub(super) fn automation_workspace_mode_from_value(value: &str) -> Option<AutomationWorkspaceMode> {
    match value.trim() {
        "existing" => Some(AutomationWorkspaceMode::Existing),
        "new" => Some(AutomationWorkspaceMode::NewPerRun),
        _ => None,
    }
}

pub(super) fn automation_agent_from_value(value: &str) -> Option<AutomationAgent> {
    match value.trim() {
        "claude" => Some(AutomationAgent::Claude),
        "kiro_gateway_claude" => Some(AutomationAgent::KiroGatewayClaude),
        "kiro_gateway_codex" => Some(AutomationAgent::KiroCodex),
        "codex" => Some(AutomationAgent::Codex),
        "kiro" => Some(AutomationAgent::Kiro),
        _ => None,
    }
}

pub(super) fn automation_gateway_model_options(
    catalog: &wecode_runtime::gateway_service::GatewayModelCatalog,
    agent: AutomationAgent,
    selected: &str,
) -> Vec<SelectOption> {
    let models: Box<dyn Iterator<Item = _>> = match agent {
        AutomationAgent::KiroCodex => Box::new(catalog.codex_cli_models()),
        _ => Box::new(catalog.claude_code_models()),
    };
    let mut options = models
        .map(|model| SelectOption::new(model.id.clone(), SharedString::from(model.name.clone())))
        .collect::<Vec<_>>();
    let selected = selected.trim();
    if !selected.is_empty()
        && !options
            .iter()
            .any(|option| option.value.as_str() == selected)
    {
        options.push(SelectOption::new(
            selected.to_string(),
            SharedString::from(selected.to_string()),
        ));
    }
    options
}

pub(super) fn automation_schedule_preset_value(preset: AutomationSchedulePreset) -> &'static str {
    match preset {
        AutomationSchedulePreset::Hourly => "hourly",
        AutomationSchedulePreset::Daily => "daily",
        AutomationSchedulePreset::Weekdays => "weekdays",
        AutomationSchedulePreset::Weekly => "weekly",
        AutomationSchedulePreset::Custom => "custom",
    }
}

pub(super) fn automation_schedule_preset_from_value(
    value: &str,
) -> Option<AutomationSchedulePreset> {
    match value.trim() {
        "hourly" => Some(AutomationSchedulePreset::Hourly),
        "daily" => Some(AutomationSchedulePreset::Daily),
        "weekdays" => Some(AutomationSchedulePreset::Weekdays),
        "weekly" => Some(AutomationSchedulePreset::Weekly),
        "custom" => Some(AutomationSchedulePreset::Custom),
        _ => None,
    }
}

pub(super) fn automation_catch_up_grace_options() -> Vec<SelectOption> {
    [
        ("0", "不补跑"),
        ("3600", "1 小时内"),
        ("21600", "6 小时内"),
        ("43200", "12 小时内"),
        ("86400", "24 小时内"),
    ]
    .into_iter()
    .map(|(value, label)| SelectOption::new(value, label))
    .collect()
}

pub(super) fn automation_catch_up_grace_label(seconds: i64) -> String {
    match seconds {
        0 => "不补跑".to_string(),
        3600 => "1 小时内".to_string(),
        21600 => "6 小时内".to_string(),
        43200 => "12 小时内".to_string(),
        86400 => "24 小时内".to_string(),
        seconds if seconds > 0 && seconds % 3600 == 0 => {
            format!("{} 小时内", seconds / 3600)
        }
        seconds => format!("{} 秒内", seconds.max(0)),
    }
}

pub(super) fn automation_default_branch_for_path(path: &str) -> String {
    automation_branch_options_for_path(path, "")
        .into_iter()
        .next()
        .unwrap_or_else(|| "main".to_string())
}

pub(super) fn automation_branch_options_for_path(path: &str, selected: &str) -> Vec<String> {
    let mut branches = Vec::new();
    push_unique_automation_branch(&mut branches, selected);
    let current = std::process::Command::new("git")
        .args(["-C", path, "branch", "--show-current"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .unwrap_or_default();
    push_unique_automation_branch(&mut branches, &current);
    if let Ok(output) = std::process::Command::new("git")
        .args([
            "-C",
            path,
            "for-each-ref",
            "--format=%(refname:short)",
            "refs/heads",
        ])
        .output()
        && output.status.success()
    {
        if let Ok(stdout) = String::from_utf8(output.stdout) {
            for branch in stdout.lines() {
                push_unique_automation_branch(&mut branches, branch);
            }
        }
    }
    branches
}

pub(super) fn automation_weekday_button(
    day: u32,
    active: u32,
    app_entity: gpui::Entity<WeCodeApp>,
) -> AnyElement {
    let label = ["周一", "周二", "周三", "周四", "周五", "周六", "周日"]
        [(day.saturating_sub(1) as usize).min(6)];
    Button::new(format!("automation-weekday-{day}"))
        .compact()
        .with_size(Size::Small)
        .when(day == active, |button| button.primary())
        .when(day != active, |button| button.secondary())
        .label(label)
        .on_click(move |_, _window, cx| {
            cx.update_entity(&app_entity, |app, cx| {
                app.automation_weekday = day;
                app.invalidate_ui(cx, [UiRegion::WorkspaceBody]);
            });
        })
        .into_any_element()
}

pub(super) fn automation_tab_button(
    tab: AutomationDetailTab,
    label: &str,
    active: AutomationDetailTab,
    app_entity: gpui::Entity<WeCodeApp>,
    cx: &mut Context<WeCodeApp>,
) -> AnyElement {
    let label = label.to_string();
    div()
        .id(format!("automation-tab-{tab:?}"))
        .px(px(12.0))
        .py(px(8.0))
        .border_b_2()
        .border_color(if tab == active {
            cx.theme().primary
        } else {
            gpui::transparent_black()
        })
        .text_size(rems(0.75))
        .font_weight(if tab == active {
            FontWeight::SEMIBOLD
        } else {
            FontWeight::NORMAL
        })
        .cursor_pointer()
        .child(label)
        .on_click(move |_, _window, cx| {
            cx.update_entity(&app_entity, |app, cx| {
                app.automation_detail_tab = tab;
                app.invalidate_ui(cx, [UiRegion::WorkspaceBody]);
            });
        })
        .into_any_element()
}

pub(super) fn automation_overview(
    definition: wecode_runtime::automation::AutomationDefinition,
    latest: Option<AutomationRun>,
    cx: &mut Context<WeCodeApp>,
) -> impl IntoElement {
    div()
        .grid()
        .grid_cols(2)
        .gap(px(12.0))
        .child(automation_status_card(definition.enabled, cx))
        .child(automation_info_card(
            "下次运行",
            format_automation_time(definition.next_run_at),
            cx,
        ))
        .child(automation_info_card(
            "执行计划",
            format!(
                "{} · {}",
                definition.schedule.display(),
                definition.timezone
            ),
            cx,
        ))
        .child(automation_info_card(
            "项目",
            definition.project_name.clone(),
            cx,
        ))
        .child(automation_info_card(
            "运行方式",
            if definition.workspace_mode == AutomationWorkspaceMode::NewPerRun {
                format!(
                    "每次新建 Worktree · 来源 {}",
                    definition.base_branch.as_deref().unwrap_or("—")
                )
            } else {
                format!("固定工作区 · {}", definition.workspace_name)
            },
            cx,
        ))
        .child(automation_info_card(
            "智能体",
            definition.model.as_ref().map_or_else(
                || definition.agent.label().to_string(),
                |model| format!("{} · {model}", definition.agent.label()),
            ),
            cx,
        ))
        .child(automation_info_card(
            "会话",
            if definition.reuse_session {
                "重复利用上次会话".to_string()
            } else {
                "每次使用新会话".to_string()
            },
            cx,
        ))
        .child(automation_info_card(
            "补跑时限",
            automation_catch_up_grace_label(definition.catch_up_grace_seconds),
            cx,
        ))
        .child(automation_info_card(
            "执行前检查",
            definition
                .precheck_command
                .as_deref()
                .map(|command| format!("{}（{} 秒）", command, definition.precheck_timeout_seconds))
                .unwrap_or_else(|| "未设置".to_string()),
            cx,
        ))
        .child(automation_info_card(
            "最近运行",
            latest
                .as_ref()
                .map(|run| automation_state_label(Some(run.state), definition.next_run_at))
                .unwrap_or_else(|| "尚未运行".to_string()),
            cx,
        ))
        .child(
            div()
                .col_span(2)
                .p(px(14.0))
                .rounded(px(9.0))
                .border_1()
                .border_color(cx.theme().border)
                .bg(cx.theme().background)
                .flex()
                .flex_col()
                .gap(px(8.0))
                .child(form_field_label("任务提示词", cx))
                .child(
                    div()
                        .text_size(rems(0.78))
                        .text_color(cx.theme().foreground)
                        .whitespace_normal()
                        .child(definition.prompt),
                ),
        )
}

pub(super) fn automation_runs(
    runs: Vec<AutomationRun>,
    app_entity: gpui::Entity<WeCodeApp>,
    cx: &mut Context<WeCodeApp>,
) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .gap(px(10.0))
        .when(runs.is_empty(), |this| {
            this.child(
                div()
                    .py(px(34.0))
                    .text_size(rems(0.78))
                    .text_color(cx.theme().muted_foreground)
                    .child("还没有运行记录"),
            )
        })
        .children(
            runs.into_iter()
                .map(|run| automation_run_card(run, app_entity.clone(), cx)),
        )
        .into_any_element()
}

pub(super) fn automation_run_card(
    run: AutomationRun,
    app_entity: gpui::Entity<WeCodeApp>,
    cx: &mut Context<WeCodeApp>,
) -> AnyElement {
    let run_id = run.id.clone();
    let has_terminal = run.terminal_id.is_some();
    let open_action = if run.state.is_terminal() && run.ai_session_id.is_some() {
        let session_run_id = run_id.clone();
        let session_app_entity = app_entity.clone();
        Some(
            Button::new(format!("automation-open-session-{session_run_id}"))
                .secondary()
                .compact()
                .with_size(Size::Small)
                .label("打开会话")
                .on_click(move |_, window, cx| {
                    cx.update_entity(&session_app_entity, |app, cx| {
                        app.open_automation_session(&session_run_id, window, cx);
                    });
                })
                .into_any_element(),
        )
    } else if has_terminal && !run.state.is_terminal() {
        let terminal_run_id = run_id.clone();
        let terminal_app_entity = app_entity.clone();
        Some(
            Button::new(format!("automation-open-run-{terminal_run_id}"))
                .secondary()
                .compact()
                .with_size(Size::Small)
                .label("打开终端")
                .on_click(move |_, window, cx| {
                    cx.update_entity(&terminal_app_entity, |app, cx| {
                        app.open_automation_terminal(&terminal_run_id, window, cx);
                    });
                })
                .into_any_element(),
        )
    } else {
        None
    };
    let output = run
        .output_snapshot
        .as_ref()
        .map(|snapshot| snapshot.content.clone());
    let precheck = run.precheck_result.as_ref().map(|result| {
        if result.passed() {
            format!("执行前检查通过 · {} ms", result.duration_ms)
        } else {
            result.failure_message()
        }
    });
    div()
        .w_full()
        .p(px(13.0))
        .rounded(px(9.0))
        .border_1()
        .border_color(cx.theme().border)
        .bg(cx.theme().background)
        .flex()
        .flex_col()
        .gap(px(8.0))
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(8.0))
                        .child(
                            div()
                                .size(px(7.0))
                                .rounded(px(999.0))
                                .bg(automation_state_color(Some(run.state), cx)),
                        )
                        .child(
                            div()
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(cx.theme().foreground)
                                .child(format!("运行 #{}", run.run_number.max(1))),
                        )
                        .child(
                            div()
                                .text_size(rems(0.7))
                                .text_color(cx.theme().muted_foreground)
                                .child(format_automation_time(Some(run.scheduled_for))),
                        ),
                )
                .when_some(open_action, |this, action| this.child(action)),
        )
        .child(
            div()
                .text_size(rems(0.72))
                .text_color(automation_state_color(Some(run.state), cx))
                .child(automation_run_state_label(run.state)),
        )
        .when_some(run.state_reason, |this, reason| {
            this.child(
                div()
                    .text_size(rems(0.72))
                    .text_color(cx.theme().danger)
                    .whitespace_normal()
                    .child(reason),
            )
        })
        .when_some(precheck, |this, precheck| {
            this.child(
                div()
                    .text_size(rems(0.68))
                    .text_color(cx.theme().muted_foreground)
                    .child(precheck),
            )
        })
        .when_some(output, |this, output| {
            this.child(
                div()
                    .w_full()
                    .h(px(150.0))
                    .overflow_y_scrollbar()
                    .p(px(10.0))
                    .rounded(px(7.0))
                    .bg(cx.theme().secondary.opacity(0.52))
                    .text_size(rems(0.7))
                    .text_color(cx.theme().foreground)
                    .whitespace_normal()
                    .child(output),
            )
        })
        .into_any_element()
}

pub(super) fn automation_info_card(
    label: &str,
    value: String,
    cx: &mut Context<WeCodeApp>,
) -> AnyElement {
    div()
        .p(px(13.0))
        .rounded(px(9.0))
        .border_1()
        .border_color(cx.theme().border)
        .bg(cx.theme().background)
        .flex()
        .flex_col()
        .gap(px(6.0))
        .child(
            div()
                .text_size(rems(0.68))
                .text_color(cx.theme().muted_foreground)
                .child(label.to_string()),
        )
        .child(
            div()
                .text_size(rems(0.78))
                .font_weight(FontWeight::MEDIUM)
                .text_color(cx.theme().foreground)
                .whitespace_normal()
                .child(value),
        )
        .into_any_element()
}

fn automation_status_card(enabled: bool, cx: &mut Context<WeCodeApp>) -> AnyElement {
    div()
        .p(px(13.0))
        .rounded(px(9.0))
        .border_1()
        .border_color(if enabled {
            cx.theme().success.opacity(0.34)
        } else {
            cx.theme().border
        })
        .bg(if enabled {
            cx.theme().success.opacity(0.08)
        } else {
            cx.theme().background
        })
        .flex()
        .flex_col()
        .gap(px(6.0))
        .child(
            div()
                .text_size(rems(0.68))
                .text_color(cx.theme().muted_foreground)
                .child("状态"),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(7.0))
                .child(div().size(px(7.0)).rounded(px(999.0)).bg(if enabled {
                    cx.theme().success
                } else {
                    cx.theme().muted_foreground
                }))
                .child(
                    div()
                        .text_size(rems(0.78))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(if enabled {
                            cx.theme().success
                        } else {
                            cx.theme().muted_foreground
                        })
                        .child(if enabled { "已启用" } else { "已暂停" }),
                ),
        )
        .into_any_element()
}

pub(super) fn automation_state_label(
    state: Option<AutomationRunState>,
    next_run_at: Option<i64>,
) -> String {
    match state {
        Some(AutomationRunState::Preparing) => "准备中".to_string(),
        Some(AutomationRunState::Running) => "运行中".to_string(),
        Some(AutomationRunState::WaitingInput) => "等待输入".to_string(),
        Some(AutomationRunState::Completed) => {
            format!("上次成功 · 下次 {}", format_automation_time(next_run_at))
        }
        Some(AutomationRunState::Failed) => {
            format!("上次失败 · 下次 {}", format_automation_time(next_run_at))
        }
        Some(AutomationRunState::Cancelled) => "已取消".to_string(),
        Some(AutomationRunState::SkippedOverlap) => "已跳过重叠执行".to_string(),
        Some(AutomationRunState::SkippedPrecheck) => "执行前检查未通过".to_string(),
        Some(AutomationRunState::Scheduled) | None => {
            format!("下次 {}", format_automation_time(next_run_at))
        }
    }
}

pub(super) fn automation_run_state_label(state: AutomationRunState) -> &'static str {
    match state {
        AutomationRunState::Scheduled => "等待调度",
        AutomationRunState::Preparing => "准备中",
        AutomationRunState::Running => "运行中",
        AutomationRunState::WaitingInput => "等待输入",
        AutomationRunState::Completed => "已完成",
        AutomationRunState::Failed => "执行失败",
        AutomationRunState::Cancelled => "已取消",
        AutomationRunState::SkippedOverlap => "已跳过：上一轮仍在运行",
        AutomationRunState::SkippedPrecheck => "已跳过：执行前检查未通过",
    }
}

pub(super) fn automation_state_color(
    state: Option<AutomationRunState>,
    cx: &mut Context<WeCodeApp>,
) -> gpui::Hsla {
    match state {
        Some(
            AutomationRunState::Failed
            | AutomationRunState::Cancelled
            | AutomationRunState::SkippedPrecheck,
        ) => cx.theme().danger,
        Some(AutomationRunState::Running | AutomationRunState::Preparing) => cx.theme().primary,
        Some(AutomationRunState::Completed) => cx.theme().success,
        _ => cx.theme().muted_foreground,
    }
}

pub(super) fn format_automation_time(timestamp: Option<i64>) -> String {
    timestamp
        .and_then(|timestamp| DateTime::from_timestamp(timestamp, 0))
        .map(|time| time.with_timezone(&Local).format("%m-%d %H:%M").to_string())
        .unwrap_or_else(|| "—".to_string())
}
