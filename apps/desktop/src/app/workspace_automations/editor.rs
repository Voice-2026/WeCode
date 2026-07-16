use super::*;

impl WeCodeApp {
    pub(super) fn automation_editor_panel(
        &self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let Some(name_input) = self.automation_name_input.as_ref() else {
            return div().into_any_element();
        };
        let Some(schedule_input) = self.automation_schedule_input.as_ref() else {
            return div().into_any_element();
        };
        let Some(timezone_input) = self.automation_timezone_input.as_ref() else {
            return div().into_any_element();
        };
        let Some(prompt_input) = self.automation_prompt_input.as_ref() else {
            return div().into_any_element();
        };
        let Some(precheck_input) = self.automation_precheck_input.as_ref() else {
            return div().into_any_element();
        };
        let Some(precheck_timeout_input) = self.automation_precheck_timeout_input.as_ref() else {
            return div().into_any_element();
        };
        let Some(project_select) = self.automation_project_select.as_ref() else {
            return div().into_any_element();
        };
        let Some(workspace_select) = self.automation_workspace_select.as_ref() else {
            return div().into_any_element();
        };
        let Some(branch_select) = self.automation_branch_select.as_ref() else {
            return div().into_any_element();
        };
        let Some(agent_select) = self.automation_agent_select.as_ref() else {
            return div().into_any_element();
        };
        let Some(model_select) = self.automation_model_select.as_ref() else {
            return div().into_any_element();
        };
        let Some(schedule_select) = self.automation_schedule_select.as_ref() else {
            return div().into_any_element();
        };
        let Some(grace_select) = self.automation_grace_select.as_ref() else {
            return div().into_any_element();
        };
        let app_entity = cx.entity();
        let is_editing = self.automation_editing_id.is_some();
        let selected_project = self.automation_selected_project().cloned();
        let selected_project_is_local = selected_project
            .as_ref()
            .is_some_and(|project| project.host_device_id.is_none() && project.exists);
        let worktrees = selected_project
            .as_ref()
            .map(|project| self.automation_worktrees_for_project(project))
            .unwrap_or_default();
        let selected_worktree = worktrees
            .worktrees
            .iter()
            .find(|worktree| worktree.id == self.automation_workspace_id)
            .cloned();
        let can_save = selected_project_is_local
            && match self.automation_workspace_mode {
                AutomationWorkspaceMode::Existing => selected_worktree.is_some(),
                AutomationWorkspaceMode::NewPerRun => {
                    !self.automation_base_branch.trim().is_empty()
                }
            };
        let project_options = self
            .state
            .projects
            .iter()
            .map(|project| {
                let label = if project.host_device_id.is_some() {
                    format!("{}（远程暂不支持）", project.name)
                } else {
                    project.name.clone()
                };
                SelectOption::new(project.id.clone(), SharedString::from(label))
            })
            .collect::<Vec<_>>();
        let workspace_options = worktrees
            .worktrees
            .iter()
            .filter(|worktree| worktree.exists)
            .map(|worktree| {
                let label = if worktree.is_default {
                    selected_project
                        .as_ref()
                        .map(|project| project.name.clone())
                        .unwrap_or_else(|| worktree.name.clone())
                } else {
                    format!("{} · {}", worktree.name, worktree.branch)
                };
                SelectOption::new(worktree.id.clone(), SharedString::from(label))
            })
            .collect::<Vec<_>>();
        let branch_options = selected_project
            .as_ref()
            .map(|project| {
                automation_branch_options_for_path(&project.path, &self.automation_base_branch)
            })
            .unwrap_or_default()
            .into_iter()
            .map(|branch| SelectOption::new(branch.clone(), SharedString::from(branch)))
            .collect::<Vec<_>>();
        let agent_options = [
            ("claude", "Claude"),
            ("kiro_gateway_claude", "Claude + Kiro"),
            ("codex", "Codex"),
            ("kiro", "Kiro"),
        ]
        .into_iter()
        .map(|(value, label)| SelectOption::new(value, label))
        .collect::<Vec<_>>();
        let model_options = automation_gateway_model_options(&self.automation_model);
        let schedule_options = [
            ("hourly", "每小时"),
            ("daily", "每天"),
            ("weekdays", "工作日"),
            ("weekly", "每周"),
            ("custom", "自定义 Cron"),
        ]
        .into_iter()
        .map(|(value, label)| SelectOption::new(value, label))
        .collect::<Vec<_>>();
        let grace_options = automation_catch_up_grace_options();
        let catch_up_grace_value = self.automation_catch_up_grace_seconds.to_string();
        let name_value = name_input.read(cx).value();
        let schedule_value = schedule_input.read(cx).value();
        let timezone_value = timezone_input.read(cx).value();
        let prompt_value = prompt_input.read(cx).value();
        let project_label = selected_project
            .as_ref()
            .map(|project| project.name.as_str())
            .unwrap_or("未选择项目");
        let agent_label = match self.automation_agent {
            AutomationAgent::Claude => "Claude",
            AutomationAgent::KiroGatewayClaude => "Claude + Kiro",
            AutomationAgent::Codex => "Codex",
            AutomationAgent::Kiro => "Kiro",
        };
        let schedule_label = match self.automation_schedule_preset {
            AutomationSchedulePreset::Hourly => "每小时".to_string(),
            AutomationSchedulePreset::Daily => format!("每天 {}", schedule_value),
            AutomationSchedulePreset::Weekdays => format!("工作日 {}", schedule_value),
            AutomationSchedulePreset::Weekly => format!("每周 {}", schedule_value),
            AutomationSchedulePreset::Custom => "自定义计划".to_string(),
        };
        let page_title = if is_editing {
            "自动任务 / 编辑任务"
        } else {
            "自动任务 / 新建任务"
        };
        let page_summary = format!("{} · {} · {}", schedule_label, project_label, agent_label);
        let basic_complete = !name_value.trim().is_empty() && selected_project_is_local;
        let runtime_complete = can_save;
        let schedule_complete = !timezone_value.trim().is_empty()
            && (self.automation_schedule_preset == AutomationSchedulePreset::Hourly
                || !schedule_value.trim().is_empty());
        let content_complete = !prompt_value.trim().is_empty();
        sync_select_state(
            project_select,
            project_options,
            &self.automation_project_id,
            window,
            cx,
        );
        sync_select_state(
            workspace_select,
            workspace_options,
            &self.automation_workspace_id,
            window,
            cx,
        );
        sync_select_state(
            branch_select,
            branch_options,
            &self.automation_base_branch,
            window,
            cx,
        );
        sync_select_state(
            agent_select,
            agent_options,
            automation_agent_value(self.automation_agent),
            window,
            cx,
        );
        sync_select_state(
            model_select,
            model_options,
            &self.automation_model,
            window,
            cx,
        );
        sync_select_state(
            schedule_select,
            schedule_options,
            automation_schedule_preset_value(self.automation_schedule_preset),
            window,
            cx,
        );
        sync_select_state(
            grace_select,
            grace_options,
            &catch_up_grace_value,
            window,
            cx,
        );

        form_page(
            page_title,
            page_summary,
            div()
                    .w_full()
                    .flex()
                    .flex_col()
                    .gap(px(20.0))
                    .when(!is_editing, |this| {
                        this.child(
                            div()
                                .min_h(px(34.0))
                                .flex()
                                .items_center()
                                .gap(px(8.0))
                                .child(
                                    div()
                                        .mr(px(2.0))
                                        .text_size(rems(0.7))
                                        .text_color(cx.theme().muted_foreground)
                                        .child("套用模板"),
                                )
                                .children(AUTOMATION_TEMPLATES.into_iter().map(|template| {
                                    automation_template_button(template, cx.entity())
                                })),
                        )
                    })
                    .child(form_card(
                        "基本信息",
                        "设置任务名称和所属项目。",
                        basic_complete,
                        responsive_form(window, 2, 1)
                            .child(form_input_field("任务名称", name_input, false, cx))
                            .child(form_control_field(
                                "项目",
                                ui_select(
                                    project_select,
                                    "选择项目",
                                    relative(1.0),
                                    px(520.0),
                                    false,
                                ),
                                cx,
                            )),
                        cx,
                    ))
                    .child(form_card(
                        "运行环境",
                        "选择运行位置、智能体和会话策略。",
                        runtime_complete,
                        responsive_form(window, 4, 2)
                            .child(
                                form_control_field(
                                    "工作区",
                                    div()
                                        .w_full()
                                        .flex()
                                        .items_center()
                                        .gap(px(8.0))
                                        .child(div().flex_none().child(segmented_control(
                                            "automation-workspace-mode",
                                            automation_workspace_mode_value(
                                                self.automation_workspace_mode,
                                            ),
                                            vec![
                                                SegmentedOption::new("existing", "工作树"),
                                                SegmentedOption::new("new", "新运行"),
                                            ],
                                            cx.entity(),
                                            |app, value, _window, cx| {
                                                if let Some(mode) =
                                                    automation_workspace_mode_from_value(&value)
                                                {
                                                    app.set_automation_workspace_mode(mode, cx);
                                                }
                                            },
                                            cx,
                                        )))
                                        .child(div().flex_1().min_w_0().child(
                                            if self.automation_workspace_mode
                                                == AutomationWorkspaceMode::Existing
                                            {
                                                ui_select(
                                                    workspace_select,
                                                    "选择工作树",
                                                    relative(1.0),
                                                    px(360.0),
                                                    !selected_project_is_local,
                                                )
                                            } else {
                                                ui_select(
                                                    branch_select,
                                                    "选择来源分支",
                                                    relative(1.0),
                                                    px(360.0),
                                                    !selected_project_is_local,
                                                )
                                            },
                                        )),
                                    cx,
                                )
                                .col_span(2),
                            )
                            .child(form_control_field(
                                "智能体",
                                ui_select(
                                    agent_select,
                                    "选择智能体",
                                    relative(1.0),
                                    px(280.0),
                                    false,
                                ),
                                cx,
                            ))
                            .when(
                                self.automation_agent == AutomationAgent::KiroGatewayClaude,
                                |this| {
                                    this.child(form_control_field(
                                        "模型",
                                        ui_select(
                                            model_select,
                                            "选择模型",
                                            relative(1.0),
                                            px(300.0),
                                            false,
                                        ),
                                        cx,
                                    ))
                                },
                            )
                            .child(form_control_field(
                                "会话",
                                segmented_control(
                                    "automation-session-mode",
                                    if self.automation_reuse_session { "reuse" } else { "new" },
                                    vec![
                                        SegmentedOption::new("new", "新建会话"),
                                        SegmentedOption::new("reuse", "复用会话").disabled(
                                            self.automation_workspace_mode
                                                != AutomationWorkspaceMode::Existing,
                                        ),
                                    ],
                                    cx.entity(),
                                    |app, value, _window, cx| {
                                        app.automation_reuse_session = value == "reuse";
                                        app.invalidate_ui(cx, [UiRegion::WorkspaceBody]);
                                    },
                                    cx,
                                ),
                                cx,
                            ))
                            .when(!selected_project_is_local, |this| {
                                this.child(
                                    field().label_indent(false).col_span(4).child(
                                        div()
                                            .text_size(rems(0.68))
                                            .text_color(cx.theme().danger)
                                            .child("请选择本地项目；远程项目暂不支持自动任务。"),
                                    ),
                                )
                            })
                            .when(
                                self.automation_workspace_mode
                                    == AutomationWorkspaceMode::NewPerRun,
                                |this| {
                                    this.child(
                                        field().label_indent(false).col_span(4).child(
                                            div()
                                                .text_size(rems(0.68))
                                                .text_color(cx.theme().muted_foreground)
                                                .child(
                                                "新运行会从所选分支创建独立 Worktree，并强制使用新会话。",
                                            ),
                                        ),
                                    )
                                },
                            ),
                        cx,
                    ))
                    .child(form_card(
                        "执行计划",
                        "设置任务何时运行，以及错过计划后的补跑范围。",
                        schedule_complete,
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(12.0))
                            .child(
                                responsive_form(
                                    window,
                                    if self.automation_schedule_preset
                                        == AutomationSchedulePreset::Hourly
                                    {
                                        3
                                    } else {
                                        4
                                    },
                                    2,
                                )
                                    .child(form_control_field(
                                        "日程",
                                        ui_select(
                                            schedule_select,
                                            "选择日程",
                                            relative(1.0),
                                            px(280.0),
                                            false,
                                        ),
                                        cx,
                                    ))
                                    .when(
                                        self.automation_schedule_preset
                                            != AutomationSchedulePreset::Hourly,
                                        |this| {
                                            this.child(form_input_field(
                                                if self.automation_schedule_preset
                                                    == AutomationSchedulePreset::Custom
                                                {
                                                    "Cron 表达式"
                                                } else {
                                                    "执行时间"
                                                },
                                                schedule_input,
                                                false,
                                                cx,
                                            ))
                                        },
                                    )
                                    .child(form_input_field(
                                        "时区",
                                        timezone_input,
                                        false,
                                        cx,
                                    ))
                                    .child(form_control_field(
                                        "补跑时限",
                                        ui_select(
                                            grace_select,
                                            "选择补跑时限",
                                            relative(1.0),
                                            px(280.0),
                                            false,
                                        ),
                                        cx,
                                    )),
                            )
                            .when(
                                self.automation_schedule_preset
                                    == AutomationSchedulePreset::Weekly,
                                |this| {
                                    this.child(
                                        div()
                                            .flex()
                                            .flex_wrap()
                                            .gap(px(6.0))
                                            .children((1..=7).map(|day| {
                                                automation_weekday_button(
                                                    day,
                                                    self.automation_weekday,
                                                    cx.entity(),
                                                )
                                            })),
                                    )
                                },
                            )
                            .child(
                                div()
                                    .text_size(rems(0.68))
                                    .text_color(cx.theme().muted_foreground)
                                    .child(
                                        "App 关闭或设备休眠错过计划后，只会在补跑时限内补跑一次。",
                                    ),
                            ),
                        cx,
                    ))
                    .child(form_card(
                        "任务内容",
                        "描述要交给智能体完成的工作。",
                        content_complete,
                        responsive_form(window, 1, 1)
                            .child(form_input_field("任务提示词", prompt_input, true, cx)),
                        cx,
                    ))
                    .child(form_card(
                        "高级设置",
                        "设置可选的执行前检查。",
                        false,
                        responsive_form(window, 2, 1)
                            .child(form_input_field(
                                "执行前检查（可选）",
                                precheck_input,
                                false,
                                cx,
                            ))
                            .child(form_input_field(
                                "检查超时（秒）",
                                precheck_timeout_input,
                                false,
                                cx,
                            )),
                        cx,
                    ))
                    .child(form_action_bar(
                        [
                                Button::new("automation-editor-close")
                                    .secondary()
                                    .label("取消")
                                    .on_click({
                                        let app_entity = app_entity.clone();
                                        move |_, _window, cx| {
                                            cx.update_entity(&app_entity, |app, cx| {
                                                app.close_automation_editor(cx);
                                            });
                                        }
                                    })
                                    .into_any_element(),
                                Button::new("automation-save")
                                    .primary()
                                    .disabled(!can_save)
                                    .label(if is_editing {
                                        "保存修改"
                                    } else {
                                        "创建任务"
                                    })
                                    .on_click(move |_, window, cx| {
                                        cx.update_entity(&app_entity, |app, cx| {
                                            app.save_automation(window, cx);
                                        });
                                    })
                                    .into_any_element(),
                        ],
                        cx,
                    )),
            cx,
        )
    }
}
