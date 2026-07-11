use super::*;

impl WeCodeApp {
    /// Full-screen level-up celebration: dark scrim, halo rings, confetti,
    /// the pet popping in, and the new level. Click anywhere dismisses.
    pub(in crate::app) fn pet_level_up_overlay(
        &self,
        fx: &PetLevelUpFx,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        const STAGE: f32 = 380.0;
        let progress = fx.progress.clamp(0.0, 1.0);
        let fade_in = (progress / 0.10).min(1.0);
        let fade_out = 1.0 - ((progress - 0.86) / 0.14).clamp(0.0, 1.0);
        let alpha = fade_in * fade_out;
        let pop = pet_ease_out_back((progress / 0.22).min(1.0));
        let sprite_size = (64.0 + 56.0 * pop).max(8.0);
        let rise = (1.0 - pet_ease_out_cubic((progress / 0.30).min(1.0))) * 16.0;
        let accent = cx.theme().primary;
        let sprite_path = pet_sprite_path(
            &self.runtime.source_root,
            &self.state.support_dir,
            &self.state.pet,
            &self.pet_custom_pets,
        );
        let sprite_frame = self.visible_pet_sprite_frame(PET_IDLE_FRAME_COUNT);
        let title = pet_catalog_text(
            &self.state.settings.language,
            "pet.level_up.title",
            "Level Up!",
        );

        let mut stage = div()
            .relative()
            .size(px(STAGE))
            .flex()
            .items_center()
            .justify_center();
        for (index, delay) in [0.06_f32, 0.20].into_iter().enumerate() {
            let ring_progress = ((progress - delay) / 0.55).clamp(0.0, 1.0);
            let eased = pet_ease_out_cubic(ring_progress);
            let diameter = 96.0 + (STAGE - 90.0 - 96.0) * eased;
            let ring_alpha = (1.0 - eased) * if index == 0 { 0.55 } else { 0.35 } * fade_out;
            stage = stage.child(
                div()
                    .absolute()
                    .left(px((STAGE - diameter) / 2.0))
                    .top(px((STAGE - diameter) / 2.0))
                    .size(px(diameter))
                    .rounded_full()
                    .border_2()
                    .border_color(accent.opacity(ring_alpha)),
            );
        }
        let palette = [accent, color(theme::GREEN), color(theme::ORANGE)];
        let particle_eased = pet_ease_out_cubic(((progress - 0.10) / 0.62).clamp(0.0, 1.0));
        for index in 0..14_usize {
            let angle =
                (index as f32) * (std::f32::consts::TAU / 14.0) + ((index * 37) % 17) as f32 * 0.05;
            let distance = 44.0 + (118.0 + ((index * 29) % 26) as f32) * particle_eased;
            let size = 5.0 + ((index % 3) as f32) * 2.0;
            let dot_alpha = (1.0 - particle_eased) * fade_out;
            stage = stage.child(
                div()
                    .absolute()
                    .left(px(STAGE / 2.0 + distance * angle.cos() - size / 2.0))
                    .top(px(STAGE / 2.0 + distance * angle.sin() - size / 2.0))
                    .size(px(size))
                    .rounded_full()
                    .bg(palette[index % palette.len()].opacity(dot_alpha)),
            );
        }
        stage = stage.child(
            div()
                .flex()
                .flex_col()
                .items_center()
                .mt(px(rise))
                .child(
                    div()
                        .size(px(150.0))
                        .rounded_full()
                        .flex()
                        .items_center()
                        .justify_center()
                        .bg(accent.opacity(0.20 * alpha))
                        .child(pet_sprite_element(
                            sprite_path,
                            sprite_size,
                            sprite_frame,
                            0,
                            accent,
                        )),
                )
                .child(
                    div()
                        .mt(px(16.0))
                        .text_size(rems(0.75))
                        .line_height(rems(1.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(accent.opacity(alpha))
                        .child(title),
                )
                .child(
                    div()
                        .mt(px(6.0))
                        .text_size(rems(1.9))
                        .line_height(rems(2.1))
                        .font_weight(FontWeight::BOLD)
                        .font_family(cx.theme().mono_font_family.clone())
                        .text_color(theme::fixed_color(0xF3F6FC).opacity(alpha))
                        .child(format!("Lv.{}", fx.level)),
                ),
        );

        div()
            .absolute()
            .inset_0()
            .occlude()
            .flex()
            .items_center()
            .justify_center()
            .bg(theme::fixed_color(0x05070C).opacity(0.58 * alpha))
            .on_mouse_down(
                gpui::MouseButton::Left,
                cx.listener(|app, _event, _window, cx| {
                    app.pet_level_up = None;
                    cx.notify();
                }),
            )
            .child(stage)
            .into_any_element()
    }
}

fn pet_ease_out_cubic(t: f32) -> f32 {
    1.0 - (1.0 - t).powi(3)
}

fn pet_ease_out_back(t: f32) -> f32 {
    let c1 = 1.70158_f32;
    let c3 = c1 + 1.0;
    1.0 + c3 * (t - 1.0).powi(3) + c1 * (t - 1.0).powi(2)
}
