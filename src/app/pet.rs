use super::*;
use codux_runtime::pet::{PetCatalogItem, PetLegacyRecord, PetSnapshot, PetStats};
use gpui_component::input::{Input, InputState};

use crate::app::workspace::workspace_pet_install_form;

impl CoduxApp {
    pub(in crate::app) fn open_pet_claim_window(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.open_pet_window(
            AppWindowMode::PetClaim,
            "Claim Pet",
            size(px(680.0), px(500.0)),
            size(px(640.0), px(460.0)),
            cx,
        );
    }

    pub(in crate::app) fn open_pet_custom_install_window(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.open_pet_window(
            AppWindowMode::PetCustomInstall,
            "Add Custom Pet",
            size(px(680.0), px(320.0)),
            size(px(620.0), px(240.0)),
            cx,
        );
    }

    pub(in crate::app) fn open_pet_dex_window(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.open_pet_window(
            AppWindowMode::PetDex,
            "Petdex",
            size(px(900.0), px(660.0)),
            size(px(780.0), px(560.0)),
            cx,
        );
    }

    fn open_pet_window(
        &mut self,
        mode: AppWindowMode,
        title: &'static str,
        window_size: gpui::Size<gpui::Pixels>,
        min_size: gpui::Size<gpui::Pixels>,
        cx: &mut Context<Self>,
    ) {
        let bounds = Bounds::centered(None, window_size, cx);
        let result = cx.open_window(
            WindowOptions {
                titlebar: Some(gpui::TitlebarOptions {
                    title: Some(title.into()),
                    appears_transparent: true,
                    ..Default::default()
                }),
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                window_min_size: Some(min_size),
                is_resizable: mode == AppWindowMode::PetDex,
                ..Default::default()
            },
            move |window, cx| {
                let app = CoduxApp::new_pet_window(mode);
                theme::apply_component_theme_for_name(&app.state.settings.theme, Some(window), cx);
                let view = cx.new(|_| app);
                cx.new(|cx| Root::new(view, window, cx))
            },
        );

        self.status_message = match result {
            Ok(_) => format!("{title} window opened"),
            Err(error) => format!("failed to open {title} window: {error}"),
        };
        cx.notify();
    }

    pub(in crate::app) fn pet_claim_workspace(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let catalog = self.runtime_service.pet_catalog();
        if self.pet_claim_species.is_empty() {
            self.pet_claim_species = if self.state.pet.species.is_empty() {
                catalog
                    .species
                    .first()
                    .map(|item| item.species.clone())
                    .unwrap_or_else(|| "voidcat".to_string())
            } else {
                self.state.pet.species.clone()
            };
        }
        let selected_species = self.pet_claim_species.clone();
        let claim_name_state = window.use_keyed_state("pet-claim-custom-name", cx, |window, cx| {
            InputState::new(window, cx).placeholder("留空则使用宠物名称")
        });
        let preview_pet = PetSummary {
            species: if selected_species == "bundled:random" {
                fallback_random_preview_species(&catalog.species)
            } else {
                selected_species.clone()
            },
            ..self.state.pet.clone()
        };
        let fallback_species = if selected_species.is_empty() {
            catalog
                .species
                .first()
                .map(|item| item.species.clone())
                .unwrap_or_else(|| "voidcat".to_string())
        } else {
            selected_species.clone()
        };
        let custom_pets = catalog.custom_pets.clone();

        pet_window_shell(
            "领取宠物",
            "选择一个 Codux 伙伴，也可以先安装自定义宠物。",
            cx,
        )
        .child(
            div()
                .min_h_0()
                .flex_1()
                .grid()
                .grid_cols(2)
                .overflow_hidden()
                .child(
                    div()
                        .min_h_0()
                        .border_r_1()
                        .border_color(color(theme::BORDER_SOFT))
                        .p(px(12.0))
                        .overflow_y_scrollbar()
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .gap(px(8.0))
                                .children(catalog.species.into_iter().map(|item| {
                                    pet_claim_option_row(item, &selected_species, window, cx)
                                        .into_any_element()
                                }))
                                .child(pet_claim_random_row(&selected_species, cx))
                                .when(!custom_pets.is_empty(), |this| {
                                    this.child(
                                        div()
                                            .pt(px(6.0))
                                            .px_1()
                                            .text_size(px(12.0))
                                            .line_height(px(16.0))
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .text_color(color(theme::TEXT_DIM))
                                            .child("自定义宠物"),
                                    )
                                    .children(
                                        custom_pets.into_iter().map(|pet| {
                                            pet_claim_custom_row(pet, &selected_species, cx)
                                                .into_any_element()
                                        }),
                                    )
                                }),
                        ),
                )
                .child(
                    div()
                        .min_h_0()
                        .flex()
                        .flex_col()
                        .child(div().min_h_0().flex_1().child(pet_claim_preview(
                            &preview_pet,
                            selected_species == "bundled:random",
                            &self.runtime.source_root,
                            &self.state.support_dir,
                            &self.pet_custom_pets,
                            cx,
                        )))
                        .child(div().px(px(20.0)).pb(px(16.0)).child(
                            Input::new(&claim_name_state).with_size(gpui_component::Size::Medium),
                        )),
                ),
        )
        .child(pet_footer_bar(pet_window_footer(vec![
            pet_footer_button(
                "pet-claim-open-custom-install",
                "添加自定义",
                IconName::Plus,
                false,
                cx,
                |app, _event, window, cx| app.open_pet_custom_install_window(window, cx),
            )
            .into_any_element(),
            pet_footer_spacer().into_any_element(),
            pet_cancel_button("pet-claim-cancel", cx).into_any_element(),
            pet_footer_button(
                "pet-claim-confirm",
                "确认领取",
                IconName::Check,
                true,
                cx,
                move |app, _event, window, cx| {
                    let selected = if app.pet_claim_species.is_empty() {
                        fallback_species.clone()
                    } else {
                        app.pet_claim_species.clone()
                    };
                    let species = if selected == "bundled:random" {
                        random_pet_species(&app.runtime_service.pet_catalog().species)
                    } else {
                        selected
                    };
                    let custom_name = claim_name_state.read(cx).value().to_string();
                    app.claim_pet_species(species, custom_name, window, cx);
                },
            )
            .into_any_element(),
        ])))
    }

    pub(in crate::app) fn pet_custom_install_workspace(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        pet_window_shell(
            "添加自定义宠物",
            "粘贴 Petdex 页面，先解析预览，再安装到本地 runtime。",
            cx,
        )
        .child(
            div()
                .min_h_0()
                .flex_1()
                .p(px(16.0))
                .overflow_y_scrollbar()
                .child(workspace_pet_install_form(
                    &self.pet_install_url,
                    &self.pet_install_display_name,
                    self.pet_install_preview.as_ref(),
                    self.pet_install_previewing,
                    self.pet_installing,
                    window,
                    cx,
                )),
        )
        .child(pet_footer_bar(pet_window_footer(vec![
            pet_cancel_button("pet-custom-install-cancel", cx).into_any_element(),
            pet_footer_spacer().into_any_element(),
            pet_footer_button(
                "pet-custom-install-window",
                "安装",
                IconName::Plus,
                true,
                cx,
                |app, _event, window, cx| app.install_custom_pet(window, cx),
            )
            .disabled(
                self.pet_install_preview.is_none()
                    || self.pet_install_previewing
                    || self.pet_installing,
            )
            .into_any_element(),
        ])))
    }

    pub(in crate::app) fn pet_dex_workspace(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let catalog = self.runtime_service.pet_catalog();
        let snapshot = self
            .runtime_service
            .pet_snapshot()
            .unwrap_or_else(|_| PetSnapshot::default());
        let current_custom_pet = snapshot.custom_pet.as_ref().map(|pet| {
            self.runtime_service
                .hydrate_custom_pet_data_url(pet.clone())
        });
        let primary_action = if self.state.pet.claimed {
            pet_inline_button(
                "pet-dex-archive",
                "归档当前",
                IconName::Delete,
                true,
                cx,
                |app, _event, window, cx| app.archive_current_pet(window, cx),
            )
            .into_any_element()
        } else {
            pet_inline_button(
                "pet-dex-claim",
                "领取宠物",
                IconName::Heart,
                true,
                cx,
                |app, _event, window, cx| app.open_pet_claim_window(window, cx),
            )
            .into_any_element()
        };
        let add_custom_action = pet_inline_button(
            "pet-dex-open-custom-install",
            "添加自定义",
            IconName::Plus,
            true,
            cx,
            |app, _event, window, cx| app.open_pet_custom_install_window(window, cx),
        )
        .into_any_element();

        pet_window_shell(
            "宠物图鉴",
            "查看当前伙伴、已归档伙伴和已安装的自定义宠物。",
            cx,
        )
        .child(
            div()
                .min_h_0()
                .flex_1()
                .grid()
                .grid_cols(2)
                .overflow_hidden()
                .child(
                    div()
                        .min_h_0()
                        .border_r_1()
                        .border_color(color(theme::BORDER_SOFT))
                        .p(px(16.0))
                        .overflow_y_scrollbar()
                        .flex()
                        .flex_col()
                        .gap(px(12.0))
                        .child(pet_dex_current_card(
                            &self.state.pet,
                            current_custom_pet.as_ref(),
                            &self.runtime.source_root,
                            &self.state.support_dir,
                            &self.pet_custom_pets,
                            cx,
                        ))
                        .child(pet_stats_grid(
                            &snapshot.current_stats,
                            snapshot.progress.total_xp,
                        ))
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(8.0))
                                .child(primary_action)
                                .child(add_custom_action),
                        )
                        .child(pet_legacy_section(snapshot.legacy, cx)),
                )
                .child(
                    div()
                        .min_h_0()
                        .p(px(16.0))
                        .overflow_y_scrollbar()
                        .flex()
                        .flex_col()
                        .gap(px(16.0))
                        .child(pet_catalog_section(catalog.species, cx))
                        .child(pet_custom_section(
                            catalog.custom_pets,
                            self.state.support_dir.clone(),
                            cx,
                        )),
                ),
        )
    }
}

fn pet_cancel_button(id: &'static str, cx: &mut Context<CoduxApp>) -> Button {
    Button::new(id)
        .compact()
        .ghost()
        .text_color(cx.theme().secondary_foreground)
        .label("取消")
        .on_click(|_, window, _| window.remove_window())
}

fn pet_window_shell(
    title: &'static str,
    subtitle: &'static str,
    cx: &mut Context<CoduxApp>,
) -> gpui::Div {
    div()
        .size_full()
        .flex()
        .flex_col()
        .bg(color(theme::BG))
        .text_color(color(theme::TEXT))
        .child(
            div()
                .h(px(56.0))
                .flex_shrink_0()
                .px(px(18.0))
                .flex()
                .items_center()
                .justify_between()
                .border_b_1()
                .border_color(color(theme::BORDER_SOFT))
                .child(
                    div()
                        .min_w_0()
                        .child(
                            div()
                                .text_size(px(14.0))
                                .line_height(px(18.0))
                                .font_weight(FontWeight::SEMIBOLD)
                                .child(title),
                        )
                        .child(
                            div()
                                .mt(px(2.0))
                                .truncate()
                                .text_size(px(12.0))
                                .line_height(px(16.0))
                                .text_color(color(theme::TEXT_MUTED))
                                .child(subtitle),
                        ),
                )
                .child(
                    Button::new("pet-window-refresh")
                        .compact()
                        .ghost()
                        .tooltip("刷新宠物数据")
                        .text_color(cx.theme().secondary_foreground)
                        .icon(
                            Icon::new(IconName::Redo2)
                                .size_3p5()
                                .text_color(cx.theme().secondary_foreground),
                        )
                        .on_click(
                            cx.listener(|app, _event, window, cx| app.refresh_pet(window, cx)),
                        ),
                ),
        )
}

fn pet_footer_bar(footer: impl IntoElement) -> impl IntoElement {
    div()
        .h(px(54.0))
        .flex_shrink_0()
        .border_t_1()
        .border_color(color(theme::BORDER_SOFT))
        .px(px(14.0))
        .flex()
        .items_center()
        .gap(px(8.0))
        .child(footer)
}

fn pet_window_footer(children: Vec<AnyElement>) -> impl IntoElement {
    div()
        .w_full()
        .flex()
        .items_center()
        .gap(px(8.0))
        .children(children)
}

fn pet_footer_spacer() -> impl IntoElement {
    div().flex_1()
}

fn pet_footer_button(
    id: &'static str,
    label: &'static str,
    icon: IconName,
    primary: bool,
    cx: &mut Context<CoduxApp>,
    on_click: impl Fn(&mut CoduxApp, &gpui::ClickEvent, &mut Window, &mut Context<CoduxApp>) + 'static,
) -> Button {
    let button = Button::new(id)
        .compact()
        .text_color(if primary {
            cx.theme().primary_foreground
        } else {
            cx.theme().secondary_foreground
        })
        .icon(Icon::new(icon).size_3p5())
        .label(label)
        .on_click(cx.listener(on_click));
    if primary {
        button.primary()
    } else {
        button.secondary()
    }
}

fn pet_inline_button(
    id: &'static str,
    label: &'static str,
    icon: IconName,
    enabled: bool,
    cx: &mut Context<CoduxApp>,
    on_click: impl Fn(&mut CoduxApp, &gpui::ClickEvent, &mut Window, &mut Context<CoduxApp>) + 'static,
) -> Button {
    Button::new(id)
        .compact()
        .secondary()
        .disabled(!enabled)
        .text_color(cx.theme().secondary_foreground)
        .icon(Icon::new(icon).size_3p5())
        .label(label)
        .on_click(cx.listener(on_click))
}

fn pet_claim_random_row(selected_species: &str, cx: &mut Context<CoduxApp>) -> impl IntoElement {
    let selected = selected_species == "bundled:random";

    pet_select_row(
        SharedString::from("pet-claim-random"),
        selected,
        "随机".to_string(),
        "让 Codux 为你选择一个伙伴".to_string(),
        cx,
    )
    .on_click(cx.listener(move |app, _event, _window, cx| {
        app.pet_claim_species = "bundled:random".to_string();
        app.status_message = "selected random pet".to_string();
        cx.notify();
    }))
}

fn pet_claim_option_row(
    item: PetCatalogItem,
    selected_species: &str,
    _window: &mut Window,
    cx: &mut Context<CoduxApp>,
) -> impl IntoElement {
    let selected = selected_species == item.species;
    let species = item.species.clone();
    let title = pet_species_name(&item.species);
    let subtitle = pet_species_subtitle(&item.species);

    pet_select_row(
        SharedString::from(format!("pet-claim-bundled-{}", item.species)),
        selected,
        title,
        subtitle,
        cx,
    )
    .on_click(cx.listener(move |app, _event, _window, cx| {
        app.pet_claim_species = species.clone();
        app.status_message = format!("selected pet species: {}", species);
        cx.notify();
    }))
}

fn pet_claim_custom_row(
    pet: PetCustomPet,
    selected_species: &str,
    cx: &mut Context<CoduxApp>,
) -> impl IntoElement {
    let selected = selected_species == format!("custom:{}", pet.id);
    let species = format!("custom:{}", pet.id);
    let title = pet.display_name.clone();
    let subtitle = empty_label(&pet.description);

    pet_select_row(
        SharedString::from(format!("pet-claim-custom-{}", pet.id)),
        selected,
        title,
        subtitle,
        cx,
    )
    .on_click(cx.listener(move |app, _event, _window, cx| {
        app.pet_claim_species = species.clone();
        app.status_message = format!("selected custom pet: {}", species);
        cx.notify();
    }))
}

fn pet_select_row(
    id: SharedString,
    selected: bool,
    title: String,
    subtitle: String,
    cx: &mut Context<CoduxApp>,
) -> gpui::Stateful<gpui::Div> {
    div()
        .id(id)
        .cursor_pointer()
        .rounded(px(6.0))
        .px(px(8.0))
        .py(px(7.0))
        .bg(if selected {
            cx.theme().secondary_hover
        } else {
            cx.theme().transparent
        })
        .hover(|style| style.bg(cx.theme().secondary_hover))
        .child(
            div()
                .text_size(px(14.0))
                .line_height(px(18.0))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(color(theme::TEXT))
                .child(title),
        )
        .child(
            div()
                .mt(px(2.0))
                .text_size(px(12.0))
                .line_height(px(15.0))
                .text_color(color(theme::TEXT_MUTED))
                .truncate()
                .child(subtitle),
        )
}

fn pet_claim_preview(
    pet: &PetSummary,
    random: bool,
    runtime_asset_root: &Path,
    support_dir: &Path,
    custom_pets: &[PetCustomPet],
    cx: &mut Context<CoduxApp>,
) -> impl IntoElement {
    let sprite_path = pet_sprite_path(runtime_asset_root, support_dir, pet, custom_pets);
    let title = if random {
        "随机".to_string()
    } else if pet.species.starts_with("custom:") {
        custom_pets
            .iter()
            .find(|custom| pet.species == format!("custom:{}", custom.id))
            .map(|custom| custom.display_name.clone())
            .unwrap_or_else(|| "自定义宠物".to_string())
    } else {
        pet_species_name(&pet.species)
    };
    let description = if random {
        "确认领取时会从内置宠物里随机选择一个伙伴。".to_string()
    } else {
        "领取后会使用现有 Tauri runtime 的宠物进度、等级和历史统计。".to_string()
    };

    div()
        .min_h_0()
        .p(px(20.0))
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .text_center()
        .child(
            div()
                .size(px(118.0))
                .rounded_full()
                .flex()
                .items_center()
                .justify_center()
                .overflow_hidden()
                .bg(color(theme::ACCENT).opacity(0.12))
                .child(if random {
                    Icon::new(IconName::Asterisk)
                        .size_8()
                        .text_color(cx.theme().primary)
                        .into_any_element()
                } else {
                    pet_sprite_element(sprite_path, 92.0, cx.theme().primary)
                }),
        )
        .child(
            div()
                .mt(px(14.0))
                .text_size(px(16.0))
                .line_height(px(20.0))
                .font_weight(FontWeight::BOLD)
                .child(title),
        )
        .child(
            div()
                .mt(px(6.0))
                .max_w(px(340.0))
                .text_size(px(12.0))
                .line_height(px(18.0))
                .text_color(color(theme::TEXT_MUTED))
                .child(description),
        )
}

fn fallback_random_preview_species(items: &[PetCatalogItem]) -> String {
    items
        .first()
        .map(|item| item.species.clone())
        .unwrap_or_else(|| "voidcat".to_string())
}

fn random_pet_species(items: &[PetCatalogItem]) -> String {
    if items.is_empty() {
        return "voidcat".to_string();
    }
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.subsec_nanos() as usize)
        .unwrap_or_default();
    items[nanos % items.len()].species.clone()
}

fn pet_dex_current_card(
    pet: &PetSummary,
    custom_pet: Option<&PetCustomPet>,
    runtime_asset_root: &Path,
    support_dir: &Path,
    custom_pets: &[PetCustomPet],
    cx: &mut Context<CoduxApp>,
) -> impl IntoElement {
    let sprite_path = pet_sprite_path(runtime_asset_root, support_dir, pet, custom_pets);
    let name = if pet.claimed {
        pet.display_name.clone()
    } else {
        "还没有领取宠物".to_string()
    };
    let description = custom_pet
        .map(|pet| empty_label(&pet.description))
        .unwrap_or_else(|| pet_species_subtitle(&pet.species));

    div()
        .rounded(px(8.0))
        .bg(color(0xFFFFFF).opacity(0.055))
        .border_1()
        .border_color(color(theme::BORDER_SOFT))
        .p(px(12.0))
        .flex()
        .items_center()
        .gap(px(12.0))
        .child(
            div()
                .size(px(54.0))
                .rounded(px(10.0))
                .overflow_hidden()
                .flex()
                .items_center()
                .justify_center()
                .bg(color(theme::ACCENT).opacity(0.12))
                .child(pet_sprite_element(sprite_path, 48.0, cx.theme().primary)),
        )
        .child(
            div()
                .min_w_0()
                .flex_1()
                .child(
                    div()
                        .truncate()
                        .text_size(px(14.0))
                        .line_height(px(18.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(name),
                )
                .child(
                    div()
                        .mt(px(3.0))
                        .truncate()
                        .text_size(px(12.0))
                        .line_height(px(16.0))
                        .text_color(color(theme::TEXT_MUTED))
                        .child(description),
                ),
        )
        .child(
            Tag::info()
                .with_size(gpui_component::Size::Small)
                .child(format!("Lv.{}", pet.level.max(1))),
        )
}

fn pet_stats_grid(stats: &PetStats, total_xp: i64) -> impl IntoElement {
    div()
        .grid()
        .grid_cols(3)
        .gap(px(8.0))
        .child(pet_stat_tile("总 XP", compact_number(total_xp)))
        .child(pet_stat_tile("智慧", stats.wisdom.to_string()))
        .child(pet_stat_tile("耐力", stats.stamina.to_string()))
        .child(pet_stat_tile("共情", stats.empathy.to_string()))
        .child(pet_stat_tile("夜行", stats.night.to_string()))
        .child(pet_stat_tile("混沌", stats.chaos.to_string()))
}

fn pet_stat_tile(label: &'static str, value: String) -> impl IntoElement {
    div()
        .rounded(px(7.0))
        .bg(color(0xFFFFFF).opacity(0.055))
        .px(px(10.0))
        .py(px(8.0))
        .child(
            div()
                .text_size(px(12.0))
                .line_height(px(16.0))
                .text_color(color(theme::TEXT_DIM))
                .child(label),
        )
        .child(
            div()
                .mt(px(2.0))
                .text_size(px(16.0))
                .line_height(px(20.0))
                .font_weight(FontWeight::BOLD)
                .child(value),
        )
}

fn pet_legacy_section(
    records: Vec<PetLegacyRecord>,
    cx: &mut Context<CoduxApp>,
) -> impl IntoElement {
    div()
        .rounded(px(8.0))
        .bg(color(0xFFFFFF).opacity(0.04))
        .border_1()
        .border_color(color(theme::BORDER_SOFT))
        .overflow_hidden()
        .child(pet_section_header("归档记录", records.len()))
        .child(
            div()
                .p(px(8.0))
                .flex()
                .flex_col()
                .gap(px(4.0))
                .when(records.is_empty(), |this| {
                    this.child(
                        div()
                            .px(px(6.0))
                            .py(px(8.0))
                            .text_size(px(12.0))
                            .line_height(px(16.0))
                            .text_color(color(theme::TEXT_DIM))
                            .child("暂无归档宠物"),
                    )
                })
                .children(records.into_iter().rev().map(|record| {
                    let legacy_id = record.id.clone();
                    div()
                        .rounded(px(6.0))
                        .px(px(7.0))
                        .py(px(6.0))
                        .flex()
                        .items_center()
                        .gap(px(8.0))
                        .hover(|style| style.bg(color(theme::BG_ROW_HOVER)))
                        .child(
                            div()
                                .min_w_0()
                                .flex_1()
                                .child(
                                    div()
                                        .truncate()
                                        .text_size(px(14.0))
                                        .line_height(px(18.0))
                                        .child(if record.custom_name.is_empty() {
                                            pet_species_name(&record.species)
                                        } else {
                                            record.custom_name
                                        }),
                                )
                                .child(
                                    div()
                                        .text_size(px(12.0))
                                        .line_height(px(15.0))
                                        .text_color(color(theme::TEXT_DIM))
                                        .child(format!(
                                            "Lv.{} · {}",
                                            record.progress.level, record.species
                                        )),
                                ),
                        )
                        .child(
                            Button::new(SharedString::from(format!(
                                "pet-restore-legacy-{legacy_id}"
                            )))
                            .compact()
                            .ghost()
                            .tooltip("恢复这个宠物")
                            .text_color(cx.theme().secondary_foreground)
                            .icon(Icon::new(IconName::Undo2).size_3p5())
                            .on_click(cx.listener(
                                move |app, _event, _window, cx| {
                                    match app.runtime_service.restore_archived_pet(
                                        PetRestoreRequest {
                                            legacy_id: legacy_id.clone(),
                                        },
                                    ) {
                                        Ok(_) => {
                                            app.state.pet = app.runtime_service.reload_pet();
                                            app.status_message = "pet restored".to_string();
                                        }
                                        Err(error) => {
                                            app.status_message =
                                                format!("failed to restore pet: {error}");
                                        }
                                    }
                                    cx.notify();
                                },
                            )),
                        )
                        .into_any_element()
                })),
        )
}

fn pet_catalog_section(
    items: Vec<PetCatalogItem>,
    _cx: &mut Context<CoduxApp>,
) -> impl IntoElement {
    div()
        .rounded(px(8.0))
        .bg(color(0xFFFFFF).opacity(0.04))
        .border_1()
        .border_color(color(theme::BORDER_SOFT))
        .overflow_hidden()
        .child(pet_section_header("内置宠物", items.len()))
        .child(
            div()
                .p(px(10.0))
                .grid()
                .grid_cols(3)
                .gap(px(8.0))
                .children(items.into_iter().map(|item| {
                    div()
                        .rounded(px(7.0))
                        .bg(color(0xFFFFFF).opacity(0.045))
                        .px(px(9.0))
                        .py(px(8.0))
                        .child(
                            div()
                                .text_size(px(14.0))
                                .line_height(px(18.0))
                                .font_weight(FontWeight::SEMIBOLD)
                                .child(pet_species_name(&item.species)),
                        )
                        .child(
                            div()
                                .mt(px(2.0))
                                .text_size(px(12.0))
                                .line_height(px(15.0))
                                .truncate()
                                .text_color(color(theme::TEXT_DIM))
                                .child(pet_species_subtitle(&item.species)),
                        )
                        .into_any_element()
                })),
        )
}

fn pet_custom_section(
    custom_pets: Vec<PetCustomPet>,
    support_dir: PathBuf,
    cx: &mut Context<CoduxApp>,
) -> impl IntoElement {
    div()
        .rounded(px(8.0))
        .bg(color(0xFFFFFF).opacity(0.04))
        .border_1()
        .border_color(color(theme::BORDER_SOFT))
        .overflow_hidden()
        .child(pet_section_header("自定义宠物", custom_pets.len()))
        .child(
            div()
                .p(px(10.0))
                .flex()
                .flex_col()
                .gap(px(6.0))
                .when(custom_pets.is_empty(), |this| {
                    this.child(
                        div()
                            .text_size(px(12.0))
                            .line_height(px(16.0))
                            .text_color(color(theme::TEXT_DIM))
                            .child("还没有安装自定义宠物"),
                    )
                })
                .children(custom_pets.into_iter().map(|pet| {
                    let sprite_path = custom_pet_sprite_path(&support_dir, &pet);
                    let claim_pet = pet.clone();
                    div()
                        .rounded(px(7.0))
                        .px(px(7.0))
                        .py(px(6.0))
                        .flex()
                        .items_center()
                        .gap(px(8.0))
                        .hover(|style| style.bg(color(theme::BG_ROW_HOVER)))
                        .child(
                            div()
                                .size(px(30.0))
                                .rounded(px(6.0))
                                .overflow_hidden()
                                .flex()
                                .items_center()
                                .justify_center()
                                .bg(color(0xFFFFFF).opacity(0.055))
                                .child(pet_sprite_element(sprite_path, 28.0, cx.theme().primary)),
                        )
                        .child(
                            div()
                                .min_w_0()
                                .flex_1()
                                .child(
                                    div()
                                        .truncate()
                                        .text_size(px(14.0))
                                        .line_height(px(18.0))
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .child(pet.display_name),
                                )
                                .child(
                                    div()
                                        .truncate()
                                        .text_size(px(12.0))
                                        .line_height(px(15.0))
                                        .text_color(color(theme::TEXT_DIM))
                                        .child(empty_label(&pet.description)),
                                ),
                        )
                        .child(
                            Button::new(SharedString::from(format!(
                                "pet-dex-claim-custom-{}",
                                pet.id
                            )))
                            .compact()
                            .ghost()
                            .tooltip("领取这个自定义宠物")
                            .text_color(cx.theme().secondary_foreground)
                            .icon(Icon::new(IconName::Check).size_3p5())
                            .on_click(cx.listener(
                                move |app, _event, window, cx| {
                                    app.claim_custom_pet(claim_pet.clone(), window, cx)
                                },
                            )),
                        )
                        .into_any_element()
                })),
        )
}

fn pet_section_header(label: &'static str, count: usize) -> impl IntoElement {
    div()
        .h(px(34.0))
        .px(px(10.0))
        .flex()
        .items_center()
        .gap(px(8.0))
        .bg(color(0xFFFFFF).opacity(0.045))
        .child(
            div()
                .text_size(px(14.0))
                .line_height(px(18.0))
                .font_weight(FontWeight::SEMIBOLD)
                .child(label),
        )
        .child(
            div()
                .px(px(7.0))
                .py(px(1.0))
                .rounded(px(999.0))
                .bg(color(theme::ACCENT).opacity(0.16))
                .text_size(px(12.0))
                .line_height(px(15.0))
                .text_color(color(theme::ACCENT))
                .child(count.to_string()),
        )
}

fn pet_species_name(species: &str) -> String {
    match species.strip_prefix("custom:").unwrap_or(species) {
        "voidcat" => "Voidcat",
        "rusthound" => "Rusthound",
        "goose" => "Goose",
        "chaossprite" => "Chaos Sprite",
        "code" => "Code",
        "sheep" => "Sheep",
        "ox" => "Ox",
        "dragon" => "Dragon",
        "phoenix" => "Phoenix",
        "dolphin" => "Dolphin",
        "penguin" => "Penguin",
        "panda" => "Panda",
        value if value.is_empty() => "Voidcat",
        value => value,
    }
    .to_string()
}

fn pet_species_subtitle(species: &str) -> String {
    match species.strip_prefix("custom:").unwrap_or(species) {
        "voidcat" => "安静观察代码变化",
        "rusthound" => "偏爱 Rust 和编译反馈",
        "goose" => "会盯住任务节奏",
        "chaossprite" => "适合高频试验",
        "code" => "默认编码伙伴",
        "sheep" => "温和的长线陪伴",
        "ox" => "稳定推进任务",
        "dragon" => "适合重构和冲刺",
        "phoenix" => "适合恢复和复盘",
        "dolphin" => "适合协作和探索",
        "penguin" => "适合终端工作流",
        "panda" => "适合安静维护",
        _ => "Codux 宠物伙伴",
    }
    .to_string()
}
