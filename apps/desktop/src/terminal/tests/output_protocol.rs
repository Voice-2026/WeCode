use super::super::*;

#[test]
fn tracks_synchronized_output_across_chunks() {
    let mut depth = 0;
    let mut tail = Vec::new();

    assert_eq!(
        update_synchronized_output_state(b"\x1b[?202", &mut depth, &mut tail),
        SyncOutputUpdate::default()
    );
    assert_eq!(depth, 0);

    assert_eq!(
        update_synchronized_output_state(b"6hpartial frame", &mut depth, &mut tail),
        SyncOutputUpdate {
            entered_from_idle: true,
            exited_to_idle: false,
            should_notify: false,
            ended_at_sync_exit: false,
        }
    );
    assert_eq!(depth, 1);

    assert_eq!(
        update_synchronized_output_state(b"done\x1b[?2026l", &mut depth, &mut tail),
        SyncOutputUpdate {
            entered_from_idle: false,
            exited_to_idle: true,
            should_notify: true,
            ended_at_sync_exit: true,
        }
    );
    assert_eq!(depth, 0);
}
#[test]
fn reports_notify_when_synchronized_output_ends() {
    let mut depth = 0;
    let mut tail = Vec::new();

    assert_eq!(
        update_synchronized_output_state(b"\x1b[?2026hframe\x1b[?2026l", &mut depth, &mut tail),
        SyncOutputUpdate {
            entered_from_idle: true,
            exited_to_idle: true,
            should_notify: true,
            ended_at_sync_exit: true,
        }
    );
    assert_eq!(depth, 0);

    // Bytes after the frame commit mean the cursor correction already arrived.
    assert_eq!(
        update_synchronized_output_state(
            b"\x1b[?2026hframe\x1b[?2026l\x1b[23;3H\x1b[?25h",
            &mut depth,
            &mut tail,
        ),
        SyncOutputUpdate {
            entered_from_idle: true,
            exited_to_idle: true,
            should_notify: true,
            ended_at_sync_exit: false,
        }
    );
    assert_eq!(depth, 0);
}
#[test]
fn tracks_nested_synchronized_output() {
    let mut depth = 0;
    let mut tail = Vec::new();

    assert_eq!(
        update_synchronized_output_state(
            b"\x1b[?2026houter\x1b[?2026hinner",
            &mut depth,
            &mut tail,
        ),
        SyncOutputUpdate {
            entered_from_idle: true,
            exited_to_idle: false,
            should_notify: false,
            ended_at_sync_exit: false,
        }
    );
    assert_eq!(depth, 2);

    assert_eq!(
        update_synchronized_output_state(b"\x1b[?2026l", &mut depth, &mut tail),
        SyncOutputUpdate {
            entered_from_idle: false,
            exited_to_idle: false,
            should_notify: true,
            ended_at_sync_exit: false,
        }
    );
    assert_eq!(depth, 1);

    assert_eq!(
        update_synchronized_output_state(b"\x1b[?2026l", &mut depth, &mut tail),
        SyncOutputUpdate {
            entered_from_idle: false,
            exited_to_idle: true,
            should_notify: true,
            ended_at_sync_exit: true,
        }
    );
    assert_eq!(depth, 0);
}
#[test]
fn protocol_flags_detect_cursor_and_color_requests() {
    assert_eq!(
        terminal_protocol_flags(b"\x1b[?25lhello\x1b[?25h\x1b]10;?\x07\x1b]11;?\x07"),
        TerminalProtocolFlags {
            show_cursor: true,
            hide_cursor: true,
            osc_10_request: true,
            osc_11_request: true,
        }
    );
}
#[test]
fn color_scheme_protocol_tracks_subscription_and_queries_across_chunks() {
    let mut state = TerminalColorSchemeState::default();

    assert_eq!(
        update_terminal_color_scheme_state(b"\x1b[?203", &mut state),
        TerminalColorSchemeUpdate::default()
    );
    assert!(!state.updates_enabled);

    assert_eq!(
        update_terminal_color_scheme_state(b"1h\x1b[?996n", &mut state),
        TerminalColorSchemeUpdate {
            enabled: true,
            disabled: false,
            query_count: 1,
            ..TerminalColorSchemeUpdate::default()
        }
    );
    assert!(state.updates_enabled);

    assert_eq!(
        update_terminal_color_scheme_state(b"\x1b[?2031l", &mut state),
        TerminalColorSchemeUpdate {
            enabled: false,
            disabled: true,
            query_count: 0,
            ..TerminalColorSchemeUpdate::default()
        }
    );
    assert!(!state.updates_enabled);
}
#[test]
fn osc_notifications_parse_titles_chunks_and_skip_progress() {
    let mut tail = Vec::new();

    // OSC 9 body, OSC 777 title;body, ConEmu progress filtered out.
    let found = scan_terminal_osc_notifications(
        b"\x1b]9;done building\x07\x1b]777;notify;Build;finished ok\x1b\\\x1b]9;4;1;50\x07",
        &mut tail,
    );
    assert_eq!(found.len(), 2);
    assert_eq!(found[0].title, None);
    assert_eq!(found[0].body, "done building");
    assert_eq!(found[1].title.as_deref(), Some("Build"));
    assert_eq!(found[1].body, "finished ok");

    // A sequence split across reads is carried in the tail and reported once.
    let first = scan_terminal_osc_notifications(b"\x1b]9;par", &mut tail);
    assert!(first.is_empty());
    let second = scan_terminal_osc_notifications(b"tial\x07", &mut tail);
    assert_eq!(second.len(), 1);
    assert_eq!(second[0].body, "partial");
    let third = scan_terminal_osc_notifications(b"no sequences here", &mut tail);
    assert!(third.is_empty());
}
#[test]
fn osc_color_queries_tracked_across_chunks() {
    let mut state = TerminalColorSchemeState::default();

    assert_eq!(
        update_terminal_color_scheme_state(b"\x1b]1", &mut state),
        TerminalColorSchemeUpdate::default()
    );
    assert_eq!(
        update_terminal_color_scheme_state(b"1;?\x07\x1b]10;?\x1b\\", &mut state),
        TerminalColorSchemeUpdate {
            osc_foreground_queries: 1,
            osc_background_queries: 1,
            ..TerminalColorSchemeUpdate::default()
        }
    );
}
#[test]
fn osc_color_queries_reply_with_palette_colors() {
    let mut state = TerminalModel::new_for_test(10, 4, 100);
    state.colors = ColorPalette::builder()
        .background(0x1e, 0x22, 0x2b)
        .foreground(0xee, 0xee, 0xee)
        .build();

    state.respond_to_osc_color_queries(&TerminalColorSchemeUpdate {
        osc_foreground_queries: 1,
        osc_background_queries: 1,
        ..TerminalColorSchemeUpdate::default()
    });

    let written = String::from_utf8(state.written_bytes_for_test()).unwrap();
    assert!(written.contains("\x1b]10;rgb:eeee/eeee/eeee\x07"));
    assert!(written.contains("\x1b]11;rgb:1e1e/2222/2b2b\x07"));
}
#[test]
fn color_scheme_report_matches_xterm_codes() {
    assert_eq!(
        terminal_color_scheme_report_for(ColorPalette::default().is_dark()),
        b"\x1b[?997;1n"
    );

    let light = ColorPalette::builder()
        .background(0xee, 0xee, 0xee)
        .foreground(0x11, 0x11, 0x11)
        .build();
    assert_eq!(
        terminal_color_scheme_report_for(light.is_dark()),
        b"\x1b[?997;2n"
    );
}
#[test]
fn color_scheme_queries_write_current_scheme() {
    let mut state = TerminalModel::new_for_test(10, 4, 100);
    state.colors = ColorPalette::builder()
        .background(0xee, 0xee, 0xee)
        .foreground(0x11, 0x11, 0x11)
        .build();

    state.respond_to_color_scheme_queries(2);

    assert_eq!(state.written_bytes_for_test(), b"\x1b[?997;2n\x1b[?997;2n");
}
// Drive one output chunk through the sync scanner and a snapshot publish,
// the way process_output_bytes + the async publish do in production.
fn feed_and_publish(state: &mut TerminalModel, bytes: &[u8]) {
    let update = state.update_synchronized_output_state(bytes);
    state.note_output_sync_boundary(update);
    state.process_bytes(bytes);
    let snapshot = state.handle.screen.lock().snapshot();
    state.publish_completed_snapshot(snapshot, Duration::ZERO);
}

#[test]
fn conpty_frame_commit_keeps_previous_cursor_until_correction() {
    let mut state = TerminalModel::new_for_test(80, 25, 100);
    state.conpty_output = true;

    // Settled state: prompt drawn, cursor at row 23 col 3 (1-based).
    feed_and_publish(&mut state, b"codex\x1b[23;3H");
    let settled = state.handle.snapshot().cursor;
    assert_eq!((settled.row, settled.col, settled.visible), (22, 2, true));

    // ConPTY frame commit: the chunk ends at 2026l with the cursor parked at
    // its repaint scan position (25;31). The publish must keep the settled
    // cursor instead of rendering the transient one.
    feed_and_publish(
        &mut state,
        b"\x1b[?2026h\x1b[?25l\x1b[19;2H\x1b[Kworking\x1b[25;31H\x1b[K\x1b[?25h\x1b[?2026l",
    );
    let held = state.handle.snapshot().cursor;
    assert_eq!((held.row, held.col, held.visible), (22, 2, true));

    // The follow-up correction restores the real cursor and publishes it.
    feed_and_publish(&mut state, b"\x1b[?25l\x1b[24;6H\x1b[?25h");
    let corrected = state.handle.snapshot().cursor;
    assert_eq!((corrected.row, corrected.col, corrected.visible), (23, 5, true));
}

#[test]
fn frame_commit_cursor_publishes_unchanged_without_conpty() {
    let mut state = TerminalModel::new_for_test(80, 25, 100);

    feed_and_publish(&mut state, b"codex\x1b[23;3H");
    // Same commit-terminated chunk: a native emitter (mac codex) ends its
    // frame with the cursor already at the right spot, so it must publish.
    feed_and_publish(
        &mut state,
        b"\x1b[?2026h\x1b[?25l\x1b[19;2H\x1b[Kworking\x1b[25;31H\x1b[K\x1b[?25h\x1b[?2026l",
    );
    let cursor = state.handle.snapshot().cursor;
    assert_eq!((cursor.row, cursor.col, cursor.visible), (24, 30, true));
}

#[test]
fn color_scheme_update_reports_theme_change_when_subscribed() {
    let mut state = TerminalModel::new_for_test(10, 4, 100);
    state.color_scheme_state.updates_enabled = true;

    state.update_colors(
        ColorPalette::builder()
            .background(0xee, 0xee, 0xee)
            .foreground(0x11, 0x11, 0x11)
            .build(),
    );
    assert_eq!(state.written_bytes_for_test(), b"\x1b[?997;2n");

    state.update_colors(
        ColorPalette::builder()
            .background(0xdd, 0xdd, 0xdd)
            .foreground(0x22, 0x22, 0x22)
            .build(),
    );
    assert_eq!(state.written_bytes_for_test(), b"\x1b[?997;2n");

    state.update_colors(ColorPalette::default());
    assert_eq!(state.written_bytes_for_test(), b"\x1b[?997;2n\x1b[?997;1n");
}
