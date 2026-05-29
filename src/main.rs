mod app;
mod terminal;
mod theme;

use anyhow::Result;
use app::CoduxApp;
use gpui::{App, AppContext, Bounds, KeyBinding, Unbind, WindowBounds, WindowOptions, px, size};
use gpui_component::Root;
use gpui_component_assets::Assets;

fn main() -> Result<()> {
    let app = gpui_platform::application().with_assets(Assets);

    app.run(|cx: &mut App| {
        gpui_component::init(cx);
        theme::apply_component_theme_for_name("GitHub Dark", None, cx);
        disable_root_tab_focus_bindings(cx);
        cx.on_action(|_: &crate::app::native_menu::QuitCodux, cx| cx.quit());
        let initial_state = codux_runtime::runtime_state::RuntimeState::load();
        cx.set_menus(crate::app::native_menu::codux_menus(
            &initial_state.settings.language,
        ));
        let bounds = Bounds::centered(None, size(px(1280.0), px(820.0)), cx);

        let result = cx.open_window(
            WindowOptions {
                titlebar: Some(gpui::TitlebarOptions {
                    title: Some("Codux GPUI".into()),
                    appears_transparent: true,
                    ..Default::default()
                }),
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                window_min_size: Some(size(px(960.0), px(640.0))),
                ..Default::default()
            },
            |window, cx| {
                let app = CoduxApp::new(window, cx).expect("failed to create Codux GPUI app");
                let view = cx.new(|_| app);
                view.update(cx, |app, cx| app.start_runtime_event_loop(cx));
                cx.new(|cx| Root::new(view, window, cx))
            },
        );

        if let Err(error) = result {
            eprintln!("failed to open Codux GPUI window: {error}");
            cx.quit();
            return;
        }

        cx.activate(true);
    });

    Ok(())
}

fn disable_root_tab_focus_bindings(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("tab", Unbind("root::Tab".into()), Some("Root")),
        KeyBinding::new("shift-tab", Unbind("root::TabPrev".into()), Some("Root")),
    ]);
}
