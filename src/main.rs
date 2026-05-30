mod app;
mod assets;
mod terminal;
mod theme;

use anyhow::Result;
use app::CoduxApp;
use assets::CoduxAssets;
use gpui::{
    AnyWindowHandle, App, AppContext, Bounds, KeyBinding, Unbind, WindowBounds, WindowOptions, px,
    size,
};
use gpui_component::Root;
use std::cell::Cell;
use std::rc::Rc;

fn main() -> Result<()> {
    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
    disable_macos_autofill_heuristics();

    let app = gpui_platform::application().with_assets(CoduxAssets);
    let main_window_handle: Rc<Cell<Option<AnyWindowHandle>>> = Rc::new(Cell::new(None));
    let reopen_main_window = main_window_handle.clone();
    app.on_reopen(move |cx| {
        if let Some(handle) = reopen_main_window.get() {
            if handle
                .update(cx, |_view, window, _cx| window.activate_window())
                .is_ok()
            {
                cx.activate(true);
                return;
            }
            reopen_main_window.set(None);
        }

        if open_main_window(cx, &reopen_main_window) {
            cx.activate(true);
        }
    });

    app.run(move |cx: &mut App| {
        app::macos_window::install_dock_reopen_handler();
        gpui_component::init(cx);
        theme::apply_component_theme_for_name("GitHub Dark", None, cx);
        disable_root_tab_focus_bindings(cx);
        cx.on_action(|_: &crate::app::native_menu::QuitCodux, cx| cx.quit());
        let initial_state = codux_runtime::runtime_state::RuntimeState::load();
        cx.set_menus(crate::app::native_menu::codux_menus(
            &initial_state.settings.language,
        ));
        if !open_main_window(cx, &main_window_handle) {
            cx.quit();
            return;
        }

        cx.activate(true);
    });

    Ok(())
}

fn open_main_window(cx: &mut App, main_window_handle: &Rc<Cell<Option<AnyWindowHandle>>>) -> bool {
    let bounds = Bounds::centered(None, size(px(1280.0), px(820.0)), cx);
    let result = cx.open_window(
        WindowOptions {
            titlebar: Some(theme::codux_titlebar("Codux GPUI")),
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

    match result {
        Ok(handle) => {
            main_window_handle.set(Some(handle.into()));
            true
        }
        Err(error) => {
            eprintln!("failed to open Codux GPUI window: {error}");
            false
        }
    }
}

#[cfg(target_os = "macos")]
fn disable_macos_autofill_heuristics() {
    use core_foundation_sys::base::{CFRelease, kCFAllocatorDefault};
    use core_foundation_sys::number::kCFBooleanFalse;
    use core_foundation_sys::preferences::{
        CFPreferencesAppSynchronize, CFPreferencesSetAppValue, kCFPreferencesCurrentApplication,
    };
    use core_foundation_sys::string::{CFStringCreateWithCString, kCFStringEncodingUTF8};
    use std::ffi::CString;

    let key = CString::new("NSAutoFillHeuristicControllerEnabled")
        .expect("static string contains no nul");
    let key_ref = unsafe {
        CFStringCreateWithCString(kCFAllocatorDefault, key.as_ptr(), kCFStringEncodingUTF8)
    };
    if key_ref.is_null() {
        return;
    }

    unsafe {
        CFPreferencesSetAppValue(
            key_ref,
            kCFBooleanFalse.cast(),
            kCFPreferencesCurrentApplication,
        );
        let _ = CFPreferencesAppSynchronize(kCFPreferencesCurrentApplication);
        CFRelease(key_ref.cast());
    }
}

#[cfg(not(target_os = "macos"))]
fn disable_macos_autofill_heuristics() {}

fn disable_root_tab_focus_bindings(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("tab", Unbind("root::Tab".into()), Some("Root")),
        KeyBinding::new("shift-tab", Unbind("root::TabPrev".into()), Some("Root")),
        KeyBinding::new("cmd-w", crate::app::native_menu::CloseWindow, None),
        KeyBinding::new("ctrl-w", crate::app::native_menu::CloseWindow, None),
    ]);
}
