use super::*;
use crate::ui::select::{SelectOption, select_control as ui_select, sync_select_state};
use crate::ui::{
    SegmentedOption, form_action_bar, form_card, form_control_field, form_field_label,
    form_input_field, form_page, responsive_form, segmented_control,
};
use chrono::{DateTime, Local};
use gpui_component::{WindowExt, button::ButtonVariant, dialog::DialogButtonProps, form::field};

mod editor;
mod selects;
mod views;

use views::*;

const AUTOMATION_OUTPUT_BUFFER_LIMIT: usize = 512 * 1024;
const AUTOMATION_LIST_ROW_HEIGHT: f32 = 82.0;
const AUTOMATION_LIST_ROW_GAP: f32 = 10.0;

#[derive(Clone, Copy)]
struct AutomationTemplate {
    id: &'static str,
    title: &'static str,
    description: &'static str,
    prompt: &'static str,
    preset: AutomationSchedulePreset,
}

const AUTOMATION_TEMPLATES: [AutomationTemplate; 3] = [
    AutomationTemplate {
        id: "daily-review",
        title: "每日变更检查",
        description: "检查当前工作区变更、风险和未完成事项。",
        prompt: "检查当前工作区的代码变更，整理风险、遗漏测试和下一步建议。只汇报，不要修改代码。",
        preset: AutomationSchedulePreset::Daily,
    },
    AutomationTemplate {
        id: "weekday-todo",
        title: "工作日待办整理",
        description: "每天汇总项目内待处理事项。",
        prompt: "检查项目中的 TODO、FIXME、需求文档和最近变更，给出今天最值得处理的事项列表。",
        preset: AutomationSchedulePreset::Weekdays,
    },
    AutomationTemplate {
        id: "hourly-health",
        title: "项目健康巡检",
        description: "每小时检查仓库和任务运行状态。",
        prompt: "检查当前项目状态、未提交变更和最近失败信息。仅在发现异常时给出简洁报告。",
        preset: AutomationSchedulePreset::Hourly,
    },
];

impl WeCodeApp {
    pub(in crate::app) fn automation_workspace_body(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let app_entity = cx.entity();
        let definitions = self.automation_snapshot.definitions.clone();
        let runs = self.automation_snapshot.runs.clone();
        let selected_id = self
            .automation_selected_id
            .clone()
            .filter(|selected| definitions.iter().any(|item| &item.id == selected))
            .or_else(|| definitions.first().map(|item| item.id.clone()));
        let selected = selected_id
            .as_deref()
            .and_then(|id| definitions.iter().find(|item| item.id == id))
            .cloned();
        let selected_runs = selected_id
            .as_deref()
            .map(|id| {
                runs.iter()
                    .rev()
                    .filter(|run| run.automation_id == id)
                    .cloned()
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let editor_open = self.automation_editor_open;

        div()
            .flex()
            .flex_col()
            .flex_1()
            .size_full()
            .min_h_0()
            .min_w_0()
            .bg(theme::vibrancy_panel(cx.theme().sidebar))
            .when(!editor_open, |this| {
                this.child(
                    div()
                        .h(px(58.0))
                        .flex_none()
                        .px(px(18.0))
                        .flex()
                        .items_center()
                        .justify_between()
                        .border_b_1()
                        .border_color(cx.theme().border)
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .gap(px(2.0))
                                .child(
                                    div()
                                        .text_size(rems(1.0))
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .text_color(cx.theme().foreground)
                                        .child("自动任务"),
                                )
                                .child(
                                    div()
                                        .text_size(rems(0.7))
                                        .text_color(cx.theme().muted_foreground)
                                        .child("按计划运行 Agent，并保留每次运行结果。"),
                                ),
                        )
                        .child(
                            Button::new("automation-new")
                                .primary()
                                .with_size(Size::Small)
                                .label("＋ 新建任务")
                                .on_click(move |_, window, cx| {
                                    cx.update_entity(&app_entity, |app, cx| {
                                        app.open_automation_create(window, cx);
                                    });
                                }),
                        ),
                )
            })
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .min_w_0()
                    .flex()
                    .child(self.automation_list_panel(
                        definitions,
                        &runs,
                        selected_id.as_deref(),
                        cx,
                    ))
                    .child(
                        div()
                            .flex_1()
                            .min_h_0()
                            .min_w_0()
                            .overflow_y_scrollbar()
                            .child(if editor_open {
                                self.automation_editor_panel(window, cx)
                            } else if let Some(definition) = selected {
                                self.automation_detail_panel(definition, selected_runs, cx)
                            } else {
                                self.automation_empty_panel(window, cx)
                            }),
                    ),
            )
    }

    fn automation_list_panel(
        &self,
        definitions: Vec<wecode_runtime::automation::AutomationDefinition>,
        runs: &[AutomationRun],
        selected_id: Option<&str>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let app_entity = cx.entity();
        div()
            .w(px(300.0))
            .min_h_0()
            .flex_none()
            .flex()
            .flex_col()
            .border_r_1()
            .border_color(cx.theme().border)
            .child(
                div()
                    .h(px(42.0))
                    .flex_none()
                    .px(px(14.0))
                    .flex()
                    .items_center()
                    .justify_between()
                    .text_size(rems(0.75))
                    .text_color(cx.theme().muted_foreground)
                    .child("全部任务")
                    .child(format!("{}", definitions.len())),
            )
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scrollbar()
                    .p(px(8.0))
                    .flex()
                    .flex_col()
                    .when(definitions.is_empty(), |this| {
                        this.child(
                            div()
                                .px(px(10.0))
                                .py(px(24.0))
                                .text_size(rems(0.75))
                                .text_color(cx.theme().muted_foreground)
                                .child("还没有自动任务"),
                        )
                    })
                    .children(definitions.into_iter().map(|definition| {
                        let latest = runs
                            .iter()
                            .rev()
                            .find(|run| run.automation_id == definition.id)
                            .cloned();
                        let active = selected_id == Some(definition.id.as_str());
                        automation_list_row(definition, latest, active, app_entity.clone(), cx)
                    })),
            )
            .into_any_element()
    }

    fn automation_empty_panel(&self, _window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        div()
            .w_full()
            .p(px(28.0))
            .flex()
            .flex_col()
            .gap(px(16.0))
            .child(
                div()
                    .text_size(rems(1.1))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(cx.theme().foreground)
                    .child("从模板开始"),
            )
            .child(
                div()
                    .text_size(rems(0.75))
                    .text_color(cx.theme().muted_foreground)
                    .child("选择一个常用任务，再根据项目调整提示词和时间。"),
            )
            .child(
                div().grid().grid_cols(3).gap(px(12.0)).children(
                    AUTOMATION_TEMPLATES
                        .into_iter()
                        .map(|template| automation_template_card(template, cx.entity(), cx)),
                ),
            )
            .into_any_element()
    }

    fn automation_selected_project(&self) -> Option<&ProjectInfo> {
        self.state
            .projects
            .iter()
            .find(|project| project.id == self.automation_project_id)
    }

    fn automation_worktrees_for_project(&self, project: &ProjectInfo) -> WorktreeSummary {
        WorktreeService::new(self.state.support_dir.clone())
            .state_summary(Some(&project.id), Some(&project.path))
    }

    fn select_automation_project(&mut self, project_id: String, cx: &mut Context<Self>) {
        self.automation_project_id = project_id;
        self.automation_reuse_session = false;
        let Some(project) = self.automation_selected_project().cloned() else {
            self.automation_workspace_id.clear();
            self.automation_base_branch.clear();
            self.invalidate_ui(cx, [UiRegion::WorkspaceBody]);
            return;
        };
        let worktrees = self.automation_worktrees_for_project(&project);
        self.automation_workspace_id = worktrees
            .selected_worktree_id
            .clone()
            .or_else(|| {
                worktrees
                    .worktrees
                    .first()
                    .map(|worktree| worktree.id.clone())
            })
            .unwrap_or_else(|| project.id.clone());
        self.automation_base_branch = automation_default_branch_for_path(&project.path);
        self.invalidate_ui(cx, [UiRegion::WorkspaceBody]);
    }

    fn automation_detail_panel(
        &self,
        definition: wecode_runtime::automation::AutomationDefinition,
        runs: Vec<AutomationRun>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let app_entity = cx.entity();
        let run_id = definition.id.clone();
        let edit_id = definition.id.clone();
        let toggle_id = definition.id.clone();
        let delete_id = definition.id.clone();
        let enabled = definition.enabled;
        let latest = runs.first().cloned();
        div()
            .w_full()
            .p(px(24.0))
            .flex()
            .flex_col()
            .gap(px(18.0))
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .items_start()
                    .justify_between()
                    .gap(px(16.0))
                    .child(
                        div()
                            .min_w_0()
                            .flex_1()
                            .flex()
                            .flex_col()
                            .gap(px(5.0))
                            .child(
                                div()
                                    .text_size(rems(1.15))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(cx.theme().foreground)
                                    .child(definition.name.clone()),
                            )
                            .child(
                                div()
                                    .text_size(rems(0.75))
                                    .text_color(cx.theme().muted_foreground)
                                    .child(format!(
                                        "{} · {} · {}",
                                        definition.agent.label(),
                                        definition.schedule.display(),
                                        definition.workspace_name
                                    )),
                            ),
                    )
                    .child(
                        div()
                            .flex_none()
                            .flex()
                            .flex_wrap()
                            .items_center()
                            .justify_end()
                            .gap(px(7.0))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(6.0))
                                    .child(
                                        div()
                                            .text_size(rems(0.7))
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(if enabled {
                                                cx.theme().foreground
                                            } else {
                                                cx.theme().muted_foreground
                                            })
                                            .child(if enabled { "启用" } else { "暂停" }),
                                    )
                                    .child(
                                        Switch::new(format!("automation-enabled-{toggle_id}"))
                                            .checked(enabled)
                                            .with_size(Size::Small)
                                            .on_click({
                                                let app_entity = app_entity.clone();
                                                let toggle_id = toggle_id.clone();
                                                move |_, _window, cx| {
                                                    cx.update_entity(&app_entity, |app, cx| {
                                                        app.toggle_automation(
                                                            &toggle_id, !enabled, cx,
                                                        );
                                                    });
                                                }
                                            }),
                                    ),
                            )
                            .child(
                                Button::new(format!("automation-run-{run_id}"))
                                    .primary()
                                    .compact()
                                    .with_size(Size::Small)
                                    .label("立即运行")
                                    .on_click(move |_, _window, cx| {
                                        cx.update_entity(&app_entity, |app, cx| {
                                            app.run_automation_now(&run_id, cx);
                                        });
                                    }),
                            )
                            .child(
                                Button::new(format!("automation-edit-{edit_id}"))
                                    .secondary()
                                    .compact()
                                    .with_size(Size::Small)
                                    .label("编辑")
                                    .on_click({
                                        let app_entity = cx.entity();
                                        move |_, window, cx| {
                                            cx.update_entity(&app_entity, |app, cx| {
                                                app.open_automation_edit(&edit_id, window, cx);
                                            });
                                        }
                                    }),
                            )
                            .child(
                                Button::new(format!("automation-toggle-{toggle_id}"))
                                    .secondary()
                                    .compact()
                                    .with_size(Size::Small)
                                    .label(if enabled {
                                        "暂停任务"
                                    } else {
                                        "启用任务"
                                    })
                                    .on_click({
                                        let app_entity = cx.entity();
                                        move |_, _window, cx| {
                                            cx.update_entity(&app_entity, |app, cx| {
                                                app.toggle_automation(&toggle_id, !enabled, cx);
                                            });
                                        }
                                    }),
                            )
                            .child(
                                Button::new(format!("automation-delete-{delete_id}"))
                                    .custom(
                                        ButtonCustomVariant::new(cx)
                                            .color(cx.theme().danger)
                                            .foreground(cx.theme().primary_foreground)
                                            .hover(cx.theme().danger.opacity(0.86))
                                            .active(cx.theme().danger.opacity(0.72)),
                                    )
                                    .compact()
                                    .with_size(Size::Small)
                                    .label("删除")
                                    .on_click({
                                        let app_entity = cx.entity();
                                        move |_, window, cx| {
                                            cx.update_entity(&app_entity, |app, cx| {
                                                app.request_delete_automation(
                                                    delete_id.clone(),
                                                    window,
                                                    cx,
                                                );
                                            });
                                        }
                                    }),
                            ),
                    ),
            )
            .child(
                div()
                    .flex()
                    .gap(px(4.0))
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .child(automation_tab_button(
                        AutomationDetailTab::Overview,
                        "概览",
                        self.automation_detail_tab,
                        cx.entity(),
                        cx,
                    ))
                    .child(automation_tab_button(
                        AutomationDetailTab::Runs,
                        &format!("运行记录 {}", runs.len()),
                        self.automation_detail_tab,
                        cx.entity(),
                        cx,
                    )),
            )
            .child(match self.automation_detail_tab {
                AutomationDetailTab::Overview => {
                    automation_overview(definition, latest, cx).into_any_element()
                }
                AutomationDetailTab::Runs => automation_runs(runs, cx.entity(), cx),
            })
            .into_any_element()
    }

    fn open_automation_create(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.automation_editor_open = true;
        self.automation_editing_id = None;
        self.automation_workspace_mode = AutomationWorkspaceMode::Existing;
        self.prepare_automation_target_from_current_project();
        self.automation_reuse_session = false;
        self.automation_catch_up_grace_seconds = DEFAULT_CATCH_UP_GRACE_SECONDS;
        self.automation_schedule_preset = AutomationSchedulePreset::Daily;
        self.automation_weekday = 1;
        self.automation_agent = AutomationAgent::Claude;
        self.set_automation_inputs("", "09:00", "Asia/Shanghai", "", "", "60", window, cx);
        self.invalidate_ui(cx, [UiRegion::WorkspaceBody]);
    }

    fn open_automation_edit(&mut self, id: &str, window: &mut Window, cx: &mut Context<Self>) {
        let Some(definition) = self
            .automation_snapshot
            .definitions
            .iter()
            .find(|item| item.id == id)
            .cloned()
        else {
            self.show_toast("自动任务不存在".to_string(), cx);
            return;
        };
        self.automation_editor_open = true;
        self.automation_editing_id = Some(definition.id.clone());
        self.automation_project_id = definition.project_id.clone();
        self.automation_workspace_id = definition.workspace_id.clone();
        self.automation_workspace_mode = definition.workspace_mode;
        self.automation_base_branch = definition
            .base_branch
            .clone()
            .unwrap_or_else(|| self.automation_default_base_branch());
        self.automation_schedule_preset = definition.schedule.preset();
        self.automation_weekday = match &definition.schedule {
            AutomationSchedule::Weekly { weekdays, .. } => weekdays.first().copied().unwrap_or(1),
            _ => 1,
        };
        self.automation_agent = definition.agent;
        self.automation_reuse_session = definition.reuse_session
            && definition.workspace_mode == AutomationWorkspaceMode::Existing;
        self.automation_catch_up_grace_seconds = definition.catch_up_grace_seconds;
        self.set_automation_inputs(
            &definition.name,
            &definition.schedule.editor_value(),
            &definition.timezone,
            &definition.prompt,
            definition.precheck_command.as_deref().unwrap_or(""),
            &definition.precheck_timeout_seconds.to_string(),
            window,
            cx,
        );
        self.invalidate_ui(cx, [UiRegion::WorkspaceBody]);
    }

    fn close_automation_editor(&mut self, cx: &mut Context<Self>) {
        self.automation_editor_open = false;
        self.automation_editing_id = None;
        self.invalidate_ui(cx, [UiRegion::WorkspaceBody]);
    }

    fn set_automation_workspace_mode(
        &mut self,
        mode: AutomationWorkspaceMode,
        cx: &mut Context<Self>,
    ) {
        self.automation_workspace_mode = mode;
        if mode == AutomationWorkspaceMode::NewPerRun {
            self.automation_reuse_session = false;
            if self.automation_base_branch.trim().is_empty() {
                self.automation_base_branch = self
                    .automation_selected_project()
                    .map(|project| automation_default_branch_for_path(&project.path))
                    .unwrap_or_else(|| self.automation_default_base_branch());
            }
        }
        self.invalidate_ui(cx, [UiRegion::WorkspaceBody]);
    }

    fn set_automation_inputs(
        &mut self,
        name: &str,
        schedule: &str,
        timezone: &str,
        prompt: &str,
        precheck: &str,
        timeout: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let values = [
            (self.automation_name_input.clone(), name),
            (self.automation_schedule_input.clone(), schedule),
            (self.automation_timezone_input.clone(), timezone),
            (self.automation_prompt_input.clone(), prompt),
            (self.automation_precheck_input.clone(), precheck),
            (self.automation_precheck_timeout_input.clone(), timeout),
        ];
        for (input, value) in values {
            if let Some(input) = input {
                input.update(cx, |state, cx| state.set_value(value, window, cx));
            }
        }
    }

    fn apply_automation_template(
        &mut self,
        template: AutomationTemplate,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.automation_editor_open = true;
        self.automation_editing_id = None;
        self.automation_workspace_mode = AutomationWorkspaceMode::Existing;
        self.prepare_automation_target_from_current_project();
        self.automation_reuse_session = false;
        self.automation_catch_up_grace_seconds = DEFAULT_CATCH_UP_GRACE_SECONDS;
        self.automation_schedule_preset = template.preset;
        self.automation_weekday = 1;
        self.automation_agent = AutomationAgent::Claude;
        self.set_automation_inputs(
            template.title,
            "09:00",
            "Asia/Shanghai",
            template.prompt,
            "",
            "60",
            window,
            cx,
        );
        self.invalidate_ui(cx, [UiRegion::WorkspaceBody]);
    }

    fn set_automation_schedule_preset(
        &mut self,
        preset: AutomationSchedulePreset,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.automation_schedule_preset = preset;
        if let Some(input) = self.automation_schedule_input.clone() {
            let current = input.read(cx).value().to_string();
            let next = match preset {
                AutomationSchedulePreset::Custom if current.contains(':') => "0 9 * * 1-5",
                AutomationSchedulePreset::Custom => current.as_str(),
                _ if !current.contains(':') || current.contains(' ') => "09:00",
                _ => current.as_str(),
            }
            .to_string();
            input.update(cx, |state, cx| state.set_value(next, window, cx));
        }
        self.invalidate_ui(cx, [UiRegion::WorkspaceBody]);
    }

    fn automation_default_base_branch(&self) -> String {
        self.state
            .git
            .branches
            .iter()
            .find(|branch| branch.is_current)
            .or_else(|| self.state.git.branches.first())
            .map(|branch| branch.name.trim().to_string())
            .filter(|branch| !branch.is_empty())
            .or_else(|| {
                super::ai_runtime_status::selected_worktree_info(&self.state)
                    .map(|worktree| worktree.branch.trim().to_string())
            })
            .filter(|branch| !branch.is_empty())
            .unwrap_or_else(|| self.state.git.branch.trim().to_string())
    }

    fn prepare_automation_target_from_current_project(&mut self) {
        let project = self.state.selected_project.clone().or_else(|| {
            self.state
                .projects
                .iter()
                .find(|project| project.host_device_id.is_none() && project.exists)
                .cloned()
        });
        let Some(project) = project else {
            self.automation_project_id.clear();
            self.automation_workspace_id.clear();
            self.automation_base_branch.clear();
            return;
        };
        let worktrees = self.automation_worktrees_for_project(&project);
        self.automation_project_id = project.id.clone();
        self.automation_workspace_id = worktrees
            .selected_worktree_id
            .clone()
            .or_else(|| {
                worktrees
                    .worktrees
                    .iter()
                    .find(|worktree| worktree.is_default)
                    .or_else(|| worktrees.worktrees.first())
                    .map(|worktree| worktree.id.clone())
            })
            .unwrap_or_else(|| project.id.clone());
        self.automation_base_branch = automation_default_branch_for_path(&project.path);
    }

    fn save_automation(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(project) = self.automation_selected_project().cloned() else {
            self.show_toast("请先选择项目".to_string(), cx);
            return;
        };
        if project.host_device_id.is_some() || !project.exists {
            self.show_toast("自动任务目前只支持本地项目".to_string(), cx);
            return;
        }
        let Some(name_input) = self.automation_name_input.clone() else {
            return;
        };
        let Some(schedule_input) = self.automation_schedule_input.clone() else {
            return;
        };
        let Some(timezone_input) = self.automation_timezone_input.clone() else {
            return;
        };
        let Some(prompt_input) = self.automation_prompt_input.clone() else {
            return;
        };
        let Some(precheck_input) = self.automation_precheck_input.clone() else {
            return;
        };
        let Some(timeout_input) = self.automation_precheck_timeout_input.clone() else {
            return;
        };
        let timeout = match timeout_input.read(cx).value().trim().parse::<u64>() {
            Ok(timeout) if (1..=1800).contains(&timeout) => timeout,
            _ => {
                self.show_toast("检查超时必须是 1-1800 秒".to_string(), cx);
                return;
            }
        };
        let schedule_value = schedule_input.read(cx).value().trim().to_string();
        let schedule_spec = match self.automation_schedule_preset {
            AutomationSchedulePreset::Hourly => "cron:0 * * * *".to_string(),
            AutomationSchedulePreset::Daily => format!("daily:{schedule_value}"),
            AutomationSchedulePreset::Weekdays => {
                format!("weekly:1,2,3,4,5@{schedule_value}")
            }
            AutomationSchedulePreset::Weekly => {
                format!("weekly:{}@{schedule_value}", self.automation_weekday)
            }
            AutomationSchedulePreset::Custom => format!("cron:{schedule_value}"),
        };
        let (workspace_id, workspace_name, workspace_path) =
            if self.automation_workspace_mode == AutomationWorkspaceMode::NewPerRun {
                (
                    project.id.clone(),
                    project.name.clone(),
                    project.path.clone(),
                )
            } else {
                let worktrees = self.automation_worktrees_for_project(&project);
                let Some(worktree) = worktrees.worktrees.iter().find(|worktree| {
                    worktree.id == self.automation_workspace_id && worktree.exists
                }) else {
                    self.show_toast("请选择有效的工作树".to_string(), cx);
                    return;
                };
                (
                    worktree.id.clone(),
                    automation_workspace_name(&project.name, Some(worktree)),
                    worktree.path.clone(),
                )
            };
        let request = AutomationCreateRequest {
            name: name_input.read(cx).value().to_string(),
            project_id: project.id.clone(),
            project_name: project.name.clone(),
            workspace_id,
            workspace_name,
            workspace_path,
            workspace_mode: self.automation_workspace_mode,
            project_path: project.path.clone(),
            base_branch: (self.automation_workspace_mode == AutomationWorkspaceMode::NewPerRun)
                .then(|| self.automation_base_branch.trim().to_string()),
            reuse_session: self.automation_reuse_session
                && self.automation_workspace_mode == AutomationWorkspaceMode::Existing,
            host_device_id: project.host_device_id.clone(),
            agent: self.automation_agent,
            prompt: prompt_input.read(cx).value().to_string(),
            precheck_command: Some(precheck_input.read(cx).value().to_string()),
            precheck_timeout_seconds: timeout,
            schedule_spec,
            timezone: timezone_input.read(cx).value().to_string(),
            catch_up_grace_seconds: self.automation_catch_up_grace_seconds,
        };
        let service = self.automation_service();
        let result = if let Some(id) = self.automation_editing_id.clone() {
            service.update_definition(&id, request, app_now_seconds() as i64)
        } else {
            service.create(request, app_now_seconds() as i64)
        };
        match result {
            Ok(definition) => {
                self.automation_selected_id = Some(definition.id);
                self.automation_editor_open = false;
                self.automation_editing_id = None;
                self.automation_detail_tab = AutomationDetailTab::Overview;
                self.refresh_automation_snapshot(cx);
                self.show_toast("自动任务已保存".to_string(), cx);
            }
            Err(error) => self.show_toast(error, cx),
        }
        let _ = window;
    }

    pub(in crate::app) fn tick_automations(&mut self, cx: &mut Context<Self>) {
        let service = self.automation_service();
        match service.claim_due(app_now_seconds() as i64) {
            Ok(plans) => {
                for plan in plans {
                    self.dispatch_automation_plan(plan);
                }
                self.automation_snapshot = service.snapshot();
                if self.workspace_view == WorkspaceView::Automations {
                    self.invalidate_ui(cx, [UiRegion::WorkspaceBody]);
                }
            }
            Err(error) => {
                self.status_message = format!("自动任务调度失败：{error}");
                self.invalidate_status_bar(cx);
            }
        }
    }

    fn dispatch_automation_plan(&mut self, plan: AutomationRunPlan) {
        let service = self.automation_service();
        let now = app_now_seconds() as i64;
        if plan.host_device_id.is_some() {
            let _ =
                service.mark_failed(&plan.run_id, "远程项目的自动任务暂不支持".to_string(), now);
            return;
        }
        let terminal_id = format!("automation-{}", plan.run_id);
        let support_dir = self.state.support_dir.clone();
        let runtime_root = self.runtime.root.clone();
        let tool_permissions_file = self
            .state
            .tool_permissions
            .error
            .is_none()
            .then(|| PathBuf::from(&self.state.tool_permissions.path));
        let terminal_manager = self.terminal_manager.clone();
        let _ = std::thread::Builder::new()
            .name(format!("wecode-automation-{}", plan.run_id))
            .spawn(move || {
                let mut workspace_id = plan.workspace_id.clone();
                let mut workspace_path = plan.workspace_path.clone();
                if plan.workspace_mode == AutomationWorkspaceMode::NewPerRun {
                    if !Path::new(&plan.project_path).is_dir() {
                        let _ = service.mark_failed(
                            &plan.run_id,
                            format!("项目目录不存在：{}", plan.project_path),
                            app_now_seconds() as i64,
                        );
                        return;
                    }
                    let Some(base_branch) = plan.base_branch.as_deref() else {
                        let _ = service.mark_failed(
                            &plan.run_id,
                            "未设置 Worktree 来源分支".to_string(),
                            app_now_seconds() as i64,
                        );
                        return;
                    };
                    let branch_name = automation_worktree_branch_name(&plan);
                    let worktree_snapshot = WorktreeService::new(support_dir.clone())
                        .create_unselected_from_request(WorktreeCreateRequest {
                            project_id: plan.project_id.clone(),
                            project_path: plan.project_path.clone(),
                            base_branch: Some(base_branch.to_string()),
                            branch_name: branch_name.clone(),
                            task_title: Some(format!(
                                "自动任务 · {} · {}",
                                plan.automation_name,
                                short_automation_id(&plan.run_id)
                            )),
                        });
                    let snapshot = match worktree_snapshot {
                        Ok(snapshot) => snapshot,
                        Err(error) => {
                            let _ = service.mark_failed(
                                &plan.run_id,
                                format!("创建运行 Worktree 失败：{error}"),
                                app_now_seconds() as i64,
                            );
                            return;
                        }
                    };
                    let Some(worktree) = snapshot
                        .worktrees
                        .iter()
                        .find(|worktree| worktree.branch == branch_name)
                    else {
                        let _ = service.mark_failed(
                            &plan.run_id,
                            "运行 Worktree 已创建，但未能读取其信息".to_string(),
                            app_now_seconds() as i64,
                        );
                        return;
                    };
                    workspace_id = worktree.id.clone();
                    let workspace_name = worktree.name.clone();
                    workspace_path = worktree.path.clone();
                    if let Err(error) = service.record_run_workspace(
                        &plan.run_id,
                        workspace_id.clone(),
                        workspace_name.clone(),
                        workspace_path.clone(),
                    ) {
                        let _ = service.mark_failed(
                            &plan.run_id,
                            format!("无法保存运行 Worktree：{error}"),
                            app_now_seconds() as i64,
                        );
                        return;
                    }
                }
                if !Path::new(&workspace_path).is_dir() {
                    let _ = service.mark_failed(
                        &plan.run_id,
                        format!("工作目录不存在：{workspace_path}"),
                        app_now_seconds() as i64,
                    );
                    return;
                }
                if let Some(command) = plan.precheck_command.as_deref() {
                    let result = run_automation_precheck(
                        command,
                        &workspace_path,
                        plan.precheck_timeout_seconds,
                    );
                    if !result.passed() {
                        let _ = service.mark_skipped_precheck(
                            &plan.run_id,
                            result,
                            app_now_seconds() as i64,
                        );
                        return;
                    }
                    let _ = service.record_precheck(&plan.run_id, result);
                }
                if let Err(error) = service.mark_running(
                    &plan.run_id,
                    terminal_id.clone(),
                    app_now_seconds() as i64,
                ) {
                    let _ = service.mark_failed(
                        &plan.run_id,
                        format!("无法更新运行状态：{error}"),
                        app_now_seconds() as i64,
                    );
                    return;
                }
                let config = TerminalPtyConfig {
                    cwd: Some(workspace_path),
                    command: Some(format!(
                        "exec {}",
                        automation_agent_command(
                            plan.agent,
                            &plan.prompt,
                            plan.resume_session_id.as_deref(),
                            &plan.run_id,
                        )
                    )),
                    project_id: Some(plan.project_id.clone()),
                    project_name: Some(plan.project_name.clone()),
                    terminal_id: Some(terminal_id.clone()),
                    session_key: Some(format!("automation:{}", plan.run_id)),
                    worktree_id: Some(workspace_id),
                    title: Some(format!("自动任务 · {}", plan.automation_name)),
                    tool: Some(plan.agent.tool_name().to_string()),
                    support_dir: Some(support_dir),
                    runtime_root: Some(runtime_root),
                    tool_permissions_file,
                    ..Default::default()
                };
                let output = Arc::new(parking_lot::Mutex::new(String::new()));
                let event_output = output.clone();
                let event_service = service.clone();
                let event_run_id = plan.run_id.clone();
                let event_terminal_id = terminal_id.clone();
                let event_terminal_manager = terminal_manager.clone();
                let ai_session_recorded = Arc::new(std::sync::atomic::AtomicBool::new(
                    plan.resume_session_id.is_some() || plan.agent == AutomationAgent::Claude,
                ));
                let event_ai_session_recorded = ai_session_recorded.clone();
                if let Err(error) = terminal_manager.create(config, move |event| match event {
                    TerminalEvent::Output { text, bytes, .. } => {
                        record_automation_ai_session_if_available(
                            &event_terminal_manager,
                            &event_service,
                            &event_run_id,
                            &event_terminal_id,
                            &event_ai_session_recorded,
                        );
                        let chunk = if text.is_empty() {
                            String::from_utf8_lossy(&bytes).into_owned()
                        } else {
                            text
                        };
                        append_automation_output(&event_output, &chunk);
                    }
                    TerminalEvent::Exit { exit_code, .. } => {
                        record_automation_ai_session_if_available(
                            &event_terminal_manager,
                            &event_service,
                            &event_run_id,
                            &event_terminal_id,
                            &event_ai_session_recorded,
                        );
                        let now = app_now_seconds() as i64;
                        let snapshot = automation_output_snapshot(&event_output.lock(), now);
                        if exit_code.unwrap_or(1) == 0 {
                            let _ = event_service.mark_completed_with_output(
                                &event_run_id,
                                snapshot,
                                now,
                            );
                        } else {
                            let _ = event_service.mark_failed_with_output(
                                &event_run_id,
                                format!("Agent 进程退出，状态码：{}", exit_code.unwrap_or(-1)),
                                snapshot,
                                now,
                            );
                        }
                    }
                    TerminalEvent::Error { message, .. } => {
                        let now = app_now_seconds() as i64;
                        let snapshot = automation_output_snapshot(&event_output.lock(), now);
                        let _ = event_service.mark_failed_with_output(
                            &event_run_id,
                            message,
                            snapshot,
                            now,
                        );
                    }
                    _ => {}
                }) {
                    let _ = service.mark_failed(
                        &plan.run_id,
                        format!("无法启动 Agent：{error}"),
                        app_now_seconds() as i64,
                    );
                }
            });
    }

    fn automation_service(&self) -> AutomationService {
        AutomationService::for_support_dir(&self.state.support_dir)
    }

    fn refresh_automation_snapshot(&mut self, cx: &mut Context<Self>) {
        self.automation_snapshot = self.automation_service().snapshot();
        self.invalidate_ui(cx, [UiRegion::WorkspaceBody, UiRegion::WorkspaceChrome]);
    }

    fn run_automation_now(&mut self, id: &str, cx: &mut Context<Self>) {
        let service = self.automation_service();
        match service.enqueue_manual(id, app_now_seconds() as i64) {
            Ok(plan) => {
                self.dispatch_automation_plan(plan);
                self.automation_detail_tab = AutomationDetailTab::Runs;
                self.refresh_automation_snapshot(cx);
                self.show_toast("自动任务已启动".to_string(), cx);
            }
            Err(error) => self.show_toast(error, cx),
        }
    }

    fn toggle_automation(&mut self, id: &str, enabled: bool, cx: &mut Context<Self>) {
        let service = self.automation_service();
        match service.set_enabled(id, enabled, app_now_seconds() as i64) {
            Ok(()) => self.refresh_automation_snapshot(cx),
            Err(error) => self.show_toast(error, cx),
        }
    }

    fn request_delete_automation(
        &mut self,
        id: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let app_entity = cx.entity();
        window.open_dialog(cx, move |dialog, _window, _cx| {
            let keyboard_id = id.clone();
            let keyboard_app_entity = app_entity.clone();
            let button_id = id.clone();
            let button_app_entity = app_entity.clone();
            dialog
                .title("删除自动任务？")
                .button_props(
                    DialogButtonProps::default()
                        .ok_text("删除")
                        .ok_variant(ButtonVariant::Danger)
                        .cancel_text("取消")
                        .show_cancel(true),
                )
                .on_ok(move |_, _window, cx| {
                    cx.update_entity(&keyboard_app_entity, |app, cx| {
                        let service = app.automation_service();
                        match service.remove(&keyboard_id) {
                            Ok(()) => {
                                app.automation_selected_id = None;
                                app.refresh_automation_snapshot(cx);
                                app.show_toast("自动任务已删除".to_string(), cx);
                            }
                            Err(error) => app.show_toast(error, cx),
                        }
                    });
                    true
                })
                .footer(
                    div()
                        .w_full()
                        .flex()
                        .justify_end()
                        .gap(px(8.0))
                        .child(
                            Button::new("automation-delete-cancel")
                                .secondary()
                                .with_size(Size::Small)
                                .label("取消")
                                .on_click(|_, window, cx| {
                                    window.close_dialog(cx);
                                }),
                        )
                        .child(
                            Button::new(format!("automation-delete-confirm-{button_id}"))
                                .custom(
                                    ButtonCustomVariant::new(_cx)
                                        .color(_cx.theme().danger)
                                        .foreground(_cx.theme().primary_foreground)
                                        .hover(_cx.theme().danger.opacity(0.86))
                                        .active(_cx.theme().danger.opacity(0.72)),
                                )
                                .with_size(Size::Small)
                                .label("删除")
                                .on_click(move |_, window, cx| {
                                    cx.update_entity(&button_app_entity, |app, cx| {
                                        let service = app.automation_service();
                                        match service.remove(&button_id) {
                                            Ok(()) => {
                                                app.automation_selected_id = None;
                                                app.refresh_automation_snapshot(cx);
                                                app.show_toast("自动任务已删除".to_string(), cx);
                                            }
                                            Err(error) => app.show_toast(error, cx),
                                        }
                                    });
                                    window.close_dialog(cx);
                                }),
                        ),
                )
                .child(
                    div()
                        .px(px(16.0))
                        .py(px(12.0))
                        .text_size(rems(0.78))
                        .text_color(_cx.theme().muted_foreground)
                        .child("任务及其运行记录会一起删除，已创建的终端不会被强制关闭。"),
                )
        });
    }

    fn open_automation_terminal(
        &mut self,
        run_id: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(run) = self
            .automation_snapshot
            .runs
            .iter()
            .find(|run| run.id == run_id)
            .cloned()
        else {
            self.show_toast("运行记录不存在".to_string(), cx);
            return;
        };
        let Some(terminal_id) = run.terminal_id.clone() else {
            self.show_toast("本次运行没有可打开的终端".to_string(), cx);
            return;
        };
        let current_workspace_id = super::ai_runtime_status::selected_worktree_info(&self.state)
            .map(|worktree| worktree.id)
            .or_else(|| {
                self.state
                    .selected_project
                    .as_ref()
                    .map(|project| project.id.clone())
            });
        if current_workspace_id.as_deref() != Some(run.workspace_id.as_str()) {
            self.show_toast(
                format!("请先切换到工作区“{}”再打开终端", run.workspace_name),
                cx,
            );
            return;
        }
        if !self
            .terminal_manager
            .list()
            .iter()
            .any(|session| session.id == terminal_id)
        {
            self.show_toast("终端进程已结束，可查看保存的输出".to_string(), cx);
            return;
        }
        if self.main_terminal().is_some_and(|tab| {
            tab.panes
                .iter()
                .any(|slot| slot.terminal_id.as_deref() == Some(terminal_id.as_str()))
        }) {
            self.set_workspace_view(WorkspaceView::Terminal, window, cx);
            self.select_active_terminal_runtime_id(Some(&terminal_id));
            self.focus_active_terminal(window, cx);
            return;
        }
        let Some(definition) = self
            .automation_snapshot
            .definitions
            .iter()
            .find(|definition| definition.id == run.automation_id)
            .cloned()
        else {
            self.show_toast("自动任务已不存在".to_string(), cx);
            return;
        };
        let title = format!("自动任务 · {}", definition.name);
        let pty_config = TerminalPtyConfig {
            cwd: Some(run.workspace_path.clone()),
            project_id: Some(definition.project_id.clone()),
            project_name: Some(definition.project_name.clone()),
            terminal_id: Some(terminal_id.clone()),
            session_key: Some(format!("automation:{}", run.id)),
            worktree_id: Some(run.workspace_id.clone()),
            title: Some(title.clone()),
            tool: Some(definition.agent.tool_name().to_string()),
            support_dir: Some(self.state.support_dir.clone()),
            runtime_root: Some(self.runtime.root.clone()),
            ..Default::default()
        };
        let (pane, attach) = TerminalPane::pending_with_pty_config(
            cx,
            pty_config.clone(),
            self.terminal_config_from_settings(),
        );
        self.register_terminal_pane(Some(&terminal_id), &pane, cx);
        let pane_count = self.main_terminal().map(|tab| tab.panes.len()).unwrap_or(0);
        if let Some(tab) = self.main_terminal_mut() {
            tab.panes.push(TerminalPaneSlot {
                title,
                terminal_id: Some(terminal_id.clone()),
                pane: Some(pane),
                restored_output_bytes: 0,
                restored_output_tail: String::new(),
                restore_command: None,
                restore_env: None,
            });
        }
        let split_tree = if pane_count == 0 {
            Some(TerminalSplitNode::Leaf { pane: 0 })
        } else {
            let current = terminal_split_tree_for_panes(
                self.state.terminal_layout.split_tree.clone(),
                &self.state.terminal_layout.top_grid,
                &self.state.terminal_layout.top_ratios,
                pane_count,
            )
            .unwrap_or(TerminalSplitNode::Leaf { pane: 0 });
            terminal_split_tree_insert_pane_root(
                &current,
                pane_count,
                TerminalSplitDirection::Right,
            )
            .ok()
        };
        self.set_terminal_split_tree(split_tree);
        self.sync_terminal_state_after_layout_change(cx);
        self.spawn_attach_pending_terminals(None, vec![(pty_config, attach)], cx);
        self.set_workspace_view(WorkspaceView::Terminal, window, cx);
        self.select_active_terminal_runtime_id(Some(&terminal_id));
        self.focus_active_terminal(window, cx);
    }

    fn open_automation_session(
        &mut self,
        run_id: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(run) = self
            .automation_snapshot
            .runs
            .iter()
            .find(|run| run.id == run_id)
            .cloned()
        else {
            self.show_toast("运行记录不存在".to_string(), cx);
            return;
        };
        let Some(definition) = self
            .automation_snapshot
            .definitions
            .iter()
            .find(|definition| definition.id == run.automation_id)
            .cloned()
        else {
            self.show_toast("自动任务已不存在".to_string(), cx);
            return;
        };
        let Some(session_id) = run
            .ai_session_id
            .as_deref()
            .map(str::trim)
            .filter(|session_id| !session_id.is_empty())
        else {
            self.show_toast("本次运行没有可恢复的 AI 会话".to_string(), cx);
            return;
        };
        let current_workspace_id = super::ai_runtime_status::selected_worktree_info(&self.state)
            .map(|worktree| worktree.id)
            .or_else(|| {
                self.state
                    .selected_project
                    .as_ref()
                    .map(|project| project.id.clone())
            });
        if current_workspace_id.as_deref() != Some(run.workspace_id.as_str()) {
            self.show_toast(
                format!("请先切换到工作区“{}”再打开会话", run.workspace_name),
                cx,
            );
            return;
        }
        if let Some(session) = self
            .state
            .ai_history
            .sessions
            .iter()
            .find(|session| {
                session.external_session_id.as_deref() == Some(session_id)
                    || session.session_key == session_id
            })
            .cloned()
        {
            if self.restore_ai_session_with_gateway_in_main_split(&session, window, cx) {
                return;
            }
            self.restore_ai_session_in_main_split(
                session.title.clone(),
                ai_session_restore_command(&session),
                window,
                cx,
            );
            return;
        }
        let command = automation_session_restore_command(definition.agent, session_id);
        self.restore_ai_session_in_main_split(
            format!("自动任务 · {}", definition.name),
            command,
            window,
            cx,
        );
    }
}

fn automation_workspace_name(project_name: &str, worktree: Option<&WorktreeInfo>) -> String {
    let Some(worktree) = worktree else {
        return project_name.to_string();
    };
    if worktree.is_default {
        return project_name.to_string();
    }
    let name = worktree.name.trim();
    if !name.is_empty() {
        name.to_string()
    } else {
        worktree.branch.trim().to_string()
    }
}

fn push_unique_automation_branch(branches: &mut Vec<String>, value: &str) {
    let branch = value.trim();
    if branch.is_empty() || branches.iter().any(|item| item == branch) {
        return;
    }
    branches.push(branch.to_string());
}

fn short_automation_id(id: &str) -> &str {
    id.get(..8).unwrap_or(id)
}

fn automation_worktree_branch_name(plan: &AutomationRunPlan) -> String {
    format!(
        "wecode/automation/{}/run-{}",
        short_automation_id(&plan.automation_id),
        short_automation_id(&plan.run_id)
    )
}

fn append_automation_output(output: &parking_lot::Mutex<String>, chunk: &str) {
    let mut output = output.lock();
    output.push_str(chunk);
    if output.len() > AUTOMATION_OUTPUT_BUFFER_LIMIT {
        let mut start = output.len() - AUTOMATION_OUTPUT_BUFFER_LIMIT;
        while start < output.len() && !output.is_char_boundary(start) {
            start += 1;
        }
        output.drain(..start);
    }
}

fn automation_agent_command(
    agent: AutomationAgent,
    prompt: &str,
    resume_session_id: Option<&str>,
    fresh_session_id: &str,
) -> String {
    let prompt = shell_quote(prompt);
    let resume_session_id = resume_session_id.map(shell_quote);
    match agent {
        AutomationAgent::Claude => resume_session_id.map_or_else(
            || {
                format!(
                    "claude --permission-mode bypassPermissions --session-id {} --print {prompt}",
                    shell_quote(fresh_session_id)
                )
            },
            |session_id| {
                format!(
                    "claude --permission-mode bypassPermissions --resume {session_id} --print {prompt}"
                )
            },
        ),
        AutomationAgent::Codex => resume_session_id.map_or_else(
            || format!("codex exec --dangerously-bypass-approvals-and-sandbox {prompt}"),
            |session_id| {
                format!(
                    "codex exec resume --dangerously-bypass-approvals-and-sandbox {session_id} {prompt}"
                )
            },
        ),
        AutomationAgent::Kiro => resume_session_id.map_or_else(
            || format!("kiro-cli chat --trust-all-tools --no-interactive {prompt}"),
            |session_id| {
                format!(
                    "kiro-cli chat --resume-id {session_id} --trust-all-tools --no-interactive {prompt}"
                )
            },
        ),
    }
}

fn automation_session_restore_command(agent: AutomationAgent, session_id: &str) -> String {
    let session_id = shell_quote(session_id);
    match agent {
        AutomationAgent::Claude => format!("claude --resume {session_id}"),
        AutomationAgent::Codex => format!("codex resume {session_id}"),
        AutomationAgent::Kiro => format!("kiro-cli chat --resume-id {session_id}"),
    }
}

fn record_automation_ai_session_if_available(
    terminal_manager: &TerminalManager,
    service: &AutomationService,
    run_id: &str,
    terminal_id: &str,
    recorded: &std::sync::atomic::AtomicBool,
) {
    use std::sync::atomic::Ordering;

    if recorded.load(Ordering::Acquire) {
        return;
    }
    let Some(ai_session_id) = terminal_manager
        .ai_runtime_state_snapshot()
        .and_then(|snapshot| {
            snapshot
                .sessions
                .into_iter()
                .find(|session| session.terminal_id == terminal_id)
                .and_then(|session| session.ai_session_id)
        })
    else {
        return;
    };
    if service.record_run_ai_session(run_id, ai_session_id).is_ok() {
        recorded.store(true, Ordering::Release);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fresh_and_reused_agent_commands_keep_session_semantics() {
        let fresh = automation_agent_command(
            AutomationAgent::Claude,
            "Review changes",
            None,
            "550e8400-e29b-41d4-a716-446655440000",
        );
        assert!(fresh.contains("--session-id 550e8400-e29b-41d4-a716-446655440000"));
        assert!(fresh.contains("--print 'Review changes'"));

        let reused = automation_agent_command(
            AutomationAgent::Codex,
            "Continue report",
            Some("session-1"),
            "unused",
        );
        assert!(reused.starts_with("codex exec resume"));
        assert!(reused.contains("session-1 'Continue report'"));

        let kiro = automation_agent_command(
            AutomationAgent::Kiro,
            "Continue report",
            Some("session-2"),
            "unused",
        );
        assert!(kiro.contains("--resume-id session-2"));
        assert!(kiro.contains("--no-interactive 'Continue report'"));
    }

    #[test]
    fn catch_up_grace_labels_match_editor_options() {
        assert_eq!(automation_catch_up_grace_label(0), "不补跑");
        assert_eq!(automation_catch_up_grace_label(43_200), "12 小时内");
        assert_eq!(automation_catch_up_grace_label(7_200), "2 小时内");
    }

    #[test]
    fn list_schedule_labels_keep_meaning_compact() {
        assert_eq!(
            automation_list_schedule_label(&AutomationSchedule::Weekly {
                weekdays: vec![1, 2, 3, 4, 5],
                hour: 10,
                minute: 10,
            }),
            "工作日 10:10"
        );
        assert_eq!(
            automation_list_schedule_label(&AutomationSchedule::Weekly {
                weekdays: vec![1, 3, 5],
                hour: 9,
                minute: 30,
            }),
            "周一/三/五 09:30"
        );
    }
}
