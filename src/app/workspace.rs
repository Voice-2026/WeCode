use super::*;
use crate::app::ui_helpers::{codux_tooltip_container, with_codux_tooltip};
use codux_runtime::{i18n::translate, settings::locale_from_language_setting};
use gpui::{Anchor, relative};
use gpui_component::{
    input::{Input, InputEvent, InputState},
    menu::{DropdownMenu, PopupMenuItem},
    popover::Popover,
    resizable::{resizable_panel, v_resizable},
};

impl CoduxApp {
    pub(in crate::app) fn workspace_toolbar(
        &self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let active_index = match self.workspace_view {
            WorkspaceView::Terminal => 0,
            WorkspaceView::Files => 1,
            WorkspaceView::Review => 2,
        };
        let pet_snapshot = self.pet_snapshot.clone();
        let today_level_tokens = workspace_today_level_tokens(&self.state);
        let has_project_context = self.state.selected_project.is_some();
        let pet_sprite_frame = self.visible_pet_sprite_frame(PET_IDLE_FRAME_COUNT);
        column_header(
            div()
                .flex()
                .items_center()
                .justify_between()
                .w_full()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(workspace_segmented_tabs(
                            active_index,
                            &self.state.settings.language,
                            cx,
                        )),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .when(self.state.settings.pet_enabled, |this| {
                            this.child(workspace_pet_button(
                                &self.state.pet,
                                Some(&pet_snapshot),
                                &self.pet_custom_pets,
                                &self.runtime.source_root,
                                &self.state.support_dir,
                                &self.state.settings.language,
                                &self.pet_install_url,
                                &self.pet_install_display_name,
                                self.pet_install_preview.as_ref(),
                                self.pet_install_error.as_deref(),
                                self.pet_install_previewing,
                                self.pet_installing,
                                self.pet_name_editing,
                                pet_sprite_frame,
                                window,
                                cx,
                            ))
                        })
                        .when(!self.state.projects.is_empty(), |this| {
                            this.child(workspace_level_button(
                                today_level_tokens,
                                &self.state.settings.language,
                                cx,
                            ))
                        })
                        .when(has_project_context, |this| {
                            this.child(workspace_open_button(
                                &self.project_open_applications,
                                true,
                                &self.state.settings.language,
                                cx,
                            ))
                            .child(workspace_assistant_button(
                                "AI",
                                AssistantPanel::AIStats,
                                self.assistant_panel,
                                cx,
                            ))
                            .child(workspace_assistant_button(
                                "SSH",
                                AssistantPanel::SSH,
                                self.assistant_panel,
                                cx,
                            ))
                            .child(workspace_assistant_button(
                                "Files",
                                AssistantPanel::FileManager,
                                self.assistant_panel,
                                cx,
                            ))
                            .child(workspace_assistant_button(
                                "Git",
                                AssistantPanel::Git,
                                self.assistant_panel,
                                cx,
                            ))
                        }),
                ),
            cx,
        )
    }

    pub(in crate::app) fn workspace_body(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .flex_1()
            .flex_basis(px(0.0))
            .w_full()
            .h_full()
            .min_w_0()
            .min_h_0()
            .child(match self.workspace_view {
                WorkspaceView::Terminal => self.terminal_workspace_body(cx).into_any_element(),
                WorkspaceView::Files => self.files_workspace_body(window, cx).into_any_element(),
                WorkspaceView::Review => self.review_workspace_body(cx).into_any_element(),
            })
    }

    fn terminal_workspace_body(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let has_bottom_tabs = self.bottom_terminals().next().is_some();
        if !has_bottom_tabs {
            return div()
                .flex()
                .flex_col()
                .flex_1()
                .flex_basis(px(0.0))
                .min_w_0()
                .min_h_0()
                .w_full()
                .h_full()
                .bg(color(theme::BG_TERMINAL))
                .child(
                    div()
                        .flex_1()
                        .flex_basis(px(0.0))
                        .min_w_0()
                        .min_h_0()
                        .w_full()
                        .child(self.terminal_main_split_area(cx)),
                )
                .child(div().h(px(40.0)).child(self.terminal_bottom_tabs_area(cx)));
        }

        div()
            .flex()
            .flex_col()
            .flex_1()
            .flex_basis(px(0.0))
            .min_w_0()
            .min_h_0()
            .w_full()
            .h_full()
            .bg(color(theme::BG_TERMINAL))
            .child(
                v_resizable("workspace-terminal-split")
                    .child(
                        resizable_panel()
                            .size(px(420.0))
                            .size_range(px(220.0)..px(900.0))
                            .child(self.terminal_main_split_area(cx)),
                    )
                    .child(
                        resizable_panel()
                            .size(px(220.0))
                            .size_range(px(44.0)..px(520.0))
                            .child(self.terminal_bottom_tabs_area(cx)),
                    ),
            )
    }

    fn terminal_main_split_area(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .flex_1()
            .size_full()
            .min_w_0()
            .min_h_0()
            .child(self.terminal_panes(cx))
    }

    fn terminal_bottom_tabs_area(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let active = self.active_bottom_terminal();
        let has_bottom_tabs = active.is_some();

        div()
            .flex()
            .flex_col()
            .size_full()
            .min_w_0()
            .min_h_0()
            .child(
                div()
                    .h(px(40.0))
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_2()
                    .px_2()
                    .border_t_1()
                    .border_color(cx.theme().border)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .flex_1()
                            .min_w_0()
                            .overflow_hidden()
                            .when(!has_bottom_tabs, |this| {
                                this.child(
                                    div()
                                        .px_2()
                                        .text_xs()
                                        .line_height(px(16.0))
                                        .text_color(cx.theme().secondary_foreground)
                                        .child("终端"),
                                )
                            })
                            .children(self.bottom_terminals().map(|terminal| {
                                terminal_bottom_tab_button(
                                    terminal.id,
                                    terminal.label.clone(),
                                    terminal.id == self.active_terminal_id,
                                    cx,
                                )
                                .into_any_element()
                            })),
                    )
                    .child(terminal_bottom_add_button(cx)),
            )
            .when_some(active, |this, tab| {
                this.child(div().flex_1().min_h_0().child(terminal_bottom_content(tab)))
            })
    }

    fn files_workspace_body(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let snapshot = self.file_editor_workspace_snapshot();
        let app_entity = cx.entity();
        div()
            .flex()
            .flex_1()
            .bg(color(theme::BG_TERMINAL))
            .child(cx.new(|_| file_editor::FileEditorWorkspaceView::new(app_entity, snapshot)))
    }

    fn review_workspace_body(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .flex_1()
            .bg(color(theme::BG_TERMINAL))
            .child(
                div()
                    .h(px(44.0))
                    .px_3()
                    .flex()
                    .items_center()
                    .justify_between()
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .bg(cx.theme().title_bar)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_3()
                            .child(metric_inline("Branch", self.state.git.branch.clone()))
                            .child(metric_inline(
                                "Files",
                                self.state.git.changed_files.len().to_string(),
                            ))
                            .child(metric_inline("Staged", self.state.git.staged.to_string())),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .child(workspace_text_button(
                                "review-commit",
                                "Commit",
                                cx,
                                |app, _event, window, cx| app.commit_staged_git(window, cx),
                            ))
                            .child(workspace_text_button(
                                "review-push",
                                "Push",
                                cx,
                                |app, _event, window, cx| app.push_project_git(window, cx),
                            )),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_1()
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .w(px(360.0))
                            .border_r_1()
                            .border_color(color(theme::BORDER_SOFT))
                            .child(git_workspace_section(
                                &self.state.git,
                                self.selected_git_file.as_deref(),
                                self.selected_git_branch.as_deref(),
                                cx,
                            )),
                    )
                    .child(div().flex_1().child(git_review_workspace(
                        self.selected_git_file.as_deref(),
                        &self.git_diff_preview,
                        self.git_review_content.as_ref(),
                    ))),
            )
    }

    pub(in crate::app) fn terminal_panes(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let Some(active) = self.main_terminal() else {
            return div().flex_1().size_full().bg(color(theme::BG_TERMINAL));
        };
        let pane_count = active.panes.len();

        div().flex().flex_1().min_w_0().overflow_hidden().children(
            active.panes.iter().enumerate().map(|(index, slot)| {
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
                                .size_full()
                                .flex()
                                .items_center()
                                .justify_center()
                                .text_color(color(theme::TEXT_DIM))
                                .child("Terminal mounting...")
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
            }),
        )
    }
}

fn terminal_bottom_content(tab: &TerminalTab) -> impl IntoElement {
    div().size_full().min_h_0().child(
        match tab.panes.first().and_then(|slot| slot.pane.as_ref()) {
            Some(pane) => gpui::AnyView::from(pane.view.clone())
                .cached(gpui::StyleRefinement::default().flex().size_full())
                .into_any_element(),
            None => div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .text_color(color(theme::TEXT_DIM))
                .child("Terminal mounting...")
                .into_any_element(),
        },
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

fn metric_inline(label: &'static str, value: String) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .gap_1()
        .text_xs()
        .child(div().text_color(color(theme::TEXT_DIM)).child(label))
        .child(
            div()
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(color(theme::TEXT))
                .child(value),
        )
}

fn workspace_open_button(
    applications: &[ProjectOpenApplicationSummary],
    has_project: bool,
    language: &str,
    cx: &mut Context<CoduxApp>,
) -> impl IntoElement {
    let applications = applications
        .iter()
        .filter(|application| application.installed)
        .cloned()
        .collect::<Vec<_>>();
    let app_entity = cx.entity();
    let reveal_entity = app_entity.clone();
    let language = language.to_string();

    div()
        .flex()
        .items_center()
        .rounded(px(6.0))
        .overflow_hidden()
        .bg(cx.theme().secondary)
        .child(
            div()
                .id("workspace-open-folder")
                .h(px(28.0))
                .w(px(38.0))
                .flex()
                .items_center()
                .justify_center()
                .cursor_pointer()
                .when(!has_project, |this| this.opacity(0.45))
                .hover(|style| style.bg(cx.theme().secondary_hover))
                .on_click(move |_, window, cx| {
                    if has_project {
                        cx.update_entity(&reveal_entity, |app, cx| {
                            app.reveal_selected_project_in_file_manager(window, cx);
                        });
                    }
                })
                .child(
                    Icon::new(HeroIconName::FolderOpen)
                        .size_3p5()
                        .text_color(cx.theme().foreground),
                ),
        )
        .child(div().w(px(1.0)).h(px(18.0)).bg(cx.theme().border))
        .child(
            Button::new("workspace-open-apps")
                .text()
                .h(px(28.0))
                .w(px(30.0))
                .cursor_pointer()
                .text_color(cx.theme().foreground)
                .child(
                    Icon::new(HeroIconName::ChevronDown)
                        .size_2()
                        .text_color(cx.theme().foreground),
                )
                .dropdown_menu(move |menu, _window, _cx| {
                    if applications.is_empty() {
                        let label = workspace_i18n(
                            &language,
                            "workspace.open.installed_apps_empty",
                            "No installed apps",
                        );
                        menu.item(
                            PopupMenuItem::new(label).icon(HeroIconName::ArrowTopRightOnSquare),
                        )
                    } else {
                        applications.iter().fold(menu, |menu, application| {
                            let app_entity = app_entity.clone();
                            let application_id = application.id.clone();
                            menu.item(
                                PopupMenuItem::new(application.label.clone())
                                    .icon(if application.category == "primary" {
                                        HeroIconName::ArrowTopRightOnSquare
                                    } else {
                                        HeroIconName::Document
                                    })
                                    .disabled(!has_project)
                                    .on_click(move |_, window, cx| {
                                        cx.update_entity(&app_entity, |app, cx| {
                                            app.open_selected_project_in_application(
                                                application_id.clone(),
                                                window,
                                                cx,
                                            );
                                        });
                                    }),
                            )
                        })
                    }
                }),
        )
}

fn workspace_level_button(
    tokens: i64,
    language: &str,
    cx: &mut Context<CoduxApp>,
) -> impl IntoElement {
    let tokens = tokens.max(0);
    let tier = daily_level_tier(tokens);
    let language = language.to_string();
    let button_label = daily_level_title(&tier, &language);

    Popover::new("workspace-level-popover")
        .anchor(Anchor::TopRight)
        .w(px(304.0))
        .trigger(
            workspace_header_button("workspace-level", cx)
                .secondary()
                .text_color(cx.theme().foreground)
                .child(workspace_daily_level_button_content(
                    tier.clone(),
                    button_label,
                    cx,
                )),
        )
        .content(move |_, _, cx| {
            let theme = cx.theme();
            workspace_level_popover_content(
                tokens,
                tier.clone(),
                language.clone(),
                theme.secondary_hover,
                theme.transparent,
            )
        })
}

pub(in crate::app) fn workspace_today_level_tokens(state: &RuntimeState) -> i64 {
    let history_tokens = state.ai_global_history.today_total_tokens.max(0);
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs_f64())
        .unwrap_or_default();
    let day_start = codux_runtime::ai_history_normalized::local_day_start_seconds(now);
    let live_tokens = state
        .ai_runtime_state
        .sessions
        .iter()
        .map(|session| workspace_live_session_tokens_for_day(session, day_start))
        .sum::<i64>();

    history_tokens + live_tokens
}

pub(in crate::app) fn workspace_live_session_tokens_for_day(
    session: &codux_runtime::ai_runtime_state::AIRuntimeSessionSummary,
    day_start: f64,
) -> i64 {
    if session.updated_at < day_start {
        return 0;
    }

    let started_at = session.started_at.unwrap_or(session.updated_at);
    let started_day = codux_runtime::ai_history_normalized::local_day_start_seconds(started_at);
    let baseline = if (started_day - day_start).abs() < 1.0 {
        session.baseline_total_tokens.max(0)
    } else {
        session.raw_total_tokens.max(session.total_tokens).max(0)
    };
    let total = if session.raw_total_tokens > 0 {
        session.raw_total_tokens
    } else {
        session.total_tokens + session.baseline_total_tokens
    };

    (total - baseline).max(0)
}

fn workspace_pet_button(
    pet: &PetSummary,
    pet_snapshot: Option<&PetSnapshot>,
    custom_pets: &[PetCustomPet],
    runtime_asset_root: &std::path::Path,
    support_dir: &std::path::Path,
    language: &str,
    _install_url: &str,
    _install_display_name: &str,
    _install_preview: Option<&PetCustomPetInstallPreview>,
    _install_error: Option<&str>,
    _install_previewing: bool,
    _installing: bool,
    pet_name_editing: bool,
    pet_sprite_frame: usize,
    window: &mut Window,
    cx: &mut Context<CoduxApp>,
) -> impl IntoElement {
    let app_entity = cx.entity();
    let pet = pet.clone();
    let language = language.to_string();
    let pet_snapshot = pet_snapshot.cloned();
    let custom_pets = custom_pets.to_vec();
    let pet_sprite_path = pet_sprite_path(runtime_asset_root, support_dir, &pet, &custom_pets);
    let label = if pet.claimed {
        format!("Lv.{}", pet.level.max(1))
    } else {
        workspace_i18n(&language, "pet.claim.action", "Claim Pet")
    };
    let trigger = workspace_header_button("workspace-pet", cx)
        .secondary()
        .text_color(cx.theme().foreground)
        .child(workspace_header_badge_button_content(
            HeroIconName::Heart,
            color(0x7C4DFF),
            label,
            cx,
        ));

    if !pet.claimed {
        return trigger
            .on_click(cx.listener(|app, _event, window, cx| {
                app.open_pet_claim_window(window, cx);
            }))
            .into_any_element();
    }

    let content = workspace_pet_popover_content(
        pet.clone(),
        pet_snapshot,
        pet_sprite_path,
        pet_name_editing,
        pet_sprite_frame,
        language.clone(),
        app_entity.clone(),
        window,
        cx,
    );

    Popover::new("workspace-pet-popover")
        .anchor(Anchor::TopRight)
        .appearance(false)
        .w(px(324.0))
        .trigger(trigger)
        .child(content)
        .into_any_element()
}

fn workspace_assistant_button(
    label: &'static str,
    panel: AssistantPanel,
    active_panel: Option<AssistantPanel>,
    cx: &mut Context<CoduxApp>,
) -> impl IntoElement {
    let active = active_panel == Some(panel);

    let button = workspace_header_button(
        match panel {
            AssistantPanel::AIStats => "workspace-assistant-ai",
            AssistantPanel::SSH => "workspace-assistant-ssh",
            AssistantPanel::FileManager => "workspace-assistant-files",
            AssistantPanel::Git => "workspace-assistant-git",
        },
        cx,
    );
    let button = if active {
        button.secondary().text_color(cx.theme().foreground)
    } else {
        button.ghost().text_color(cx.theme().secondary_foreground)
    };

    with_codux_tooltip(
        cx.entity(),
        match panel {
            AssistantPanel::AIStats => "workspace-assistant-ai-tooltip",
            AssistantPanel::SSH => "workspace-assistant-ssh-tooltip",
            AssistantPanel::FileManager => "workspace-assistant-files-tooltip",
            AssistantPanel::Git => "workspace-assistant-git-tooltip",
        },
        button
            .on_click(cx.listener(move |app, _event, window, cx| {
                app.toggle_assistant_panel(panel, window, cx)
            }))
            .child(
                div()
                    .h(px(20.0))
                    .w(px(20.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(
                        Icon::new(match panel {
                            AssistantPanel::AIStats => HeroIconName::Sparkles,
                            AssistantPanel::SSH => HeroIconName::CommandLine,
                            AssistantPanel::FileManager => HeroIconName::Document,
                            AssistantPanel::Git => HeroIconName::ArrowPathRoundedSquare,
                        })
                        .size_3p5()
                        .text_color(if active {
                            cx.theme().foreground
                        } else {
                            cx.theme().secondary_foreground
                        }),
                    ),
            ),
        label,
    )
}

fn workspace_pet_dex_button(
    dex_tooltip: SharedString,
    app_entity: gpui::Entity<CoduxApp>,
) -> impl IntoElement {
    codux_tooltip_container(app_entity.clone(), "workspace-pet-dex-tooltip", dex_tooltip)
        .absolute()
        .right(px(10.0))
        .top(px(10.0))
        .child(
            Button::new("workspace-pet-dex-open")
                .compact()
                .ghost()
                .icon(Icon::new(HeroIconName::BookOpen).size_3p5())
                .on_click(move |_, window, cx| {
                    cx.update_entity(&app_entity, |app, cx| {
                        app.open_pet_dex_window(window, cx);
                    });
                }),
        )
}

fn workspace_pet_rename_action_button(
    id: &'static str,
    icon: HeroIconName,
    tooltip: SharedString,
    cx: &mut Context<CoduxApp>,
    on_click: impl Fn(&mut CoduxApp, &gpui::ClickEvent, &mut Window, &mut Context<CoduxApp>) + 'static,
) -> impl IntoElement {
    with_codux_tooltip(
        cx.entity(),
        format!("pet-rename-tooltip-{id}"),
        Button::new(id)
            .compact()
            .ghost()
            .text_color(cx.theme().secondary_foreground)
            .icon(
                Icon::new(icon)
                    .size_3p5()
                    .text_color(cx.theme().secondary_foreground),
            )
            .on_click(cx.listener(on_click)),
        tooltip,
    )
}

fn workspace_pet_install_action_button(
    button: Button,
    tooltip: SharedString,
    label: SharedString,
    icon: HeroIconName,
    cx: &mut Context<CoduxApp>,
    on_click: impl Fn(&mut CoduxApp, &gpui::ClickEvent, &mut Window, &mut Context<CoduxApp>) + 'static,
) -> impl IntoElement {
    with_codux_tooltip(
        cx.entity(),
        format!("pet-install-tooltip-{tooltip}"),
        button
            .text_color(cx.theme().secondary_foreground)
            .icon(
                Icon::new(icon)
                    .size_3p5()
                    .text_color(cx.theme().secondary_foreground),
            )
            .child(workspace_pet_install_button_label(label))
            .on_click(cx.listener(on_click)),
        tooltip,
    )
}

fn workspace_header_button(id: &'static str, cx: &mut Context<CoduxApp>) -> Button {
    Button::new(id)
        .compact()
        .h(px(28.0))
        .text_color(cx.theme().foreground)
}

fn workspace_header_badge_button_content(
    icon: HeroIconName,
    icon_bg: gpui::Hsla,
    label: impl Into<SharedString>,
    cx: &mut Context<CoduxApp>,
) -> impl IntoElement {
    div()
        .h(px(20.0))
        .flex()
        .items_center()
        .gap_2()
        .text_color(cx.theme().foreground)
        .child(
            div()
                .size(px(18.0))
                .flex()
                .items_center()
                .justify_center()
                .rounded_full()
                .bg(icon_bg)
                .text_color(color(0xFFFFFF))
                .child(Icon::new(icon).size_2()),
        )
        .child(
            div()
                .text_xs()
                .text_color(cx.theme().foreground)
                .child(label.into()),
        )
}

#[derive(Clone)]
struct DailyLevelTier {
    id: &'static str,
    title: &'static str,
    min: i64,
    color: u32,
    icon: DailyLevelIcon,
}

#[derive(Clone)]
enum DailyLevelIcon {
    Component(HeroIconName),
    Asset(&'static str),
}

const DAILY_LEVEL_TIERS: [DailyLevelTier; 8] = [
    DailyLevelTier {
        id: "iron",
        title: "Iron",
        min: 0,
        color: 0x5B616D,
        icon: DailyLevelIcon::Component(HeroIconName::Minus),
    },
    DailyLevelTier {
        id: "bronze",
        title: "Bronze",
        min: 1_000_000,
        color: 0xC98663,
        icon: DailyLevelIcon::Asset("rank-icons/zap.svg"),
    },
    DailyLevelTier {
        id: "silver",
        title: "Silver",
        min: 3_000_000,
        color: 0xC8D1E3,
        icon: DailyLevelIcon::Asset("rank-icons/shield-check.svg"),
    },
    DailyLevelTier {
        id: "gold",
        title: "Gold",
        min: 6_000_000,
        color: 0xE8AA34,
        icon: DailyLevelIcon::Component(HeroIconName::Star),
    },
    DailyLevelTier {
        id: "platinum",
        title: "Platinum",
        min: 10_000_000,
        color: 0x7ED6D8,
        icon: DailyLevelIcon::Component(HeroIconName::Star),
    },
    DailyLevelTier {
        id: "diamond",
        title: "Diamond",
        min: 18_000_000,
        color: 0x59A7FF,
        icon: DailyLevelIcon::Asset("rank-icons/sparkles.svg"),
    },
    DailyLevelTier {
        id: "master",
        title: "Master",
        min: 30_000_000,
        color: 0x9A72FF,
        icon: DailyLevelIcon::Asset("rank-icons/trophy.svg"),
    },
    DailyLevelTier {
        id: "grandmaster",
        title: "Grandmaster",
        min: 50_000_000,
        color: 0xFF5E8E,
        icon: DailyLevelIcon::Asset("rank-icons/flame.svg"),
    },
];

fn daily_level_tier(tokens: i64) -> DailyLevelTier {
    DAILY_LEVEL_TIERS
        .iter()
        .rev()
        .find(|tier| tokens >= tier.min)
        .cloned()
        .unwrap_or_else(|| DAILY_LEVEL_TIERS[0].clone())
}

fn daily_level_title(tier: &DailyLevelTier, language: &str) -> String {
    workspace_i18n(language, &format!("rank.{}", tier.id), tier.title)
}

fn workspace_i18n(language: &str, key: &str, fallback: &str) -> String {
    let locale = locale_from_language_setting(language);
    translate(&locale, key, fallback)
}

fn workspace_daily_level_button_content(
    tier: DailyLevelTier,
    label: String,
    cx: &mut Context<CoduxApp>,
) -> impl IntoElement {
    div()
        .h(px(20.0))
        .flex()
        .items_center()
        .gap_1()
        .text_color(cx.theme().foreground)
        .child(daily_level_badge(&tier, 18.0, 8.0))
        .child(div().text_size(px(12.0)).line_height(px(12.0)).child(label))
}

fn workspace_level_popover_content(
    tokens: i64,
    current_tier: DailyLevelTier,
    language: String,
    hover_surface: gpui::Hsla,
    transparent: gpui::Hsla,
) -> impl IntoElement {
    let tokens = tokens.max(0);
    let current_title = daily_level_title(&current_tier, &language);
    let today_level_label = workspace_i18n(&language, "ai.today_level", "Today's Level");
    let today_tokens_label = workspace_i18n(&language, "ai.today_tokens", "Today's Tokens");
    let current_label = workspace_i18n(&language, "common.current", "Current");
    let need_template = workspace_i18n(&language, "common.need_format", "Need %@");

    div()
        .flex()
        .flex_col()
        .text_color(color(theme::TEXT))
        .child(
            div()
                .flex()
                .items_center()
                .gap_3()
                .child(daily_level_badge(&current_tier, 34.0, 14.0))
                .child(
                    div()
                        .min_w_0()
                        .flex_1()
                        .child(
                            div()
                                .text_size(px(12.0))
                                .line_height(px(16.0))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(color(theme::TEXT_MUTED))
                                .child(today_level_label),
                        )
                        .child(
                            div()
                                .mt(px(2.0))
                                .text_size(px(15.0))
                                .line_height(px(18.0))
                                .font_weight(FontWeight::BOLD)
                                .child(current_title),
                        ),
                )
                .child(
                    div()
                        .text_right()
                        .child(
                            div()
                                .text_size(px(11.0))
                                .line_height(px(14.0))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(color(theme::TEXT_MUTED))
                                .child(today_tokens_label),
                        )
                        .child(
                            div()
                                .mt(px(2.0))
                                .text_size(px(15.0))
                                .line_height(px(18.0))
                                .font_weight(FontWeight::BOLD)
                                .child(compact_number(tokens)),
                        ),
                ),
        )
        .child(div().mt(px(12.0)).flex().flex_col().gap_1().children(
            DAILY_LEVEL_TIERS.into_iter().map(|tier| {
                let current = tier.id == current_tier.id;
                let title = daily_level_title(&tier, &language);
                let need = need_template.replace("%@", &compact_number(tier.min));
                div()
                    .rounded(px(8.0))
                    .px(px(10.0))
                    .py(px(8.0))
                    .flex()
                    .items_center()
                    .gap_2()
                    .bg(if current { hover_surface } else { transparent })
                    .border_1()
                    .border_color(if current {
                        color(tier.color).opacity(0.28)
                    } else {
                        transparent
                    })
                    .child(daily_level_badge(&tier, 24.0, 10.0))
                    .child(
                        div()
                            .min_w_0()
                            .flex_1()
                            .child(
                                div()
                                    .text_size(px(13.0))
                                    .line_height(px(16.0))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child(title),
                            )
                            .child(
                                div()
                                    .mt(px(2.0))
                                    .text_size(px(11.0))
                                    .line_height(px(14.0))
                                    .text_color(color(theme::TEXT_MUTED))
                                    .child(need),
                            ),
                    )
                    .when(current, |this| {
                        this.child(
                            div()
                                .rounded_full()
                                .px(px(8.0))
                                .py(px(4.0))
                                .text_size(px(11.0))
                                .line_height(px(12.0))
                                .font_weight(FontWeight::BOLD)
                                .bg(color(tier.color).opacity(0.14))
                                .text_color(color(tier.color))
                                .child(current_label.clone()),
                        )
                    })
                    .into_any_element()
            }),
        ))
}

fn daily_level_badge(tier: &DailyLevelTier, box_size: f32, icon_size: f32) -> impl IntoElement {
    div()
        .size(px(box_size))
        .rounded_full()
        .flex()
        .items_center()
        .justify_center()
        .bg(linear_gradient(
            135.0,
            linear_color_stop(color(tier.color), 0.0),
            linear_color_stop(color(tier.color).opacity(0.72), 1.0),
        ))
        .text_color(color(0xFFFFFF))
        .child(daily_level_icon(tier.icon.clone(), icon_size))
}

fn daily_level_icon(icon: DailyLevelIcon, icon_size: f32) -> impl IntoElement {
    let icon = match icon {
        DailyLevelIcon::Component(name) => Icon::new(name),
        DailyLevelIcon::Asset(path) => Icon::empty().path(path),
    };
    icon.with_size(px(icon_size)).text_color(color(0xFFFFFF))
}

fn workspace_pet_popover_content(
    pet: PetSummary,
    pet_snapshot: Option<PetSnapshot>,
    pet_sprite_path: std::path::PathBuf,
    pet_name_editing: bool,
    _pet_sprite_frame: usize,
    language: String,
    app_entity: gpui::Entity<CoduxApp>,
    window: &mut Window,
    cx: &mut Context<CoduxApp>,
) -> impl IntoElement {
    let name = if pet.claimed && !pet.display_name.is_empty() {
        pet.display_name.clone()
    } else {
        workspace_i18n(&language, "pet.unclaimed", "No pet claimed")
    };
    let species_name = pet_snapshot
        .as_ref()
        .and_then(|snapshot| {
            snapshot
                .custom_pet
                .as_ref()
                .map(|pet| pet.display_name.clone())
        })
        .unwrap_or_else(|| workspace_pet_species_name(&pet.species, &language));
    let subtitle = if pet.custom_name.trim().is_empty() {
        None
    } else {
        Some(species_name.clone())
    };
    let sprite_fallback_color = cx.theme().primary;
    let progress = pet_snapshot
        .as_ref()
        .map(|snapshot| snapshot.progress.clone())
        .unwrap_or_else(|| codux_runtime::pet::PetProgressInfo {
            level: pet.level.max(1),
            xp_in_level: 0,
            xp_for_level: 0,
            total_xp: pet.total_xp.max(0),
            progress: pet.progress,
            is_at_max_level: false,
        });
    let stats = pet_snapshot
        .as_ref()
        .map(|snapshot| snapshot.current_stats.clone())
        .unwrap_or_default();
    let persona = pet_snapshot
        .as_ref()
        .map(|snapshot| snapshot.persona_id.clone())
        .unwrap_or_else(|| "observer".to_string());
    let persona_label = pet_persona_label(&persona, &language);
    let dex_tooltip = workspace_i18n(&language, "pet.dex.open", "Open Dex");
    let xp_label = workspace_i18n(&language, "pet.xp.label", "Experience");
    let stats_title = workspace_i18n(&language, "pet.stats.title", "Traits");
    let total_xp_label = workspace_i18n(&language, "pet.total_xp", "Total XP");
    let wisdom_label = workspace_i18n(&language, "pet.attribute.wisdom", "Wisdom");
    let chaos_label = workspace_i18n(&language, "pet.attribute.chaos", "Chaos");
    let night_label = workspace_i18n(&language, "pet.attribute.night", "Night");
    let stamina_label = workspace_i18n(&language, "pet.attribute.stamina", "Stamina");
    let empathy_label = workspace_i18n(&language, "pet.attribute.empathy", "Empathy");
    let trait_label_width = pet_trait_label_width([
        &wisdom_label,
        &chaos_label,
        &night_label,
        &stamina_label,
        &empathy_label,
    ]);

    div()
        .flex()
        .flex_col()
        .rounded(px(12.0))
        .border_1()
        .border_color(cx.theme().border)
        .bg(cx.theme().popover)
        .shadow_lg()
        .text_color(color(theme::TEXT))
        .child(
            div()
                .relative()
                .flex()
                .flex_col()
                .items_center()
                .p(px(10.0))
                .child(
                    div()
                        .size(px(104.0))
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(pet_sprite_element(
                            pet_sprite_path,
                            104.0,
                            0,
                            0,
                            sprite_fallback_color,
                        )),
                )
                .child(workspace_pet_dex_button(
                    dex_tooltip.into(),
                    app_entity.clone(),
                ))
                .child(workspace_pet_name_row(
                    pet.clone(),
                    name,
                    subtitle,
                    pet_name_editing,
                    &language,
                    window,
                    cx,
                ))
                .child(
                    div()
                        .mt(px(8.0))
                        .rounded_full()
                        .bg(color(theme::ACCENT).opacity(0.14))
                        .px(px(10.0))
                        .py(px(4.0))
                        .text_size(px(12.0))
                        .line_height(px(12.0))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(color(theme::ACCENT))
                        .child(persona_label),
                )
                .child(
                    div()
                        .mt(px(10.0))
                        .text_size(px(26.0))
                        .line_height(px(32.0))
                        .font_weight(FontWeight::BLACK)
                        .child(format!("Lv.{}", progress.level.max(1))),
                ),
        )
        .child(workspace_popover_separator())
        .child(div().p(px(10.0)).child(workspace_pet_meter(
            xp_label,
            format!(
                "{} / {}",
                compact_number(progress.xp_in_level),
                compact_number(progress.xp_for_level)
            ),
            progress.progress,
            theme::ACCENT,
        )))
        .child(workspace_popover_separator())
        .child(
            div()
                .p(px(10.0))
                .child(
                    div()
                        .mb(px(6.0))
                        .text_size(px(12.0))
                        .line_height(px(16.0))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(color(theme::TEXT_MUTED))
                        .child(stats_title),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .child(workspace_pet_trait(
                            app_entity.clone(),
                            "brain",
                            wisdom_label,
                            stats.wisdom,
                            0x2F8FFF,
                            trait_label_width,
                            workspace_i18n(
                                &language,
                                "pet.attribute.wisdom.help",
                                "Reflects deeper, denser sessions with more substantial exchanges.",
                            ),
                        ))
                        .child(workspace_pet_trait(
                            app_entity.clone(),
                            "flame",
                            chaos_label,
                            stats.chaos,
                            0xFF6030,
                            trait_label_width,
                            workspace_i18n(
                                &language,
                                "pet.attribute.chaos.help",
                                "Reflects fast, jumpy, high-tempo sessions with frequent bursts.",
                            ),
                        ))
                        .child(workspace_pet_trait(
                            app_entity.clone(),
                            "moon",
                            night_label,
                            stats.night,
                            0x6060CC,
                            trait_label_width,
                            workspace_i18n(
                                &language,
                                "pet.attribute.night.help",
                                "Reflects how much of your recent activity leans into late-night hours.",
                            ),
                        ))
                        .child(workspace_pet_trait(
                            app_entity.clone(),
                            "arm",
                            stamina_label,
                            stats.stamina,
                            0x20A060,
                            trait_label_width,
                            workspace_i18n(
                                &language,
                                "pet.attribute.stamina.help",
                                "Reflects steadier sessions that hold focus across more sustained back-and-forth.",
                            ),
                        ))
                        .child(workspace_pet_trait(
                            app_entity.clone(),
                            "bandage",
                            empathy_label,
                            stats.empathy,
                            0xE060A0,
                            trait_label_width,
                            workspace_i18n(
                                &language,
                                "pet.attribute.empathy.help",
                                "Reflects patient repair work, iterative debugging, and careful refinement.",
                            ),
                        )),
                ),
        )
        .child(workspace_popover_separator())
        .child(
            div()
                .p(px(10.0))
                .text_center()
                .child(
                    div()
                        .text_size(px(12.0))
                        .line_height(px(16.0))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(color(theme::TEXT_MUTED))
                        .child(total_xp_label),
                )
                .child(
                    div()
                        .mt(px(2.0))
                        .text_size(px(13.0))
                        .line_height(px(16.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(compact_number(progress.total_xp)),
                ),
        )
        .when_some(pet.error, |this, error| {
            this.child(
                div()
                    .p(px(10.0))
                    .child(workspace_popover_notice(error)),
            )
        })
}

fn workspace_popover_separator() -> impl IntoElement {
    div().mx(px(10.0)).h(px(1.0)).bg(color(theme::BORDER_SOFT))
}

fn workspace_pet_meter(
    label: String,
    value: String,
    progress: f64,
    accent: u32,
) -> impl IntoElement {
    div()
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .gap_2()
                .text_size(px(12.0))
                .line_height(px(16.0))
                .child(
                    div()
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(color(theme::TEXT_MUTED))
                        .child(label),
                )
                .child(
                    div()
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(color(theme::TEXT_DIM))
                        .child(value),
                ),
        )
        .child(
            div()
                .mt(px(6.0))
                .h(px(7.0))
                .rounded_full()
                .overflow_hidden()
                .bg(color(accent).opacity(0.15))
                .child(
                    div()
                        .h_full()
                        .w(relative(progress.clamp(0.0, 1.0) as f32))
                        .rounded_full()
                        .bg(color(accent)),
                ),
        )
}

fn workspace_pet_trait(
    app_entity: gpui::Entity<CoduxApp>,
    emoji_kind: &'static str,
    label: String,
    value: i64,
    accent: u32,
    label_width: f32,
    help: String,
) -> impl IntoElement {
    let ratio = (value as f32 / 330.0).clamp(0.0, 1.0);
    codux_tooltip_container(
        app_entity,
        SharedString::from(format!("pet-trait-{emoji_kind}")),
        help,
    )
    .flex()
    .items_center()
    .gap(px(8.0))
    .text_size(px(12.0))
    .line_height(px(16.0))
    .child(pet_trait_emoji(emoji_kind))
    .child(
        div()
            .w(px(label_width))
            .flex_none()
            .text_color(color(theme::TEXT_MUTED))
            .font_weight(FontWeight::MEDIUM)
            .truncate()
            .child(label),
    )
    .child(
        div()
            .flex_1()
            .min_w(px(0.0))
            .h(px(5.0))
            .rounded_full()
            .overflow_hidden()
            .bg(color(accent).opacity(0.12))
            .child(
                div()
                    .h_full()
                    .w(relative(ratio))
                    .rounded_full()
                    .bg(color(accent).opacity(0.75)),
            ),
    )
    .child(
        div()
            .w(px(34.0))
            .flex_none()
            .text_right()
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(color(theme::TEXT_DIM))
            .child(compact_number(value)),
    )
}

fn pet_trait_label_width<'a>(labels: impl IntoIterator<Item = &'a String>) -> f32 {
    let max_units = labels
        .into_iter()
        .map(|label| {
            label
                .chars()
                .map(|ch| if ch.is_ascii() { 0.58 } else { 1.0 })
                .sum::<f32>()
        })
        .fold(0.0, f32::max);
    (max_units * 12.0).ceil().clamp(32.0, 76.0)
}

fn pet_trait_emoji(kind: &'static str) -> impl IntoElement {
    let emoji = match kind {
        "brain" => "🧠",
        "flame" => "🔥",
        "moon" => "🌙",
        "arm" => "💪",
        "bandage" => "🩹",
        _ => "",
    };
    div()
        .w(px(16.0))
        .text_center()
        .text_size(px(12.0))
        .line_height(px(12.0))
        .child(emoji)
}

fn pet_persona_label(persona: &str, language: &str) -> String {
    let fallback = match persona {
        "observer" => "Observer",
        "sprinter" => "Sprinter",
        "guardian" => "Guardian",
        "nightowl" => "Night Owl",
        "maker" => "Maker",
        value => value,
    };
    workspace_i18n(language, &format!("pet.persona.{persona}"), fallback)
}

fn workspace_pet_species_name(species: &str, language: &str) -> String {
    match species.strip_prefix("custom:") {
        Some(id) if !id.trim().is_empty() => id.to_string(),
        _ => {
            let fallback = match species {
                "voidcat" => "Voidcat",
                "fox" => "Fox",
                "panda" => "Panda",
                "otter" => "Otter",
                "owl" => "Owl",
                "dragon" => "Dragon",
                value if !value.trim().is_empty() => value,
                _ => "Pet",
            };
            workspace_i18n(language, &format!("pet.species.{species}.base"), fallback)
        }
    }
}

fn workspace_pet_name_row(
    pet: PetSummary,
    name: String,
    subtitle: Option<String>,
    editing: bool,
    language: &str,
    window: &mut Window,
    cx: &mut Context<CoduxApp>,
) -> AnyElement {
    if !editing {
        return div()
            .mt(px(12.0))
            .flex()
            .items_baseline()
            .justify_center()
            .gap_1()
            .min_w_0()
            .child(
                div()
                    .id("pet-name-edit-trigger")
                    .cursor_pointer()
                    .text_size(px(17.0))
                    .line_height(px(22.0))
                    .font_weight(FontWeight::BOLD)
                    .truncate()
                    .on_click(cx.listener(|app, _event, window, cx| {
                        app.start_current_pet_rename(window, cx)
                    }))
                    .child(name),
            )
            .when_some(subtitle, |this, subtitle| {
                this.child(
                    div()
                        .max_w(px(92.0))
                        .truncate()
                        .text_size(px(14.0))
                        .line_height(px(20.0))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(color(theme::TEXT_MUTED))
                        .child(subtitle),
                )
            })
            .into_any_element();
    }

    let value = pet.custom_name.clone();
    let placeholder = workspace_i18n(language, "pet.name.placeholder", "Pet Name");
    let name_state = window.use_keyed_state("pet-rename-custom-name", cx, |window, cx| {
        InputState::new(window, cx)
            .default_value(value.clone())
            .placeholder(placeholder)
    });
    name_state.update(cx, |state, cx| {
        if state.value().as_ref() != pet.custom_name {
            state.set_value(pet.custom_name.clone(), window, cx);
        }
    });
    cx.subscribe_in(&name_state, window, |app, state, event, window, cx| {
        if matches!(event, InputEvent::PressEnter { .. }) {
            app.rename_current_pet_to(state.read(cx).value().to_string(), window, cx);
        }
    })
    .detach();
    let save_state = name_state.clone();

    div()
        .mt(px(12.0))
        .flex()
        .items_center()
        .justify_center()
        .gap_1()
        .child(
            div()
                .w(px(150.0))
                .child(Input::new(&name_state).with_size(gpui_component::Size::Small)),
        )
        .child(workspace_pet_rename_action_button(
            "pet-rename-current",
            HeroIconName::Check,
            workspace_i18n(&language, "pet.name.save", "Save pet name").into(),
            cx,
            move |app, _event, window, cx| {
                let custom_name = save_state.read(cx).value().to_string();
                app.rename_current_pet_to(custom_name, window, cx)
            },
        ))
        .child(workspace_pet_rename_action_button(
            "pet-rename-cancel",
            HeroIconName::XMark,
            workspace_i18n(&language, "common.cancel", "Cancel").into(),
            cx,
            |app, _event, window, cx| app.cancel_current_pet_rename(window, cx),
        ))
        .into_any_element()
}

pub(in crate::app) fn workspace_pet_install_form(
    install_url: &str,
    install_display_name: &str,
    install_preview: Option<&PetCustomPetInstallPreview>,
    install_error: Option<&str>,
    install_previewing: bool,
    installing: bool,
    language: &str,
    window: &mut Window,
    cx: &mut Context<CoduxApp>,
) -> impl IntoElement {
    let url_value = install_url.to_string();
    let name_value = install_display_name.to_string();
    let url_placeholder = workspace_i18n(
        language,
        "pet.custom.install.url.placeholder",
        "https://petdex.crafter.run/zh/pets/boba",
    );
    let name_placeholder = workspace_i18n(language, "pet.custom.install.name.label", "Pet Name");
    let url_state = window.use_keyed_state("pet-install-url", cx, |window, cx| {
        InputState::new(window, cx)
            .default_value(url_value.clone())
            .placeholder(url_placeholder.clone())
    });
    url_state.update(cx, |state, cx| {
        if state.value().as_ref() != install_url {
            state.set_value(install_url.to_string(), window, cx);
        }
    });
    cx.subscribe_in(&url_state, window, |app, state, event, window, cx| {
        if matches!(event, InputEvent::Change) {
            app.set_pet_install_url(state.read(cx).value().to_string(), window, cx);
        }
    })
    .detach();

    let name_state = window.use_keyed_state("pet-install-display-name", cx, |window, cx| {
        InputState::new(window, cx)
            .default_value(name_value.clone())
            .placeholder(name_placeholder.clone())
    });
    name_state.update(cx, |state, cx| {
        if state.value().as_ref() != install_display_name {
            state.set_value(install_display_name.to_string(), window, cx);
        }
    });
    cx.subscribe_in(&name_state, window, |app, state, event, window, cx| {
        if matches!(event, InputEvent::Change) {
            app.set_pet_install_display_name(state.read(cx).value().to_string(), window, cx);
        }
    })
    .detach();

    div()
        .flex()
        .flex_col()
        .gap(px(16.0))
        .text_size(px(14.0))
        .line_height(px(18.0))
        .child(
            div()
                .rounded(px(8.0))
                .bg(cx.theme().group_box)
                .p(px(14.0))
                .child(
                    div()
                        .mb(px(8.0))
                        .text_size(px(12.0))
                        .line_height(px(16.0))
                        .text_color(color(theme::TEXT_MUTED))
                        .child(workspace_i18n(
                            &language,
                            "pet.custom.install.url.label",
                            "Petdex Page URL",
                        )),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(
                            div().flex_1().min_w_0().child(
                                Input::new(&url_state).with_size(gpui_component::Size::Medium),
                            ),
                        )
                        .child(workspace_pet_install_action_button(
                            Button::new("pet-custom-market").ghost(),
                            workspace_i18n(
                                &language,
                                "pet.custom.market.title",
                                "Petdex Marketplace",
                            )
                            .into(),
                            workspace_i18n(&language, "pet.custom.market.action", "Get Pets")
                                .into(),
                            HeroIconName::ArrowTopRightOnSquare,
                            cx,
                            |app, _event, window, cx| app.open_pet_market(window, cx),
                        ))
                        .child(workspace_pet_install_action_button(
                            Button::new("pet-preview-custom")
                                .secondary()
                                .loading(install_previewing)
                                .disabled(
                                    install_url.trim().is_empty()
                                        || install_previewing
                                        || installing,
                                ),
                            workspace_i18n(
                                &language,
                                "pet.custom.install.preview.label",
                                "Pet Preview",
                            )
                            .into(),
                            if install_previewing {
                                workspace_i18n(
                                    &language,
                                    "pet.custom.install.resolving",
                                    "Reading Petdex page...",
                                )
                                .into()
                            } else if install_preview.is_some() {
                                workspace_i18n(
                                    &language,
                                    "pet.custom.install.resolve_again",
                                    "Parse Again",
                                )
                                .into()
                            } else {
                                workspace_i18n(&language, "pet.custom.install.resolve", "Parse")
                                    .into()
                            },
                            HeroIconName::Eye,
                            cx,
                            |app, _event, window, cx| app.preview_custom_pet_install(window, cx),
                        )),
                ),
        )
        .when_some(install_preview.cloned(), |this, preview| {
            this.child(workspace_pet_install_preview(
                preview,
                &name_state,
                installing,
                &language,
                cx,
            ))
        })
        .when(installing, |this| {
            this.child(
                div()
                    .rounded(px(8.0))
                    .bg(color(theme::ACCENT).opacity(0.1))
                    .px(px(12.0))
                    .py(px(8.0))
                    .text_size(px(12.0))
                    .line_height(px(16.0))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(color(theme::ACCENT))
                    .child(workspace_i18n(
                        &language,
                        "pet.custom.install.installing.detail",
                        "Downloading, unpacking, and validating the pet package.",
                    )),
            )
        })
        .when_some(install_error.map(str::to_string), |this, error| {
            this.child(workspace_pet_install_error(error))
        })
}

fn workspace_pet_install_preview(
    preview: PetCustomPetInstallPreview,
    name_state: &gpui::Entity<InputState>,
    installing: bool,
    language: &str,
    cx: &mut Context<CoduxApp>,
) -> impl IntoElement {
    let image = if let Some(path) = preview
        .local_image_path
        .as_ref()
        .filter(|path| !path.trim().is_empty())
    {
        img(PathBuf::from(path))
            .size_full()
            .object_fit(ObjectFit::Cover)
            .with_fallback(|| workspace_pet_install_preview_fallback())
            .into_any_element()
    } else if let Some(url) = preview
        .image_url
        .as_ref()
        .filter(|url| !url.trim().is_empty())
    {
        img(url.clone())
            .size_full()
            .object_fit(ObjectFit::Cover)
            .with_fallback(|| workspace_pet_install_preview_fallback())
            .into_any_element()
    } else {
        workspace_pet_install_preview_fallback()
    };

    div()
        .flex()
        .flex_col()
        .gap(px(12.0))
        .rounded(px(10.0))
        .border_1()
        .border_color(color(theme::BORDER_SOFT))
        .bg(cx.theme().group_box)
        .p(px(14.0))
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(14.0))
                .child(
                    div()
                        .size(px(104.0))
                        .rounded(px(10.0))
                        .overflow_hidden()
                        .flex()
                        .items_center()
                        .justify_center()
                        .bg(color(theme::ACCENT).opacity(0.1))
                        .child(image),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .child(
                            div()
                                .truncate()
                                .text_size(px(14.0))
                                .line_height(px(18.0))
                                .child(preview.display_name.clone()),
                        )
                        .child(
                            div()
                                .mt(px(4.0))
                                .text_size(px(12.0))
                                .line_height(px(20.0))
                                .text_color(color(theme::TEXT_MUTED))
                                .child(empty_label(&preview.description)),
                        )
                        .child(
                            div()
                                .mt(px(8.0))
                                .flex()
                                .items_center()
                                .gap(px(6.0))
                                .text_size(px(12.0))
                                .line_height(px(16.0))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(color(theme::TEXT_DIM))
                                .child(Icon::new(HeroIconName::ArrowTopRightOnSquare).size_3())
                                .child(
                                    div()
                                        .min_w_0()
                                        .truncate()
                                        .child(pet_install_host_label(&preview.page_url)),
                                ),
                        ),
                ),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(8.0))
                .child(
                    div()
                        .text_size(px(12.0))
                        .line_height(px(16.0))
                        .text_color(color(theme::TEXT_MUTED))
                        .child(workspace_i18n(
                            language,
                            "pet.custom.install.name.label",
                            "Pet Name",
                        )),
                )
                .child(
                    Input::new(name_state)
                        .with_size(gpui_component::Size::Medium)
                        .disabled(installing),
                ),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(7.0))
                .child(workspace_pet_install_check(workspace_i18n(
                    language,
                    "pet.custom.install.validation.page",
                    "Petdex page verified",
                )))
                .child(workspace_pet_install_check(workspace_i18n(
                    language,
                    "pet.custom.install.validation.package",
                    "Package link found",
                )))
                .child(workspace_pet_install_check(workspace_i18n(
                    language,
                    "pet.custom.install.validation.format",
                    "Codex-format check runs during install",
                ))),
        )
}

fn workspace_pet_install_button_label(label: impl Into<SharedString>) -> impl IntoElement {
    div()
        .text_size(px(14.0))
        .line_height(px(18.0))
        .child(label.into())
}

fn workspace_pet_install_preview_fallback() -> AnyElement {
    div()
        .size_full()
        .flex()
        .items_center()
        .justify_center()
        .child(
            Icon::new(HeroIconName::InformationCircle)
                .size_8()
                .text_color(color(theme::ACCENT)),
        )
        .into_any_element()
}

fn workspace_pet_install_check(text: String) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .gap(px(8.0))
        .text_size(px(12.0))
        .line_height(px(16.0))
        .text_color(color(theme::TEXT_MUTED))
        .child(
            Icon::new(HeroIconName::CheckCircle)
                .size_3p5()
                .text_color(color(theme::GREEN)),
        )
        .child(text)
}

fn workspace_pet_install_error(error: String) -> impl IntoElement {
    div()
        .rounded(px(8.0))
        .bg(color(theme::ORANGE).opacity(0.12))
        .px(px(12.0))
        .py(px(8.0))
        .text_size(px(12.0))
        .line_height(px(16.0))
        .font_weight(FontWeight::MEDIUM)
        .text_color(color(theme::ORANGE))
        .child(error)
}

fn pet_install_host_label(page_url: &str) -> String {
    let trimmed = page_url.trim();
    trimmed
        .split_once("://")
        .map(|(_, rest)| rest)
        .unwrap_or(trimmed)
        .split('/')
        .next()
        .filter(|host| !host.trim().is_empty())
        .unwrap_or("petdex.crafter.run")
        .to_string()
}

fn workspace_popover_notice(message: String) -> impl IntoElement {
    div()
        .rounded(px(6.0))
        .bg(color(theme::ORANGE).opacity(0.12))
        .px_2()
        .py_1()
        .text_size(px(12.0))
        .line_height(px(16.0))
        .text_color(color(theme::ORANGE))
        .child(message)
}

fn workspace_segmented_tabs(
    active_index: usize,
    language: &str,
    cx: &mut Context<CoduxApp>,
) -> impl IntoElement {
    let locale = locale_from_language_setting(language);
    let terminal_label = translate(&locale, "workspace.create_split.terminal", "Terminal");
    let files_label = translate(&locale, "titlebar.files", "Files");
    let review_label = translate(&locale, "titlebar.review", "Review");
    div()
        .flex()
        .items_center()
        .gap_1()
        .h(px(32.0))
        .p(px(4.0))
        .rounded_sm()
        .bg(cx.theme().secondary)
        .child(workspace_segmented_tab(
            0,
            terminal_label,
            HeroIconName::CommandLine,
            active_index == 0,
            cx,
        ))
        .child(workspace_segmented_tab(
            1,
            files_label,
            HeroIconName::Document,
            active_index == 1,
            cx,
        ))
        .child(workspace_segmented_tab(
            2,
            review_label,
            HeroIconName::ArrowPathRoundedSquare,
            active_index == 2,
            cx,
        ))
}

fn workspace_segmented_tab(
    index: usize,
    label: impl Into<SharedString>,
    icon: HeroIconName,
    active: bool,
    cx: &mut Context<CoduxApp>,
) -> impl IntoElement {
    let label = label.into();
    div()
        .id(SharedString::from(format!("workspace-view-tab-{index}")))
        .h(px(22.0))
        .px_3()
        .flex()
        .items_center()
        .justify_center()
        .rounded_sm()
        .text_color(if active {
            cx.theme().primary_foreground
        } else {
            cx.theme().secondary_foreground
        })
        .bg(if active {
            cx.theme().primary
        } else {
            cx.theme().transparent
        })
        .cursor_pointer()
        .hover(|style| {
            if active {
                style.bg(cx.theme().primary)
            } else {
                style.bg(cx.theme().secondary_hover)
            }
        })
        .on_click(cx.listener(move |app, _event, window, cx| {
            let view = match index {
                0 => WorkspaceView::Terminal,
                1 => WorkspaceView::Files,
                _ => WorkspaceView::Review,
            };
            app.set_workspace_view(view, window, cx);
        }))
        .child(
            div()
                .h(px(16.0))
                .flex()
                .items_center()
                .justify_center()
                .gap_2()
                .child(
                    div()
                        .size(px(14.0))
                        .flex()
                        .flex_none()
                        .items_center()
                        .justify_center()
                        .child(Icon::new(icon).size_3()),
                )
                .child(div().flex_none().mt(px(1.0)).text_xs().child(label)),
        )
}

fn terminal_bottom_tab_button(
    terminal_id: usize,
    label: String,
    active: bool,
    cx: &mut Context<CoduxApp>,
) -> impl IntoElement {
    div()
        .id(SharedString::from(format!(
            "terminal-bottom-tab-{terminal_id}"
        )))
        .h(px(32.0))
        .px_3()
        .relative()
        .flex()
        .items_center()
        .gap_2()
        .rounded_md()
        .cursor_pointer()
        .text_color(if active {
            cx.theme().foreground
        } else {
            cx.theme().secondary_foreground
        })
        .bg(if active {
            cx.theme().secondary_hover
        } else {
            cx.theme().transparent
        })
        .hover(|style| style.bg(cx.theme().secondary_hover))
        .on_click(cx.listener(move |app, _event, window, cx| {
            app.select_terminal_tab(terminal_id, window, cx)
        }))
        .child(div().text_xs().line_height(px(14.0)).child(label))
        .child(
            div()
                .id(SharedString::from(format!(
                    "terminal-bottom-tab-close-{terminal_id}"
                )))
                .size(px(20.0))
                .flex()
                .items_center()
                .justify_center()
                .rounded_sm()
                .text_color(cx.theme().secondary_foreground)
                .hover(|style| style.bg(cx.theme().secondary_hover))
                .on_click(cx.listener(move |app, _event, window, cx| {
                    cx.stop_propagation();
                    window.prevent_default();
                    app.close_terminal_tab(terminal_id, window, cx)
                }))
                .child(Icon::new(HeroIconName::XMark).size_3()),
        )
}

fn terminal_bottom_add_button(cx: &mut Context<CoduxApp>) -> impl IntoElement {
    div()
        .id("terminal-bottom-tab-add")
        .size(px(26.0))
        .flex()
        .flex_none()
        .items_center()
        .justify_center()
        .rounded_sm()
        .cursor_pointer()
        .text_color(cx.theme().secondary_foreground)
        .hover(|style| style.bg(cx.theme().secondary_hover))
        .on_click(cx.listener(|app, _event, window, cx| app.add_terminal_tab(window, cx)))
        .child(Icon::new(HeroIconName::Plus).size_3p5())
}

pub(in crate::app) fn workspace_text_button(
    id: &'static str,
    label: &'static str,
    cx: &mut Context<CoduxApp>,
    on_click: impl Fn(&mut CoduxApp, &gpui::ClickEvent, &mut Window, &mut Context<CoduxApp>) + 'static,
) -> impl IntoElement {
    Button::new(id)
        .ghost()
        .text_color(cx.theme().secondary_foreground)
        .label(label)
        .on_click(cx.listener(on_click))
}
