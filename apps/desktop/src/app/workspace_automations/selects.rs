use super::*;
use crate::ui::select::UiSelectEvent;

#[derive(Clone, Copy)]
enum AutomationSelectKind {
    Project,
    Workspace,
    Branch,
    Agent,
    Schedule,
    Grace,
}

impl WeCodeApp {
    pub(crate) fn observe_automation_selects(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let selects = [
            (
                AutomationSelectKind::Project,
                self.automation_project_select.clone(),
            ),
            (
                AutomationSelectKind::Workspace,
                self.automation_workspace_select.clone(),
            ),
            (
                AutomationSelectKind::Branch,
                self.automation_branch_select.clone(),
            ),
            (
                AutomationSelectKind::Agent,
                self.automation_agent_select.clone(),
            ),
            (
                AutomationSelectKind::Schedule,
                self.automation_schedule_select.clone(),
            ),
            (
                AutomationSelectKind::Grace,
                self.automation_grace_select.clone(),
            ),
        ];
        for (kind, select) in selects {
            let Some(select) = select else {
                continue;
            };
            cx.subscribe_in(
                &select,
                window,
                move |app, _, event: &UiSelectEvent, window, cx| {
                    let UiSelectEvent::Confirm(Some(value)) = event else {
                        return;
                    };
                    app.apply_automation_select(kind, value.clone(), window, cx);
                },
            )
            .detach();
        }
    }

    fn apply_automation_select(
        &mut self,
        kind: AutomationSelectKind,
        value: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match kind {
            AutomationSelectKind::Project => self.select_automation_project(value, cx),
            AutomationSelectKind::Workspace => {
                self.automation_workspace_id = value;
                self.automation_reuse_session = false;
                self.invalidate_ui(cx, [UiRegion::WorkspaceBody]);
            }
            AutomationSelectKind::Branch => {
                self.automation_base_branch = value;
                self.invalidate_ui(cx, [UiRegion::WorkspaceBody]);
            }
            AutomationSelectKind::Agent => {
                if let Some(agent) = automation_agent_from_value(&value) {
                    self.automation_agent = agent;
                }
                self.automation_reuse_session = false;
                self.invalidate_ui(cx, [UiRegion::WorkspaceBody]);
            }
            AutomationSelectKind::Schedule => {
                if let Some(preset) = automation_schedule_preset_from_value(&value) {
                    self.set_automation_schedule_preset(preset, window, cx);
                }
            }
            AutomationSelectKind::Grace => {
                if let Ok(seconds) = value.parse::<i64>() {
                    self.automation_catch_up_grace_seconds = seconds;
                }
                self.invalidate_ui(cx, [UiRegion::WorkspaceBody]);
            }
        }
    }
}
