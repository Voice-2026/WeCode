use super::super::*;

#[test]
fn alternate_scroll_honors_application_cursor_mode() {
    // Normal-cursor apps (Claude's pager) need CSI arrows; only
    // application-cursor mode wants SS3 — sending SS3 to a normal-mode app
    // is why its wheel scroll did nothing.
    assert_eq!(alternate_scroll_sequence(true, false), b"\x1b[A");
    assert_eq!(alternate_scroll_sequence(false, false), b"\x1b[B");
    assert_eq!(alternate_scroll_sequence(true, true), b"\x1bOA");
    assert_eq!(alternate_scroll_sequence(false, true), b"\x1bOB");
}
#[test]
fn absolute_scroll_targets_do_not_compound_when_published_offset_lags() {
    let mut state = TerminalModel::new_for_test(20, 4, 100);
    state.process_output_bytes_for_test(b"one\r\ntwo\r\nthree\r\nfour\r\nfive\r\nsix\r\nseven");

    // Two drag frames target the same offset while no publish has
    // landed in between; the engine must end exactly at the target.
    state.scroll_to_display_offset(2);
    state.publish_snapshot_now();
    state.scroll_to_display_offset(2);
    let content = state.publish_snapshot_now();

    assert_eq!(content.display_offset, 2);
}
#[test]
fn input_viewport_republishes_when_publish_is_in_flight() {
    let mut state = TerminalModel::new_for_test(20, 4, 100);
    state.process_output_bytes_for_test(b"one\r\ntwo\r\nthree\r\nfour\r\nfive\r\nsix");
    state.handle.publish_snapshot();
    state.snapshot_dirty = false;

    // A publish is in flight: the published offset can't be trusted.
    state.snapshot_publish_pending = true;
    assert!(state.prepare_input_viewport_snapshot());
    assert!(state.snapshot_dirty);
}
#[test]
fn paste_uses_live_bracketed_paste_mode_before_snapshot_publish() {
    let mut state = TerminalModel::new_for_test(20, 4, 100);
    // Enable bracketed paste in the engine without publishing the
    // snapshot: the published input_mode is still stale.
    state.process_output_bytes_for_test(b"\x1b[?2004h");
    assert!(!state.handle.input_mode().bracketed_paste);

    state.paste_text("line1\nline2");

    let written = state.written_bytes_for_test();
    let text = String::from_utf8_lossy(&written);
    assert!(
        text.starts_with("\x1b[200~") && text.ends_with("\x1b[201~"),
        "paste was not bracketed: {text:?}"
    );
}
#[test]
fn display_cursor_tracks_ghostty_viewport_coordinates() {
    let mut state = TerminalModel::new_for_test(10, 4, 100);
    state.process_bytes(b"one\r\ntwo\r\nthree\r\nfour\r\nfive\r\nsix\r\nseven");
    state.handle.publish_snapshot();
    assert!(state.scroll_display(2));
    let snapshot = state.sync_for_test();

    let display_cursor = snapshot.display_cursor();

    assert_eq!(snapshot.display_offset, 2);
    assert_eq!(
        display_cursor,
        DisplayCursor {
            row: snapshot.cursor.row as i32,
            col: snapshot.cursor.col,
        }
    );
}
#[test]
fn local_visible_rows_map_bottom_slice_without_screen_gaps() {
    let mut state = TerminalModel::new_for_test(10, 6, 100);
    state.process_bytes(b"one\r\ntwo\r\nthree\r\nfour\r\nfive\r\nsix");
    state.handle.publish_snapshot();

    let snapshot = state.sync_for_test().with_visible_row_shift(4);

    assert_eq!(snapshot.screen_lines, 6);
    assert_eq!(snapshot.visible_rows(), 4);
    assert_eq!(snapshot.visible_row_shift, 2);
    assert_eq!(snapshot.display_row_for_line(0), None);
    assert_eq!(snapshot.display_row_for_line(1), None);
    assert_eq!(snapshot.display_row_for_line(2), Some(0));
    assert_eq!(snapshot.display_row_for_line(3), Some(1));
    assert_eq!(snapshot.display_row_for_line(4), Some(2));
    assert_eq!(snapshot.display_row_for_line(5), Some(3));
    assert_eq!(snapshot.line_for_display_row(0), 2);
    assert_eq!(snapshot.line_for_display_row(3), 5);
}
#[test]
fn scroll_to_bottom_restores_input_viewport() {
    let mut state = TerminalModel::new_for_test(10, 4, 100);
    state.process_bytes(b"one\r\ntwo\r\nthree\r\nfour\r\nfive\r\nsix\r\nseven");
    state.handle.publish_snapshot();
    assert!(state.scroll_display(2));
    let scrolled = state.sync_for_test();
    assert_eq!(scrolled.display_offset, 2);
    assert!(!scrolled.scrolled_to_bottom);

    state.prepare_input_viewport_for_test();
    let bottom = state.live_snapshot();

    assert_eq!(bottom.display_offset, 0);
    assert!(bottom.scrolled_to_bottom);
}
#[test]
fn remote_viewport_event_adopts_remote_grid_size() {
    let mut state = TerminalModel::new_for_test(10, 4, 100);

    // A remote client owning the viewport sizes the PTY to its own grid;
    // the desktop model adopts that grid so the running TUI's repaint
    // renders at the size it was drawn for, rather than being misplaced in
    // the larger desktop grid.
    assert!(state.apply_ui_event(TerminalUiEvent::Viewport {
        remote_owner: true,
        generation: 1,
        cols: 40,
        rows: 30,
    }));
    assert_eq!(state.dimensions(), (40, 30));
    assert!(state.remote_viewer);
}
#[test]
fn input_viewport_preparation_discards_pending_history_scroll() {
    let mut state = TerminalModel::new_for_test(10, 4, 100);
    state.process_bytes(b"one\r\ntwo\r\nthree\r\nfour\r\nfive\r\nsix\r\nseven");
    state.handle.publish_snapshot();
    assert!(state.scroll_display(2));
    assert_eq!(state.sync_for_test().display_offset, 2);

    assert!(state.scroll_display(1));
    state.prepare_input_viewport_for_test();
    let snapshot = state.sync_for_test();

    assert_eq!(snapshot.display_offset, 0);
    assert!(snapshot.scrolled_to_bottom);
}
#[test]
fn input_viewport_preparation_keeps_bottom_stable_across_drift_events() {
    let mut state = TerminalModel::new_for_test(10, 4, 100);
    state.process_bytes(b"one\r\ntwo\r\nthree\r\nfour\r\nfive\r\nsix\r\nseven\r\neight");
    state.handle.publish_snapshot();

    for lines in [2, -1, 3, 1] {
        assert!(state.scroll_display(lines));
    }
    state.prepare_input_viewport_for_test();
    let snapshot = state.sync_for_test();

    assert_eq!(snapshot.display_offset, 0);
    assert!(snapshot.scrolled_to_bottom);
}
#[test]
fn keyboard_input_suppresses_residual_precise_scroll_from_same_gesture() {
    let mut state = TerminalScrollInputState {
        pending_lines: 3,
        pending_pixels: 12.0,
        frame_pending: false,
        suppress_residual_precise_scroll: false,
    };
    state.prepare_for_keyboard_input();

    assert_eq!(state.pending_lines, 0);
    assert_eq!(state.pending_pixels, 0.0);
    assert!(state.should_suppress_residual_scroll(&ScrollWheelEvent {
        delta: gpui::ScrollDelta::Pixels(Point {
            x: px(0.0),
            y: px(8.0),
        }),
        touch_phase: TouchPhase::Moved,
        ..Default::default()
    }));
    assert_eq!(state.pending_pixels, 0.0);
}
#[test]
fn new_scroll_gesture_after_keyboard_input_is_not_suppressed() {
    let mut state = TerminalScrollInputState::default();
    state.prepare_for_keyboard_input();

    assert!(!state.should_suppress_residual_scroll(&ScrollWheelEvent {
        delta: gpui::ScrollDelta::Pixels(Point {
            x: px(0.0),
            y: px(8.0),
        }),
        touch_phase: TouchPhase::Started,
        ..Default::default()
    }));
    assert!(!state.should_suppress_residual_scroll(&ScrollWheelEvent {
        delta: gpui::ScrollDelta::Pixels(Point {
            x: px(0.0),
            y: px(8.0),
        }),
        touch_phase: TouchPhase::Moved,
        ..Default::default()
    }));
}
#[test]
fn keyboard_input_does_not_suppress_line_wheel_scroll() {
    let mut state = TerminalScrollInputState::default();
    state.prepare_for_keyboard_input();

    assert!(!state.should_suppress_residual_scroll(&ScrollWheelEvent {
        delta: gpui::ScrollDelta::Lines(Point { x: 0.0, y: 1.0 }),
        touch_phase: TouchPhase::Moved,
        ..Default::default()
    }));
}
