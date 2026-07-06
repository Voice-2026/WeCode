use super::*;

impl CoduxApp {
    pub(in crate::app) fn pet_dex_workspace(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let catalog = self.pet_catalog.clone();
        let snapshot = self.pet_snapshot.clone();
        let current_custom_pet = snapshot.custom_pet.as_ref().map(|pet| {
            self.runtime_service
                .hydrate_custom_pet_data_url(pet.clone())
        });
        let mut unlocked_species = HashSet::new();
        if self.state.pet.claimed && snapshot.custom_pet.is_none() {
            unlocked_species.insert(self.state.pet.species.clone());
        }
        for record in &snapshot.legacy {
            if record.custom_pet.is_none() {
                unlocked_species.insert(record.species.clone());
            }
        }
        let custom_pets = catalog.custom_pets.clone();
        let current_catalog_item = catalog
            .species
            .iter()
            .find(|item| item.species == self.state.pet.species)
            .cloned();
        let total_count = catalog.species.len() + custom_pets.len();
        let unlocked_count = unlocked_species.len() + custom_pets.len();
        let language = self.state.settings.language.clone();
        let current_name = if self.state.pet.claimed {
            self.state.pet.display_name.clone()
        } else {
            pet_catalog_text(&language, "pet.dex.unclaimed", "Not Claimed")
        };
        let current_level = if self.state.pet.claimed {
            pet_format_placeholders(
                &pet_catalog_text(&language, "pet.dex.current_level_format", "Lv.%@"),
                &[snapshot.progress.level.max(1).to_string()],
            )
        } else {
            pet_catalog_text(&language, "pet.dex.unclaimed", "Not Claimed")
        };
        let archived_count = snapshot.legacy.len();
        let archived_subtitle = if archived_count == 0 {
            pet_catalog_text(&language, "pet.dex.archived.none", "No archived pets yet")
        } else {
            pet_catalog_text(&language, "pet.dex.archived.history", "Past companions")
        };
        let collection_subtitle = if total_count > 0 && unlocked_count == total_count {
            pet_catalog_text(
                &language,
                "pet.dex.collection.complete",
                "All companions unlocked",
            )
        } else {
            pet_catalog_text(&language, "pet.dex.collection.continue", "Keep exploring")
        };
        let primary_action = if self.state.pet.claimed {
            pet_inline_button(
                "pet-dex-archive",
                pet_catalog_text(&language, "pet.archive.action", "Archive"),
                HeroIconName::Trash,
                true,
                cx,
                |app, _event, window, cx| app.archive_current_pet(window, cx),
            )
            .into_any_element()
        } else {
            pet_inline_button(
                "pet-dex-claim",
                pet_catalog_text(&language, "pet.claim.action", "Claim Pet"),
                HeroIconName::Heart,
                true,
                cx,
                |app, _event, window, cx| app.defer_open_pet_claim_window(window, cx),
            )
            .into_any_element()
        };
        let add_custom_action = pet_inline_button(
            "pet-dex-open-custom-install",
            pet_catalog_text(&language, "pet.custom.install.action", "Add Custom Pet"),
            HeroIconName::Plus,
            true,
            cx,
            |app, _event, window, cx| app.defer_open_pet_custom_install_window(window, cx),
        )
        .into_any_element();

        child_window_shell(
            pet_catalog_text(&language, "pet.dex.window.title", "Pet Dex"),
            cx,
        )
        .child(
            div()
                .min_h_0()
                .flex_1()
                .relative()
                .flex()
                .overflow_hidden()
                .child(
                    div()
                        .w(px(270.0))
                        .flex_none()
                        .min_h_0()
                        .border_r_1()
                        .border_color(color(theme::BORDER_SOFT))
                        .flex()
                        .flex_col()
                        .child(
                            div().min_h_0().flex_1().overflow_y_scrollbar().child(
                                div()
                                    .p(px(16.0))
                                    .flex()
                                    .flex_col()
                                    .child(pet_dex_sidebar_overview(
                                        &language,
                                        current_name,
                                        current_level,
                                        archived_count,
                                        archived_subtitle,
                                        unlocked_count,
                                        total_count,
                                        collection_subtitle,
                                        cx,
                                    ))
                                    .child(pet_dex_current_card(
                                        &self.state.pet,
                                        current_custom_pet.as_ref(),
                                        current_catalog_item.as_ref(),
                                        &self.runtime.source_root,
                                        &self.state.support_dir,
                                        &self.pet_custom_pets,
                                        &snapshot.current_stats,
                                        snapshot.progress.total_xp,
                                        snapshot.claimed_at,
                                        &language,
                                        cx,
                                    )),
                            ),
                        )
                        .child(
                            div()
                                .flex_none()
                                .border_t_1()
                                .border_color(color(theme::BORDER_SOFT))
                                .p(px(16.0))
                                .flex()
                                .flex_col()
                                .gap(px(8.0))
                                .child(pet_dex_sidebar_action(primary_action))
                                .child(pet_dex_sidebar_action(add_custom_action)),
                        ),
                )
                .child(self.pet_dex_virtual_content(
                    catalog.species.clone(),
                    unlocked_species,
                    custom_pets,
                    snapshot.legacy,
                    _window,
                    cx,
                )),
        )
        .when_some(self.pet_dex_spotlight.clone(), |this, spotlight| {
            this.child(pet_dex_spotlight_overlay(
                spotlight,
                &catalog,
                &self.runtime.source_root,
                &self.state.support_dir,
                &self.state.settings.language,
                self.visible_pet_sprite_frame(PET_IDLE_FRAME_COUNT),
                cx,
            ))
        })
    }

    fn pet_dex_virtual_content(
        &mut self,
        bundled_items: Vec<PetCatalogItem>,
        unlocked_species: HashSet<String>,
        custom_pets: Vec<PetCustomPet>,
        legacy_records: Vec<PetLegacyRecord>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let rows = Rc::new(pet_dex_virtual_rows(
            bundled_items,
            &unlocked_species,
            custom_pets,
            legacy_records,
            &self.runtime.source_root,
            &self.state.support_dir,
            &self.pet_sprite_paths,
            &self.state.settings.language,
            window,
        ));
        let item_sizes = Rc::new(
            rows.iter()
                .map(|row| size(px(1.0), row.height()))
                .collect::<Vec<_>>(),
        );
        let scroll_handle = self.pet_dex_scroll_handle.clone();

        div()
            .flex_1()
            .min_h_0()
            .relative()
            .overflow_hidden()
            .child(
                v_virtual_list(
                    cx.entity().clone(),
                    "pet-dex-virtual-content",
                    item_sizes,
                    move |_app, visible_range: Range<usize>, _window, cx| {
                        visible_range
                            .filter_map(|index| {
                                rows.get(index).map(|row| row.render(&rows, index, cx))
                            })
                            .collect::<Vec<_>>()
                    },
                )
                .track_scroll(&scroll_handle)
                .with_sizing_behavior(ListSizingBehavior::Auto),
            )
            .vertical_scrollbar(&scroll_handle)
            .into_any_element()
    }
}
#[derive(Clone)]
enum PetDexVirtualRow {
    Spacer {
        height: f32,
    },
    SectionHeader {
        label: String,
        trailing: Option<String>,
    },
    PetCardRow {
        cards: Vec<PetDexCard>,
        columns: usize,
    },
    EmptyState {
        message: String,
    },
    LegacyRow {
        record: PetLegacyRecord,
        sprite_path: ImageSource,
        language: String,
    },
}

#[derive(Clone)]
enum PetDexCard {
    Bundled {
        item: PetCatalogItem,
        unlocked: bool,
        sprite_path: Option<ImageSource>,
        title: String,
        subtitle: String,
    },
    Custom {
        pet: PetCustomPet,
        sprite_path: ImageSource,
        subtitle: String,
    },
}

impl PetDexVirtualRow {
    fn height(&self) -> gpui::Pixels {
        px(match self {
            PetDexVirtualRow::Spacer { height } => *height,
            PetDexVirtualRow::SectionHeader { .. } => 34.0,
            PetDexVirtualRow::PetCardRow { .. } => 148.0,
            PetDexVirtualRow::EmptyState { .. } => 84.0,
            PetDexVirtualRow::LegacyRow { .. } => 72.0,
        })
    }

    fn render(
        &self,
        rows: &Rc<Vec<PetDexVirtualRow>>,
        index: usize,
        cx: &mut Context<CoduxApp>,
    ) -> gpui::Div {
        match self {
            PetDexVirtualRow::Spacer { .. } => div().w_full(),
            PetDexVirtualRow::SectionHeader { label, trailing } => div()
                .w_full()
                .px(px(20.0))
                .pt(px(8.0))
                .child(pet_section_header(label.clone(), trailing.clone())),
            PetDexVirtualRow::PetCardRow { cards, columns } => div()
                .w_full()
                .px(px(20.0))
                .pt(px(12.0))
                .flex()
                .gap(px(12.0))
                .children(
                    cards
                        .iter()
                        .cloned()
                        .map(|card| pet_dex_virtual_card(card, cx)),
                )
                .children(
                    (cards.len()..*columns)
                        .map(|_| div().flex_1().min_w_0().h(px(136.0)).into_any_element()),
                ),
            PetDexVirtualRow::EmptyState { message } => div()
                .w_full()
                .px(px(20.0))
                .pt(px(12.0))
                .child(pet_dex_empty_state(message.clone(), cx)),
            PetDexVirtualRow::LegacyRow {
                record,
                sprite_path,
                language,
            } => div()
                .w_full()
                .px(px(20.0))
                .pt(px(8.0))
                .child(pet_legacy_row(
                    record.clone(),
                    sprite_path.clone(),
                    language.clone(),
                    cx,
                )),
        }
        .when(
            matches!(self, PetDexVirtualRow::LegacyRow { .. })
                && rows
                    .get(index + 1)
                    .map(|next| !matches!(next, PetDexVirtualRow::LegacyRow { .. }))
                    .unwrap_or(true),
            |this| this.mb(px(12.0)),
        )
    }
}

fn pet_dex_virtual_rows(
    bundled_items: Vec<PetCatalogItem>,
    unlocked_species: &HashSet<String>,
    custom_pets: Vec<PetCustomPet>,
    legacy_records: Vec<PetLegacyRecord>,
    runtime_asset_root: &Path,
    support_dir: &Path,
    sprite_paths: &HashMap<String, ImageSource>,
    language: &str,
    window: &mut Window,
) -> Vec<PetDexVirtualRow> {
    let columns = pet_dex_columns(window);
    let mut rows = Vec::new();
    let unlocked_count = bundled_items
        .iter()
        .filter(|item| unlocked_species.contains(&item.species))
        .count();
    let total_count = bundled_items.len();

    rows.push(PetDexVirtualRow::Spacer { height: 20.0 });
    rows.push(PetDexVirtualRow::SectionHeader {
        label: pet_catalog_text(language, "pet.dex.bundled.section", "Bundled Pets"),
        trailing: Some(pet_format_placeholders(
            &pet_catalog_text(language, "pet.dex.unlocked_count", "%@/%@ unlocked"),
            &[unlocked_count.to_string(), total_count.to_string()],
        )),
    });
    for chunk in bundled_items.chunks(columns) {
        rows.push(PetDexVirtualRow::PetCardRow {
            columns,
            cards: chunk
                .iter()
                .map(|item| {
                    let unlocked = unlocked_species.contains(&item.species);
                    PetDexCard::Bundled {
                        item: item.clone(),
                        unlocked,
                        sprite_path: sprite_paths.get(&item.species).cloned(),
                        title: if unlocked {
                            pet_catalog_text(
                                language,
                                &item.name_key,
                                &pet_species_name(&item.species),
                            )
                        } else {
                            pet_catalog_text(language, "pet.dex.unknown", "???")
                        },
                        subtitle: if unlocked {
                            pet_catalog_text(language, "pet.stage.companion", "Companion")
                        } else {
                            pet_catalog_text(language, "pet.dex.locked", "Locked")
                        },
                    }
                })
                .collect(),
        });
    }

    rows.push(PetDexVirtualRow::Spacer { height: 28.0 });
    rows.push(PetDexVirtualRow::SectionHeader {
        label: pet_catalog_text(language, "pet.claim.custom.section", "Custom Pets"),
        trailing: Some(pet_format_placeholders(
            &pet_catalog_text(language, "pet.custom.installed_count", "%@ installed"),
            &[custom_pets.len().to_string()],
        )),
    });
    if custom_pets.is_empty() {
        rows.push(PetDexVirtualRow::EmptyState {
            message: pet_catalog_text(
                language,
                "pet.custom.install.subtitle",
                "Install a Codex-format pet from Petdex.",
            ),
        });
    } else {
        for chunk in custom_pets.chunks(columns) {
            rows.push(PetDexVirtualRow::PetCardRow {
                columns,
                cards: chunk
                    .iter()
                    .map(|pet| PetDexCard::Custom {
                        pet: pet.clone(),
                        sprite_path: sprite_paths
                            .get(&format!("custom:{}", pet.id))
                            .cloned()
                            .unwrap_or_else(|| custom_pet_sprite_path(support_dir, pet).into()),
                        subtitle: pet_catalog_text(language, "pet.custom.installed", "Custom pet"),
                    })
                    .collect(),
            });
        }
    }

    rows.push(PetDexVirtualRow::Spacer { height: 28.0 });
    rows.push(PetDexVirtualRow::SectionHeader {
        label: pet_catalog_text(language, "pet.archive.history", "Archive History"),
        trailing: None,
    });
    if legacy_records.is_empty() {
        rows.push(PetDexVirtualRow::EmptyState {
            message: pet_catalog_text(language, "pet.dex.archived.none", "No archived pets yet"),
        });
    } else {
        for record in legacy_records.into_iter().rev() {
            let sprite_path = legacy_pet_sprite_path(runtime_asset_root, support_dir, &record);
            rows.push(PetDexVirtualRow::LegacyRow {
                record,
                sprite_path,
                language: language.to_string(),
            });
        }
    }
    rows.push(PetDexVirtualRow::Spacer { height: 20.0 });

    rows
}

fn pet_dex_columns(window: &mut Window) -> usize {
    let width = window.viewport_size().width.as_f32();
    if width >= 1160.0 {
        5
    } else if width >= 900.0 {
        4
    } else {
        3
    }
}

fn pet_dex_card_frame(id: SharedString) -> gpui::Stateful<gpui::Div> {
    div()
        .id(id)
        .flex_1()
        .min_w_0()
        .h(px(136.0))
        .rounded(px(8.0))
        .border_1()
        .px(px(10.0))
        .py(px(12.0))
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .text_center()
}

fn pet_dex_virtual_card(card: PetDexCard, cx: &mut Context<CoduxApp>) -> AnyElement {
    match card {
        PetDexCard::Bundled {
            item,
            unlocked,
            sprite_path,
            title,
            subtitle,
        } => {
            let species = item.species.clone();
            pet_dex_card_frame(SharedString::from(format!("pet-dex-bundled-{species}")))
                .cursor_pointer()
                .border_color(if unlocked {
                    color(theme::ACCENT).opacity(0.25)
                } else {
                    color(theme::BORDER_SOFT)
                })
                .bg(if unlocked {
                    cx.theme().secondary
                } else {
                    cx.theme().group_box
                })
                .opacity(if unlocked { 1.0 } else { 0.8 })
                .hover(|style| style.bg(cx.theme().secondary_hover))
                .on_click(cx.listener(move |app, _event, _window, cx| {
                    if unlocked {
                        app.show_pet_dex_spotlight(PetDexSpotlight::Bundled(species.clone()), cx);
                    }
                }))
                .child(
                    div()
                        .size(px(56.0))
                        .rounded_full()
                        .flex()
                        .items_center()
                        .justify_center()
                        .bg(if unlocked {
                            color(pet_accent_color(&item.species)).opacity(0.16)
                        } else {
                            cx.theme().secondary
                        })
                        .child(if unlocked {
                            if let Some(sprite_path) = sprite_path {
                                pet_sprite_element(sprite_path, 44.0, 0, 0, cx.theme().primary)
                            } else {
                                div()
                                    .text_size(rems(1.75))
                                    .line_height(rems(2.0))
                                    .font_weight(FontWeight::BOLD)
                                    .text_color(color(theme::TEXT_DIM))
                                    .child("?")
                                    .into_any_element()
                            }
                        } else {
                            div()
                                .text_size(rems(1.75))
                                .line_height(rems(2.0))
                                .font_weight(FontWeight::BOLD)
                                .text_color(color(theme::TEXT_DIM))
                                .child("?")
                                .into_any_element()
                        }),
                )
                .child(
                    div()
                        .mt(px(10.0))
                        .w_full()
                        .truncate()
                        .text_size(rems(0.75))
                        .line_height(rems(1.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(title),
                )
                .child(
                    div()
                        .mt(px(4.0))
                        .text_size(rems(0.75))
                        .line_height(rems(1.0))
                        .truncate()
                        .text_color(if unlocked {
                            color(theme::TEXT_MUTED)
                        } else {
                            color(theme::TEXT_DIM)
                        })
                        .child(subtitle),
                )
                .into_any_element()
        }
        PetDexCard::Custom {
            pet,
            sprite_path,
            subtitle,
        } => {
            let pet_id = pet.id.clone();
            pet_dex_card_frame(SharedString::from(format!("pet-dex-custom-{pet_id}")))
                .cursor_pointer()
                .border_color(color(theme::ACCENT).opacity(0.25))
                .bg(cx.theme().secondary)
                .hover(|style| style.bg(cx.theme().secondary_hover))
                .on_click(cx.listener(move |app, _event, _window, cx| {
                    app.show_pet_dex_spotlight(PetDexSpotlight::Custom(pet_id.clone()), cx);
                }))
                .child(
                    div()
                        .size(px(56.0))
                        .rounded_full()
                        .flex()
                        .items_center()
                        .justify_center()
                        .bg(color(theme::ACCENT).opacity(0.12))
                        .child(pet_sprite_element(
                            sprite_path,
                            44.0,
                            0,
                            0,
                            cx.theme().primary,
                        )),
                )
                .child(
                    div()
                        .mt(px(10.0))
                        .w_full()
                        .truncate()
                        .text_size(rems(0.75))
                        .line_height(rems(1.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(pet.display_name),
                )
                .child(
                    div()
                        .mt(px(4.0))
                        .w_full()
                        .truncate()
                        .text_size(rems(0.75))
                        .line_height(rems(1.0))
                        .text_color(color(theme::ACCENT))
                        .child(subtitle),
                )
                .into_any_element()
        }
    }
}

fn pet_dex_empty_state(message: String, cx: &mut Context<CoduxApp>) -> impl IntoElement {
    div()
        .rounded(px(10.0))
        .border_1()
        .border_color(color(theme::BORDER_SOFT))
        .bg(cx.theme().group_box)
        .px(px(14.0))
        .py(px(24.0))
        .text_center()
        .text_size(rems(0.75))
        .line_height(rems(1.0))
        .text_color(color(theme::TEXT_DIM))
        .child(message)
}

fn pet_legacy_row(
    record: PetLegacyRecord,
    sprite_path: ImageSource,
    language: String,
    cx: &mut Context<CoduxApp>,
) -> impl IntoElement {
    let legacy_id = record.id.clone();
    let pet_name = if record.custom_name.trim().is_empty() {
        record
            .custom_pet
            .as_ref()
            .map(|pet| pet.display_name.clone())
            .unwrap_or_else(|| pet_species_name(&record.species))
    } else {
        record.custom_name.clone()
    };

    div()
        .rounded(px(8.0))
        .bg(cx.theme().secondary)
        .px(px(12.0))
        .py(px(10.0))
        .flex()
        .items_center()
        .gap(px(12.0))
        .hover(|style| style.bg(cx.theme().secondary_hover))
        .child(
            div()
                .size(px(44.0))
                .rounded(px(8.0))
                .overflow_hidden()
                .flex_none()
                .flex()
                .items_center()
                .justify_center()
                .bg(cx.theme().group_box)
                .child(pet_sprite_element(
                    sprite_path,
                    38.0,
                    0,
                    0,
                    cx.theme().primary,
                )),
        )
        .child(
            div()
                .min_w_0()
                .flex_1()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(8.0))
                        .child(
                            div()
                                .min_w_0()
                                .truncate()
                                .text_size(rems(0.875))
                                .line_height(rems(1.125))
                                .font_weight(FontWeight::SEMIBOLD)
                                .child(pet_name),
                        )
                        .child(
                            div()
                                .rounded_full()
                                .bg(color(theme::ACCENT).opacity(0.12))
                                .px(px(8.0))
                                .py(px(2.0))
                                .text_size(rems(0.75))
                                .line_height(rems(0.875))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(color(theme::ACCENT))
                                .child(pet_catalog_text(
                                    &language,
                                    "pet.stage.companion",
                                    "Companion",
                                )),
                        ),
                )
                .child(
                    div()
                        .mt(px(2.0))
                        .text_size(rems(0.75))
                        .line_height(rems(1.0))
                        .text_color(color(theme::TEXT_DIM))
                        .child(format!(
                            "{} XP · Lv.{}",
                            compact_number(record.total_xp),
                            record.progress.level
                        )),
                ),
        )
        .child(
            div()
                .w(px(100.0))
                .flex_none()
                .text_right()
                .text_size(rems(0.75))
                .line_height(rems(1.0))
                .text_color(color(theme::TEXT_DIM))
                .child(pet_date_label(record.retired_at)),
        )
        .child(with_codux_tooltip(
            cx.entity(),
            format!("pet-restore-legacy-tooltip-{legacy_id}"),
            Button::new(SharedString::from(format!(
                "pet-restore-legacy-{legacy_id}"
            )))
            .compact()
            .ghost()
            .text_color(cx.theme().secondary_foreground)
            .icon(Icon::new(HeroIconName::ArrowUturnLeft).size_3p5())
            .on_click(cx.listener(move |app, _event, _window, cx| {
                let legacy_id = legacy_id.clone();
                app.run_pet_change_async(
                    "restore_pet",
                    "restoring pet".to_string(),
                    move |service| {
                        service
                            .restore_archived_pet(PetRestoreRequest { legacy_id })
                            .map(|_| ())
                    },
                    |app, _cx| {
                        app.pet_dex_spotlight = None;
                        app.status_message = "pet restored".to_string();
                    },
                    cx,
                );
            })),
            pet_catalog_text(&language, "pet.archive.restore.action", "Restore"),
        ))
}
fn pet_dex_current_card(
    pet: &PetSummary,
    custom_pet: Option<&PetCustomPet>,
    catalog_item: Option<&PetCatalogItem>,
    runtime_asset_root: &Path,
    support_dir: &Path,
    custom_pets: &[PetCustomPet],
    stats: &PetStats,
    total_xp: i64,
    claimed_at: Option<i64>,
    language: &str,
    cx: &mut Context<CoduxApp>,
) -> impl IntoElement {
    let sprite_path = pet_sprite_path(runtime_asset_root, support_dir, pet, custom_pets);
    let name = if pet.claimed {
        pet.display_name.clone()
    } else {
        pet_catalog_text(language, "pet.dex.no_current_pet", "No active pet yet")
    };
    let description = custom_pet
        .map(|pet| empty_label(&pet.description))
        .or_else(|| {
            catalog_item.map(|item| {
                pet_catalog_text(
                    language,
                    &item.subtitle_key,
                    &pet_species_subtitle(&pet.species),
                )
            })
        })
        .unwrap_or_else(|| pet_species_subtitle(&pet.species));
    let level = pet_format_placeholders(
        &pet_catalog_text(language, "pet.dex.current_level_format", "Lv.%@"),
        &[pet.level.max(1).to_string()],
    );

    div()
        .child(
            div()
                .mb(px(8.0))
                .text_size(rems(0.75))
                .line_height(rems(1.0))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(color(theme::TEXT_MUTED))
                .child(pet_catalog_text(
                    language,
                    "pet.dex.current_pet",
                    "Current Pet",
                )),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .items_center()
                .gap(px(8.0))
                .child(
                    div()
                        .size(px(84.0))
                        .overflow_hidden()
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(pet_sprite_element(
                            sprite_path,
                            84.0,
                            0,
                            0,
                            cx.theme().primary,
                        )),
                )
                .child(
                    div()
                        .max_w(px(210.0))
                        .truncate()
                        .text_center()
                        .text_size(rems(0.875))
                        .line_height(rems(1.125))
                        .font_weight(FontWeight::BOLD)
                        .child(name),
                )
                .child(
                    div()
                        .max_w(px(210.0))
                        .truncate()
                        .text_center()
                        .text_size(rems(0.75))
                        .line_height(rems(1.0))
                        .text_color(color(theme::TEXT_MUTED))
                        .child(format!("{description} · {level}")),
                ),
        )
        .child(
            div()
                .mt(px(16.0))
                .flex()
                .flex_col()
                .gap(px(8.0))
                .child(pet_trait_bar(
                    "🧠",
                    pet_catalog_text(language, "pet.attribute.wisdom", "Wisdom"),
                    stats.wisdom,
                    0x2F8FFF,
                ))
                .child(pet_trait_bar(
                    "🔥",
                    pet_catalog_text(language, "pet.attribute.chaos", "Chaos"),
                    stats.chaos,
                    0xFF6030,
                ))
                .child(pet_trait_bar(
                    "🌙",
                    pet_catalog_text(language, "pet.attribute.night", "Night"),
                    stats.night,
                    0x6060CC,
                ))
                .child(pet_trait_bar(
                    "💪",
                    pet_catalog_text(language, "pet.attribute.stamina", "Stamina"),
                    stats.stamina,
                    0x20A060,
                ))
                .child(pet_trait_bar(
                    "🩹",
                    pet_catalog_text(language, "pet.attribute.empathy", "Empathy"),
                    stats.empathy,
                    0xE060A0,
                )),
        )
        .child(
            div()
                .mt(px(14.0))
                .flex()
                .items_center()
                .justify_between()
                .gap(px(8.0))
                .child(
                    div()
                        .rounded_full()
                        .bg(color(theme::ACCENT).opacity(0.12))
                        .px(px(10.0))
                        .py(px(5.0))
                        .text_size(rems(0.75))
                        .line_height(rems(0.875))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(color(theme::ACCENT))
                        .child(pet_catalog_text(
                            language,
                            "pet.stage.companion",
                            "Companion",
                        )),
                )
                .when_some(claimed_at, |this, timestamp| {
                    this.child(
                        div()
                            .text_size(rems(0.75))
                            .line_height(rems(1.0))
                            .text_color(color(theme::TEXT_DIM))
                            .child(pet_date_label(timestamp)),
                    )
                }),
        )
        .child(
            div()
                .mt(px(12.0))
                .rounded(px(8.0))
                .bg(cx.theme().group_box)
                .px(px(12.0))
                .py(px(9.0))
                .flex()
                .items_center()
                .justify_between()
                .gap(px(8.0))
                .text_size(rems(0.75))
                .line_height(rems(1.0))
                .child(
                    div()
                        .text_color(color(theme::TEXT_MUTED))
                        .child(pet_catalog_text(language, "pet.total_xp", "Total XP")),
                )
                .child(
                    div()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(color(theme::TEXT))
                        .child(compact_number(total_xp)),
                ),
        )
}

fn pet_trait_bar(emoji: &'static str, label: String, value: i64, accent: u32) -> impl IntoElement {
    let ratio = (value as f32 / 330.0).clamp(0.0, 1.0);
    div()
        .flex()
        .items_center()
        .gap(px(6.0))
        .text_size(rems(0.75))
        .line_height(rems(1.0))
        .child(div().w(px(18.0)).child(emoji))
        .child(
            div()
                .w(px(34.0))
                .font_weight(FontWeight::MEDIUM)
                .text_color(color(theme::TEXT_MUTED))
                .child(label),
        )
        .child(
            div()
                .min_w(px(0.0))
                .flex_1()
                .h(px(5.0))
                .rounded_full()
                .overflow_hidden()
                .bg(color(accent).opacity(0.16))
                .child(
                    div()
                        .h_full()
                        .w(gpui::relative(ratio))
                        .rounded_full()
                        .bg(color(accent).opacity(0.75)),
                ),
        )
        .child(
            div()
                .w(px(34.0))
                .text_right()
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(color(theme::TEXT_DIM))
                .child(compact_number(value)),
        )
}

fn pet_dex_sidebar_overview(
    language: &str,
    current_name: String,
    current_level: String,
    archived_count: usize,
    archived_subtitle: String,
    unlocked_count: usize,
    total_count: usize,
    collection_subtitle: String,
    cx: &mut Context<CoduxApp>,
) -> impl IntoElement {
    div()
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(8.0))
                .child(
                    Icon::new(HeroIconName::BookOpen)
                        .size_3p5()
                        .text_color(color(theme::TEXT_MUTED)),
                )
                .child(
                    div()
                        .text_size(rems(1.0625))
                        .line_height(rems(1.375))
                        .font_weight(FontWeight::BOLD)
                        .child(pet_catalog_text(language, "pet.dex.title", "Pet Dex")),
                ),
        )
        .child(
            div()
                .mt(px(4.0))
                .text_size(rems(0.75))
                .line_height(rems(1.0))
                .text_color(color(theme::TEXT_MUTED))
                .child(pet_catalog_text(
                    language,
                    "pet.dex.subtitle",
                    "A record of every coding companion you've raised",
                )),
        )
        .child(pet_dex_separator())
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(8.0))
                .child(pet_dex_summary_row(
                    pet_catalog_text(language, "pet.dex.current_companion", "Current Companion"),
                    current_level,
                    current_name,
                    cx,
                ))
                .child(pet_dex_summary_row(
                    pet_catalog_text(language, "pet.dex.archived", "Archived"),
                    archived_subtitle.to_string(),
                    archived_count.to_string(),
                    cx,
                ))
                .child(pet_dex_summary_row(
                    pet_catalog_text(language, "pet.dex.collection", "Dex Collection"),
                    collection_subtitle.to_string(),
                    format!("{unlocked_count}/{}", total_count.max(1)),
                    cx,
                )),
        )
        .child(pet_dex_separator())
}

fn pet_dex_separator() -> impl IntoElement {
    div()
        .my(px(16.0))
        .h(px(1.0))
        .w_full()
        .bg(color(theme::BORDER_SOFT).opacity(0.75))
}

fn pet_dex_summary_row(
    label: String,
    subtitle: String,
    value: String,
    cx: &mut Context<CoduxApp>,
) -> impl IntoElement {
    div()
        .rounded(px(8.0))
        .bg(cx.theme().group_box)
        .px(px(12.0))
        .py(px(10.0))
        .flex()
        .items_center()
        .justify_between()
        .gap(px(10.0))
        .child(
            div()
                .min_w_0()
                .child(
                    div()
                        .text_size(rems(0.75))
                        .line_height(rems(1.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(color(theme::TEXT_MUTED))
                        .child(label),
                )
                .child(
                    div()
                        .mt(px(2.0))
                        .truncate()
                        .text_size(rems(0.75))
                        .line_height(rems(0.9375))
                        .text_color(color(theme::TEXT_DIM))
                        .child(subtitle),
                ),
        )
        .child(
            div()
                .max_w(px(96.0))
                .truncate()
                .text_right()
                .text_size(rems(0.875))
                .line_height(rems(1.125))
                .font_weight(FontWeight::BOLD)
                .child(value),
        )
}
fn pet_dex_spotlight_overlay(
    spotlight: PetDexSpotlight,
    catalog: &PetCatalog,
    runtime_asset_root: &Path,
    support_dir: &Path,
    language: &str,
    sprite_frame: usize,
    cx: &mut Context<CoduxApp>,
) -> impl IntoElement {
    if spotlight == PetDexSpotlight::ArchiveConfirm {
        return pet_dex_archive_confirm_overlay(language, cx);
    }

    let detail = match spotlight {
        PetDexSpotlight::Bundled(species) => catalog
            .species
            .iter()
            .find(|item| item.species == species)
            .map(|item| {
                let pet = PetSummary {
                    species: item.species.clone(),
                    ..PetSummary::default()
                };
                (
                    pet_catalog_text(language, &item.name_key, &pet_species_name(&item.species)),
                    pet_catalog_text(language, "pet.stage.companion", "Companion"),
                    pet_catalog_text(
                        language,
                        &item.description_key,
                        &pet_species_subtitle(&item.species),
                    ),
                    pet_sprite_path(runtime_asset_root, support_dir, &pet, &[]),
                    pet_accent_color(&item.species),
                )
            }),
        PetDexSpotlight::Custom(custom_id) => catalog
            .custom_pets
            .iter()
            .find(|pet| pet.id == custom_id)
            .map(|pet| {
                (
                    pet.display_name.clone(),
                    pet_catalog_text(language, "pet.custom.installed", "Custom pet"),
                    empty_label(&pet.description),
                    custom_pet_sprite_path(support_dir, pet).into(),
                    theme::ACCENT,
                )
            }),
        PetDexSpotlight::ArchiveConfirm => None,
    };

    let Some((title, subtitle, description, sprite_path, accent)) = detail else {
        return div().into_any_element();
    };

    div()
        .id("pet-dex-spotlight-overlay")
        .occlude()
        .absolute()
        .top(px(0.0))
        .right(px(0.0))
        .bottom(px(0.0))
        .left(px(0.0))
        .flex()
        .items_center()
        .justify_center()
        .bg(cx.theme().overlay)
        .p(px(24.0))
        .on_click(cx.listener(|app, _event, _window, cx| app.close_pet_dex_spotlight(cx)))
        .child(
            div()
                .id("pet-dex-spotlight-preview")
                .max_w(px(520.0))
                .flex()
                .flex_col()
                .items_center()
                .text_center()
                .child(
                    div()
                        .mx_auto()
                        .size(px(212.0))
                        .rounded_full()
                        .flex()
                        .items_center()
                        .justify_center()
                        .bg(color(accent).opacity(0.09))
                        .child(pet_sprite_element(
                            sprite_path,
                            168.0,
                            sprite_frame,
                            0,
                            cx.theme().primary,
                        )),
                )
                .child(
                    div()
                        .mt(px(20.0))
                        .text_size(rems(1.5))
                        .line_height(rems(1.875))
                        .font_weight(FontWeight::BOLD)
                        .text_color(color(theme::TEXT))
                        .child(title),
                )
                .child(
                    div()
                        .mt(px(8.0))
                        .text_size(rems(0.875))
                        .line_height(rems(1.125))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(color(accent))
                        .child(subtitle),
                )
                .when(!description.is_empty(), |this| {
                    this.child(
                        div()
                            .mt(px(18.0))
                            .max_w(px(420.0))
                            .text_size(rems(0.875))
                            .line_height(rems(1.375))
                            .text_color(color(theme::TEXT_MUTED))
                            .child(description),
                    )
                }),
        )
        .into_any_element()
}

fn pet_dex_archive_confirm_overlay(language: &str, cx: &mut Context<CoduxApp>) -> AnyElement {
    div()
        .id("pet-dex-archive-overlay")
        .occlude()
        .absolute()
        .top(px(0.0))
        .right(px(0.0))
        .bottom(px(0.0))
        .left(px(0.0))
        .flex()
        .items_center()
        .justify_center()
        .bg(cx.theme().overlay)
        .p(px(24.0))
        .on_click(cx.listener(|app, _event, _window, cx| app.close_pet_dex_spotlight(cx)))
        .child(
            div()
                .id("pet-dex-archive-card")
                .occlude()
                .w(px(360.0))
                .rounded(px(12.0))
                .border_1()
                .border_color(color(theme::BORDER_SOFT))
                .bg(color(theme::BG_PANEL))
                .p(px(20.0))
                .shadow_lg()
                .on_click(|_event, _window, cx| cx.stop_propagation())
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(
                            Icon::new(HeroIconName::Trash)
                                .size_4()
                                .text_color(color(theme::ORANGE)),
                        )
                        .child(
                            div()
                                .text_size(rems(1.0))
                                .line_height(rems(1.375))
                                .font_weight(FontWeight::BOLD)
                                .child(pet_catalog_text(
                                    &language,
                                    "pet.archive.alert.title",
                                    "Archive Current Pet",
                                )),
                        ),
                )
                .child(
                    div()
                        .mt(px(12.0))
                        .text_size(rems(0.75))
                        .line_height(rems(1.25))
                        .text_color(color(theme::TEXT_MUTED))
                        .child(pet_catalog_text(
                            &language,
                            "pet.archive.alert.message",
                            "Archive this pet into the dex and choose a new companion.",
                        )),
                )
                .child(
                    div()
                        .mt(px(20.0))
                        .flex()
                        .justify_end()
                        .gap_2()
                        .child(
                            dialog_cancel_button(
                                "pet-dex-cancel-archive",
                                pet_catalog_text(&language, "common.cancel", "Cancel"),
                                cx,
                                |app, _event, _window, cx| app.close_pet_dex_spotlight(cx),
                            )
                            .compact(),
                        )
                        .child(
                            dialog_primary_button(
                                "pet-dex-confirm-archive",
                                pet_catalog_text(
                                    &language,
                                    "pet.archive.confirm",
                                    "Confirm Archive",
                                ),
                                cx,
                                |app, _event, window, cx| {
                                    app.archive_current_pet_confirmed(window, cx)
                                },
                            )
                            .compact(),
                        ),
                ),
        )
        .into_any_element()
}

fn pet_section_header(label: String, trailing: Option<String>) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .justify_between()
        .gap(px(12.0))
        .child(
            div()
                .min_w_0()
                .truncate()
                .text_size(rems(1.0))
                .line_height(rems(1.25))
                .font_weight(FontWeight::BOLD)
                .child(label),
        )
        .when_some(trailing, |this, trailing| {
            this.child(
                div()
                    .flex_none()
                    .text_size(rems(0.75))
                    .line_height(rems(1.0))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(color(theme::TEXT_MUTED))
                    .child(trailing),
            )
        })
}
