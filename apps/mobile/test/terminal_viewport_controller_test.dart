import 'package:codux_flutter/models/remote_models.dart';
import 'package:codux_flutter/services/terminal_viewport_controller.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('emits the first terminal resize and ignores duplicates', () {
    final controller = TerminalViewportController();

    final first = controller.resize(cols: 80, rows: 24, keyboardVisible: false);
    final duplicate = controller.resize(
      cols: 80,
      rows: 24,
      keyboardVisible: false,
    );

    expect(first, isNotNull);
    expect(first!.cols, 80);
    expect(first.rows, 24);
    expect(duplicate, isNull);
  });

  test('keeps the last row count while keyboard is visible', () {
    final controller = TerminalViewportController();

    controller.resize(cols: 80, rows: 24, keyboardVisible: false);
    final next = controller.resize(cols: 100, rows: 10, keyboardVisible: true);

    expect(next, isNotNull);
    expect(next!.cols, 100);
    expect(next.rows, 24);
    expect(controller.pendingCols, 100);
    expect(controller.pendingRows, 10);
  });

  test('flushes pending keyboard resize when forced', () {
    final controller = TerminalViewportController();

    controller.resize(cols: 80, rows: 24, keyboardVisible: false);
    controller.resize(cols: 100, rows: 10, keyboardVisible: true);

    final flushed = controller.flushPending(force: true);

    expect(flushed, isNotNull);
    expect(flushed!.cols, 100);
    expect(flushed.rows, 10);
  });

  test('tracks remote viewport owner and ignores stale generations', () {
    final controller = TerminalViewportController();

    expect(
      controller.applyRemoteState(
        const RelayEnvelope(
          type: 'terminal.viewport.state',
          sessionId: 'session-1',
          payload: {
            'owner': 'desktop',
            'cols': 120,
            'rows': 40,
            'generation': 2,
          },
        ),
      ),
      isTrue,
    );
    expect(controller.owner, 'desktop');
    expect(controller.generation, 2);

    expect(
      controller.applyRemoteState(
        const RelayEnvelope(
          type: 'terminal.viewport.state',
          sessionId: 'session-1',
          payload: {'owner': 'mobile', 'cols': 80, 'rows': 24, 'generation': 1},
        ),
      ),
      isFalse,
    );
    expect(controller.owner, 'desktop');
    expect(controller.generation, 2);
  });
}
