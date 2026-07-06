use super::super::*;

#[test]
fn ime_cursor_bounds_follow_current_viewport_after_history_scroll() {
    let mut layout = TerminalLayoutMetrics::default();
    layout.update(
        Bounds {
            origin: Point {
                x: px(10.0),
                y: px(20.0),
            },
            size: Size {
                width: px(100.0),
                height: px(80.0),
            },
        },
        Edges {
            top: px(2.0),
            right: px(3.0),
            bottom: px(4.0),
            left: px(5.0),
        },
        px(10.0),
        px(20.0),
        10,
        4,
    );

    let mut state = TerminalModel::new_for_test(10, 4, 100);
    state.process_bytes(b"one\r\ntwo\r\nthree\r\nfour\r\nfive\r\nsix\r\nseven");
    state.handle.publish_snapshot();

    assert!(state.scroll_display(2));
    let scrolled = state.sync_for_test();
    assert_eq!(scrolled.display_offset, 2);
    assert!(state.current_ime_cursor_bounds(&layout).is_none());

    state.prepare_input_viewport_for_test();
    let bottom = state.sync_for_test();
    let bounds = state.current_ime_cursor_bounds(&layout).unwrap();
    let row = bottom.display_cursor().row;

    assert_eq!(bottom.display_offset, 0);
    assert!(bottom.scrolled_to_bottom);
    assert!(row >= 0);
    assert_eq!(
        bounds.origin.x,
        px(15.0) + px(10.0) * bottom.cursor.col as f32
    );
    assert_eq!(bounds.origin.y, px(22.0) + px(20.0) * row as f32);
    assert_eq!(bounds.size.width, px(10.0));
    assert_eq!(bounds.size.height, px(20.0));
}
#[test]
fn ime_bounds_for_range_offsets_from_current_cursor_cell() {
    let mut layout = TerminalLayoutMetrics::default();
    layout.update(
        Bounds {
            origin: Point {
                x: px(10.0),
                y: px(20.0),
            },
            size: Size {
                width: px(100.0),
                height: px(80.0),
            },
        },
        Edges {
            top: px(2.0),
            right: px(0.0),
            bottom: px(0.0),
            left: px(5.0),
        },
        px(10.0),
        px(20.0),
        10,
        4,
    );
    let cursor = Bounds {
        origin: Point {
            x: px(25.0),
            y: px(42.0),
        },
        size: Size {
            width: px(10.0),
            height: px(20.0),
        },
    };

    let bounds = ime_bounds_for_range(Some(cursor), &layout, 2..4).unwrap();

    assert_eq!(bounds.origin.x, px(45.0));
    assert_eq!(bounds.origin.y, px(42.0));
}
#[test]
fn ime_bounds_fall_back_to_first_cell_when_terminal_has_no_cursor_rect() {
    let mut layout = TerminalLayoutMetrics::default();
    layout.update(
        Bounds {
            origin: Point {
                x: px(10.0),
                y: px(20.0),
            },
            size: Size {
                width: px(100.0),
                height: px(80.0),
            },
        },
        Edges {
            top: px(2.0),
            right: px(3.0),
            bottom: px(4.0),
            left: px(5.0),
        },
        px(10.0),
        px(20.0),
        10,
        4,
    );

    let bounds = ime_bounds_for_range(layout.first_cell_ime_bounds(), &layout, 0..0).unwrap();

    assert_eq!(bounds.origin.x, px(15.0));
    assert_eq!(bounds.origin.y, px(22.0));
    assert_eq!(bounds.size.width, px(10.0));
    assert_eq!(bounds.size.height, px(20.0));
}
#[test]
fn ime_bounds_reuse_last_valid_cursor_when_current_cursor_is_missing() {
    let mut layout = TerminalLayoutMetrics::default();
    layout.update(
        Bounds {
            origin: Point {
                x: px(10.0),
                y: px(20.0),
            },
            size: Size {
                width: px(100.0),
                height: px(80.0),
            },
        },
        Edges::all(px(0.0)),
        px(10.0),
        px(20.0),
        10,
        4,
    );
    let cursor = Bounds {
        origin: Point {
            x: px(30.0),
            y: px(60.0),
        },
        size: Size {
            width: px(10.0),
            height: px(20.0),
        },
    };
    layout.record_ime_cursor_bounds(Some(cursor));
    layout.record_ime_cursor_bounds(None);

    let bounds = ime_bounds_for_range(layout.last_ime_cursor_bounds(), &layout, 1..1).unwrap();

    assert_eq!(bounds.origin.x, px(40.0));
    assert_eq!(bounds.origin.y, px(60.0));
}
#[test]
fn ime_bounds_keep_last_cursor_through_degenerate_reflow_bounds() {
    let mut layout = TerminalLayoutMetrics::default();
    let valid = Bounds {
        origin: Point {
            x: px(10.0),
            y: px(20.0),
        },
        size: Size {
            width: px(100.0),
            height: px(80.0),
        },
    };
    layout.update(valid, Edges::all(px(0.0)), px(10.0), px(20.0), 10, 4);
    let cursor = Bounds {
        origin: Point {
            x: px(30.0),
            y: px(60.0),
        },
        size: Size {
            width: px(10.0),
            height: px(20.0),
        },
    };
    layout.record_ime_cursor_bounds(Some(cursor));

    // A layout sync paints one frame with zero-sized bounds; the cached
    // cursor must survive it instead of falling back to the screen corner.
    layout.update(
        Bounds {
            origin: valid.origin,
            size: Size {
                width: px(0.0),
                height: px(0.0),
            },
        },
        Edges::all(px(0.0)),
        px(10.0),
        px(20.0),
        10,
        4,
    );

    assert_eq!(layout.first_cell_ime_bounds(), None);
    assert_eq!(layout.last_ime_cursor_bounds(), Some(cursor));
}
#[test]
fn ime_bounds_ignore_cached_cursor_outside_current_terminal_bounds() {
    let mut layout = TerminalLayoutMetrics::default();
    layout.update(
        Bounds {
            origin: Point {
                x: px(10.0),
                y: px(20.0),
            },
            size: Size {
                width: px(100.0),
                height: px(80.0),
            },
        },
        Edges {
            top: px(2.0),
            right: px(0.0),
            bottom: px(0.0),
            left: px(5.0),
        },
        px(10.0),
        px(20.0),
        10,
        4,
    );
    layout.record_ime_cursor_bounds(Some(Bounds {
        origin: Point {
            x: px(500.0),
            y: px(600.0),
        },
        size: Size {
            width: px(10.0),
            height: px(20.0),
        },
    }));

    assert!(layout.last_ime_cursor_bounds().is_none());
    let bounds = ime_bounds_for_range(layout.first_cell_ime_bounds(), &layout, 0..0).unwrap();

    assert_eq!(bounds.origin.x, px(15.0));
    assert_eq!(bounds.origin.y, px(22.0));
}
