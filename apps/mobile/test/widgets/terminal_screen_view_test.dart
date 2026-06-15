import 'package:codux_flutter/theme/app_theme.dart';
import 'package:codux_flutter/widgets/terminal_screen_view.dart';
import 'package:codux_protocol_ffi/codux_protocol_ffi.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('native terminal scroll maps drag direction to core pixels', (
    tester,
  ) async {
    final scrollPixels = <double>[];
    var settleCount = 0;

    await tester.pumpWidget(
      MaterialApp(
        theme: buildAppTheme(),
        home: SizedBox(
          width: 320,
          height: 240,
          child: TerminalScreenView(
            snapshot: _snapshot(totalLines: 200),
            keyboardRequested: false,
            scrollEnabled: true,
            onInput: (_) {},
            onResize: (_, _) {},
            onScrollPixels: (pixels, _) => scrollPixels.add(pixels),
            onSettleScroll: () => settleCount++,
            onScrollToBottom: () {},
            onCursorBottom: (_) {},
          ),
        ),
      ),
    );
    await tester.pump();

    final terminal = find.byType(TerminalScreenView);
    await tester.drag(terminal, const Offset(0, 48));
    await tester.pump();
    final downwardPixels = scrollPixels.fold<double>(
      0,
      (sum, value) => sum + value,
    );
    expect(downwardPixels, greaterThan(0));

    scrollPixels.clear();
    await tester.drag(terminal, const Offset(0, -48));
    await tester.pump();
    final upwardPixels = scrollPixels.fold<double>(
      0,
      (sum, value) => sum + value,
    );
    expect(upwardPixels, lessThan(0));
    expect(settleCount, greaterThanOrEqualTo(1));
  });

  testWidgets(
    'native terminal slow multi-line drag only has tiny tail motion',
    (tester) async {
      final scrollPixels = <double>[];

      await tester.pumpWidget(
        _terminalHarness(
          onScrollPixels: (pixels, _) => scrollPixels.add(pixels),
        ),
      );
      await tester.pump();

      final gesture = await tester.startGesture(
        tester.getCenter(find.byType(TerminalScreenView)),
      );
      for (var i = 0; i < 6; i++) {
        await gesture.moveBy(const Offset(0, 8));
        await tester.pump(const Duration(milliseconds: 60));
      }
      await gesture.up();
      await tester.pump();

      final beforeInertiaWindow = scrollPixels.fold<double>(
        0,
        (sum, value) => sum + value,
      );
      await tester.pump(const Duration(milliseconds: 220));
      final afterInertiaWindow = scrollPixels.fold<double>(
        0,
        (sum, value) => sum + value,
      );

      expect((afterInertiaWindow - beforeInertiaWindow).abs(), lessThan(1));
    },
  );

  testWidgets('native terminal quick fling continues with inertia', (
    tester,
  ) async {
    final scrollPixels = <double>[];

    await tester.pumpWidget(
      _terminalHarness(
        snapshot: _snapshot(totalLines: 200),
        onScrollPixels: (pixels, _) => scrollPixels.add(pixels),
      ),
    );
    await tester.pump();

    await tester.fling(
      find.byType(TerminalScreenView),
      const Offset(0, 160),
      1200,
    );
    await tester.pump();
    final beforeInertiaWindow = scrollPixels.fold<double>(
      0,
      (sum, value) => sum + value,
    );
    await tester.pump(const Duration(milliseconds: 120));
    await tester.pump(const Duration(milliseconds: 120));
    final afterInertiaWindow = scrollPixels.fold<double>(
      0,
      (sum, value) => sum + value,
    );

    expect(afterInertiaWindow, greaterThan(beforeInertiaWindow));
    await tester.pumpAndSettle();
  });

  testWidgets('native terminal does not scroll past live tail', (tester) async {
    final scrollPixels = <double>[];

    await tester.pumpWidget(
      _terminalHarness(onScrollPixels: (pixels, _) => scrollPixels.add(pixels)),
    );
    await tester.pump();

    final gesture = await tester.startGesture(
      tester.getCenter(find.byType(TerminalScreenView)),
    );
    await gesture.moveBy(const Offset(0, -80));
    await tester.pump(const Duration(milliseconds: 16));
    await gesture.up();
    await tester.pump(const Duration(milliseconds: 180));

    expect(scrollPixels, isEmpty);
  });

  testWidgets('native terminal clamps fast scroll to live tail', (
    tester,
  ) async {
    final scrollPixels = <double>[];

    await tester.pumpWidget(
      _terminalHarness(
        snapshot: _snapshot(totalLines: 200),
        onScrollPixels: (pixels, _) => scrollPixels.add(pixels),
      ),
    );
    await tester.pump();

    final terminal = find.byType(TerminalScreenView);
    // Scroll a short distance up into history first.
    await tester.drag(terminal, const Offset(0, 100));
    await tester.pump();
    final intoHistory = scrollPixels.fold<double>(
      0,
      (sum, value) => sum + value,
    );
    expect(intoHistory, greaterThan(0));

    // A fast scroll back down far past the bottom clamps at the live
    // tail: the emitted pixels return exactly the scrolled-back
    // distance and never overshoot it.
    scrollPixels.clear();
    await tester.drag(terminal, const Offset(0, -400));
    await tester.pump(const Duration(milliseconds: 180));

    final total = scrollPixels.fold<double>(0, (sum, value) => sum + value);
    expect(total, lessThan(0));
    expect(total, closeTo(-intoHistory, 0.01));
  });

  testWidgets(
    'native terminal tolerates rebuild before core snapshot catches up',
    (tester) async {
      var snapshot = _snapshot(totalLines: 200, scrollPixelOffset: 0);
      final scrollPixels = <double>[];

      await tester.pumpWidget(
        _terminalHarness(
          snapshot: snapshot,
          onScrollPixels: (pixels, _) => scrollPixels.add(pixels),
        ),
      );
      await tester.pump();

      final terminal = find.byType(TerminalScreenView);
      await tester.drag(terminal, const Offset(0, 48));
      await tester.pump();
      final dragged = scrollPixels.fold<double>(0, (sum, value) => sum + value);
      expect(dragged, greaterThan(0));

      // Rebuild before the host confirms the new scrollback offset: the
      // Flutter-owned position must hold without emitting spurious deltas.
      await tester.pumpWidget(
        _terminalHarness(
          snapshot: snapshot,
          onScrollPixels: (pixels, _) => scrollPixels.add(pixels),
        ),
      );
      await tester.pump();
      expect(find.byType(TerminalScreenView), findsOneWidget);
      expect(
        scrollPixels.fold<double>(0, (sum, value) => sum + value),
        dragged,
      );

      // The host snapshot catches up with the already-applied scroll.
      snapshot = _snapshot(totalLines: 200, scrollPixelOffset: dragged);
      await tester.pumpWidget(
        _terminalHarness(
          snapshot: snapshot,
          onScrollPixels: (pixels, _) => scrollPixels.add(pixels),
        ),
      );
      await tester.pump();
      expect(find.byType(TerminalScreenView), findsOneWidget);
      expect(
        scrollPixels.fold<double>(0, (sum, value) => sum + value),
        dragged,
      );
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets(
    'remote terminal scrolls locally inside the confirmed host window',
    (tester) async {
      final scrollPixels = <double>[];
      final snapshot = _snapshot(
        totalLines: 200,
        displayOffset: 8,
        marginRows: 8,
        marginRowsBelow: 4,
      );

      await tester.pumpWidget(
        _terminalHarness(
          snapshot: snapshot,
          remoteScroll: true,
          onScrollPixels: (pixels, _) => scrollPixels.add(pixels),
        ),
      );
      await tester.pump();
      final scrollable = tester.state<ScrollableState>(find.byType(Scrollable));
      final initialOffset = scrollable.position.pixels;

      await tester.drag(find.byType(TerminalScreenView), const Offset(0, 40));
      await tester.pump();
      expect(scrollable.position.pixels, lessThan(initialOffset));
      expect(scrollPixels, isEmpty);

      // Rebuilding before a new host snapshot arrives keeps the local
      // scroll position; it does not jump back to the previous confirmed
      // edge or emit another remote request.
      await tester.pumpWidget(
        _terminalHarness(
          snapshot: snapshot,
          remoteScroll: true,
          onScrollPixels: (pixels, _) => scrollPixels.add(pixels),
        ),
      );
      await tester.pump();
      expect(scrollable.position.pixels, lessThan(initialOffset));
      expect(scrollPixels, isEmpty);
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets(
    'remote terminal paints only the confirmed host snapshot window',
    (tester) async {
      final scrollPixels = <double>[];
      final snapshot = _snapshot(
        totalLines: 200,
        displayOffset: 8,
        marginRows: 4,
        marginRowsBelow: 2,
      );

      await tester.pumpWidget(
        _terminalHarness(
          snapshot: snapshot,
          remoteScroll: true,
          onScrollPixels: (pixels, _) => scrollPixels.add(pixels),
        ),
      );
      await tester.pump();

      await tester.drag(find.byType(TerminalScreenView), const Offset(0, 400));
      await tester.pump();

      expect(scrollPixels.fold<double>(0, (sum, value) => sum + value), 400);
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets('remote terminal with no history cannot scroll past the prompt', (
    tester,
  ) async {
    final scrollPixels = <double>[];

    await tester.pumpWidget(
      _terminalHarness(
        snapshot: _snapshot(totalLines: 1, rows: 1),
        remoteScroll: true,
        onScrollPixels: (pixels, _) => scrollPixels.add(pixels),
      ),
    );
    await tester.pump();

    await tester.drag(find.byType(TerminalScreenView), const Offset(0, 160));
    await tester.pump();
    await tester.pump();

    final scrollable = tester.state<ScrollableState>(find.byType(Scrollable));
    expect(scrollable.position.pixels, 0);
    expect(scrollPixels, isEmpty);
    expect(tester.takeException(), isNull);
  });

  testWidgets(
    'remote terminal ignores empty host rows when only the prompt has content',
    (tester) async {
      final scrollPixels = <double>[];

      await tester.pumpWidget(
        _terminalHarness(
          snapshot: _snapshot(
            totalLines: 24,
            rows: 24,
            cursorRow: 0,
            cellRow: 0,
          ),
          remoteScroll: true,
          onScrollPixels: (pixels, _) => scrollPixels.add(pixels),
        ),
      );
      await tester.pump();

      await tester.drag(find.byType(TerminalScreenView), const Offset(0, 160));
      await tester.pump();
      await tester.pump();

      final scrollable = tester.state<ScrollableState>(find.byType(Scrollable));
      expect(scrollable.position.pixels, 0);
      expect(scrollPixels, isEmpty);
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets('remote terminal with no history ignores upward drags too', (
    tester,
  ) async {
    final scrollPixels = <double>[];

    await tester.pumpWidget(
      _terminalHarness(
        snapshot: _snapshot(totalLines: 24, rows: 24, cursorRow: 0, cellRow: 0),
        remoteScroll: true,
        onScrollPixels: (pixels, _) => scrollPixels.add(pixels),
      ),
    );
    await tester.pump();

    await tester.drag(find.byType(TerminalScreenView), const Offset(0, -160));
    await tester.pump();
    await tester.pump();

    final scrollable = tester.state<ScrollableState>(find.byType(Scrollable));
    expect(scrollable.position.pixels, 0);
    expect(scrollPixels, isEmpty);
    expect(tester.takeException(), isNull);
  });

  testWidgets(
    'remote terminal does not pin to bottom when total lines are only screen rows',
    (tester) async {
      final scrollPixels = <double>[];

      await tester.pumpWidget(
        _terminalHarness(
          height: 240,
          snapshot: _snapshot(totalLines: 80, rows: 80),
          remoteScroll: true,
          onScrollPixels: (pixels, _) => scrollPixels.add(pixels),
        ),
      );
      await tester.pump();
      await tester.pump();

      final scrollable = tester.state<ScrollableState>(find.byType(Scrollable));
      expect(scrollable.position.pixels, 0);
      expect(scrollPixels, isEmpty);
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets('remote terminal row guard handles invalid host margins', (
    tester,
  ) async {
    final scrollPixels = <double>[];

    await tester.pumpWidget(
      _terminalHarness(
        snapshot: _snapshot(
          totalLines: 24,
          rows: 10,
          marginRows: 10,
          marginRowsBelow: 10,
        ),
        remoteScroll: true,
        onScrollPixels: (pixels, _) => scrollPixels.add(pixels),
      ),
    );
    await tester.pump();

    await tester.drag(find.byType(TerminalScreenView), const Offset(0, 160));
    await tester.pump();
    await tester.pump();

    expect(tester.takeException(), isNull);
  });

  testWidgets('remote terminal requests host data when leaving confirmed window', (
    tester,
  ) async {
    final scrollPixels = <double>[];
    final snapshot = _snapshot(
      totalLines: 200,
      displayOffset: 8,
      marginRows: 4,
      marginRowsBelow: 2,
    );

    await tester.pumpWidget(
      _terminalHarness(
        snapshot: snapshot,
        remoteScroll: true,
        onScrollPixels: (pixels, _) => scrollPixels.add(pixels),
      ),
    );
    await tester.pump();

    await tester.drag(find.byType(TerminalScreenView), const Offset(0, 400));
    await tester.pump();
    await tester.pump();

    expect(scrollPixels.fold<double>(0, (sum, value) => sum + value), 400);
    expect(tester.takeException(), isNull);
  });

  testWidgets('remote terminal prefetches before reaching snapshot edge', (
    tester,
  ) async {
    final scrollPixels = <double>[];
    final snapshot = _snapshot(
      totalLines: 200,
      displayOffset: 24,
      rows: 60,
      marginRows: 24,
      marginRowsBelow: 24,
    );

    await tester.pumpWidget(
      _terminalHarness(
        snapshot: snapshot,
        remoteScroll: true,
        onScrollPixels: (pixels, _) => scrollPixels.add(pixels),
      ),
    );
    await tester.pump();

    await tester.drag(find.byType(TerminalScreenView), const Offset(0, 120));
    await tester.pump();
    expect(scrollPixels, isEmpty);

    await tester.drag(find.byType(TerminalScreenView), const Offset(0, 180));
    await tester.pump();
    expect(scrollPixels.fold<double>(0, (sum, value) => sum + value), 300);
    expect(tester.takeException(), isNull);
  });

  testWidgets(
    'remote terminal prefetches history from live-tail snapshot',
    (tester) async {
      final scrollPixels = <double>[];
      final snapshot = _snapshot(
        totalLines: 200,
        displayOffset: 0,
        rows: 48,
        marginRows: 24,
        marginRowsBelow: 0,
      );

      await tester.pumpWidget(
        _terminalHarness(
          snapshot: snapshot,
          remoteScroll: true,
          onScrollPixels: (pixels, _) => scrollPixels.add(pixels),
        ),
      );
      await tester.pump();

      await tester.drag(find.byType(TerminalScreenView), const Offset(0, 120));
      await tester.pump();
      expect(scrollPixels.fold<double>(0, (sum, value) => sum + value), 120);
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets(
    'remote terminal clamps only at the full scrollback boundary',
    (tester) async {
      final scrollPixels = <double>[];
      final snapshot = _snapshot(
        totalLines: 200,
        displayOffset: 8,
        marginRows: 4,
        marginRowsBelow: 2,
      );

      await tester.pumpWidget(
        _terminalHarness(
          snapshot: snapshot,
          remoteScroll: true,
          onScrollPixels: (pixels, _) => scrollPixels.add(pixels),
        ),
      );
      await tester.pump();

      final scrollable = tester.state<ScrollableState>(find.byType(Scrollable));
      final initialOffset = scrollable.position.pixels;

      await tester.drag(find.byType(TerminalScreenView), const Offset(0, 5000));
      await tester.pump();

      expect(scrollable.position.pixels, 0);
      expect(scrollable.position.pixels, lessThan(initialOffset));
      // Only the actual movement from the current viewport to the top of
      // scrollback is sent to the host. Extra drag distance is ScrollView
      // boundary overscroll, not terminal history.
      expect(
        scrollPixels.fold<double>(0, (sum, value) => sum + value),
        closeTo(initialOffset, 0.01),
      );
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets('native terminal defers follow-tail scroll until after build', (
    tester,
  ) async {
    var scrollToBottomCount = 0;

    await tester.pumpWidget(
      _terminalHarness(
        snapshot: _snapshot(displayOffset: 0, data: 'first'),
        onScrollPixels: (_, _) {},
        onScrollToBottom: () => scrollToBottomCount++,
      ),
    );
    await tester.pump();

    await tester.pumpWidget(
      _terminalHarness(
        snapshot: _snapshot(displayOffset: 1, data: 'second'),
        onScrollPixels: (_, _) {},
        onScrollToBottom: () => scrollToBottomCount++,
      ),
    );

    expect(tester.takeException(), isNull);
    await tester.pump();
    expect(scrollToBottomCount, 1);
  });

  testWidgets('native terminal keyboard request owns input focus', (
    tester,
  ) async {
    final inputs = <String>[];
    await tester.pumpWidget(
      _terminalHarness(
        keyboardRequested: false,
        onInput: inputs.add,
        onScrollPixels: (_, _) {},
      ),
    );
    await tester.pump();

    expect(tester.testTextInput.hasAnyClients, isFalse);

    await tester.pumpWidget(
      _terminalHarness(
        keyboardRequested: true,
        onInput: inputs.add,
        onScrollPixels: (_, _) {},
      ),
    );
    await tester.pump();
    await tester.pump();
    expect(tester.testTextInput.hasAnyClients, isTrue);
    expect(tester.testTextInput.setClientArgs?['enableDeltaModel'], isFalse);

    tester.testTextInput.updateEditingValue(
      const TextEditingValue(
        text: '  h',
        selection: TextSelection.collapsed(offset: 3),
      ),
    );
    await tester.pump();
    expect(inputs, ['h']);

    tester.testTextInput.updateEditingValue(
      const TextEditingValue(
        text: ' ',
        selection: TextSelection.collapsed(offset: 1),
      ),
    );
    await tester.pump();
    expect(inputs, ['h', '\u007f']);

    tester.testTextInput.updateEditingValue(
      const TextEditingValue(
        text: '  \n你好\u{f700}',
        selection: TextSelection.collapsed(offset: 6),
      ),
    );
    await tester.pump();
    expect(inputs, ['h', '\u007f', '\r你好']);

    tester.testTextInput.updateEditingValue(
      const TextEditingValue(
        text: '  你',
        selection: TextSelection.collapsed(offset: 3),
      ),
    );
    await tester.pump();
    expect(inputs, ['h', '\u007f', '\r你好', '你']);

    await _sendTextInputSelectors(tester, ['deleteBackward:', 'moveLeft:']);
    await tester.pump();
    expect(inputs, ['h', '\u007f', '\r你好', '你', '\u007f', '\u001b[D']);

    await tester.sendKeyEvent(LogicalKeyboardKey.backspace);
    await tester.pump();
    expect(inputs, [
      'h',
      '\u007f',
      '\r你好',
      '你',
      '\u007f',
      '\u001b[D',
      '\u007f',
    ]);

    await tester.pumpWidget(
      _terminalHarness(
        keyboardRequested: false,
        onInput: inputs.add,
        onScrollPixels: (_, _) {},
      ),
    );
    await tester.pump();
    await tester.pump();
    expect(tester.testTextInput.hasAnyClients, isFalse);
  });

  testWidgets('long press selects terminal text for copy', (tester) async {
    String? selectedText;
    await tester.pumpWidget(
      MaterialApp(
        theme: buildAppTheme(),
        home: SizedBox(
          width: 320,
          height: 24,
          child: TerminalScreenView(
            snapshot: _textSnapshot('hello'),
            keyboardRequested: false,
            scrollEnabled: true,
            onInput: (_) {},
            onResize: (_, _) {},
            onScrollPixels: (_, _) {},
            onSettleScroll: () {},
            onScrollToBottom: () {},
            onCursorBottom: (_) {},
            onSelectionChanged: (text) => selectedText = text,
          ),
        ),
      ),
    );
    await tester.pump();

    final terminal = find.byType(TerminalScreenView);
    final topLeft = tester.getTopLeft(terminal);
    final gesture = await tester.startGesture(topLeft + const Offset(4, 590));
    await tester.pump(const Duration(milliseconds: 650));
    await gesture.up();
    await tester.pump();

    expect(selectedText, 'h');
  });
}

Future<void> _sendTextInputSelectors(
  WidgetTester tester,
  List<String> selectors,
) async {
  final client = _currentTextInputClient(tester);
  final message = SystemChannels.textInput.codec.encodeMethodCall(
    MethodCall('TextInputClient.performSelectors', [client, selectors]),
  );
  await TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
      .handlePlatformMessage(SystemChannels.textInput.name, message, (_) {});
}

int _currentTextInputClient(WidgetTester tester) {
  return tester.testTextInput.log
          .where((call) => call.method == 'TextInput.setClient')
          .map((call) => call.arguments as List<dynamic>)
          .last
          .first
      as int;
}

Widget _terminalHarness({
  TerminalScreenSnapshot? snapshot,
  bool keyboardRequested = false,
  bool remoteScroll = false,
  double height = 240,
  ValueChanged<String>? onInput,
  ValueChanged<String?>? onSelectionChanged,
  required void Function(double pixels, double cellHeight) onScrollPixels,
  VoidCallback? onScrollToBottom,
}) {
  return MaterialApp(
    theme: buildAppTheme(),
    home: SizedBox(
      width: 320,
      height: height,
      child: TerminalScreenView(
        snapshot: snapshot ?? _snapshot(),
        keyboardRequested: keyboardRequested,
        scrollEnabled: true,
        remoteScroll: remoteScroll,
        onInput: onInput ?? (_) {},
        onResize: (_, _) {},
        onScrollPixels: onScrollPixels,
        onSettleScroll: () {},
        onScrollToBottom: onScrollToBottom ?? () {},
        onCursorBottom: (_) {},
        onSelectionChanged: onSelectionChanged,
      ),
    ),
  );
}

TerminalScreenSnapshot _textSnapshot(String text) {
  return TerminalScreenSnapshot(
    data: text,
    cols: 80,
    rows: 1,
    totalLines: 1,
    displayOffset: 0,
    scrollPixelOffset: 0,
    applicationCursor: false,
    cells: [
      for (var i = 0; i < text.length; i++)
        TerminalScreenCell(
          row: 0,
          col: i,
          text: text[i],
          width: 1,
          fg: {'kind': 'default'},
          bg: {'kind': 'default'},
          bold: false,
          dim: false,
          italic: false,
          underline: false,
          inverse: false,
          hidden: false,
          strikeout: false,
        ),
    ],
    cursor: const TerminalScreenCursor(
      row: 0,
      col: 5,
      visible: true,
      shape: TerminalScreenCursorShape.block,
    ),
  );
}

TerminalScreenSnapshot _snapshot({
  int displayOffset = 0,
  double scrollPixelOffset = 0,
  String data = 'ready',
  int totalLines = 24,
  int rows = 24,
  int cursorRow = 0,
  int cellRow = 0,
  int marginRows = 0,
  int marginRowsBelow = 0,
}) {
  return TerminalScreenSnapshot(
    data: data,
    cols: 80,
    rows: rows,
    totalLines: totalLines,
    displayOffset: displayOffset,
    marginRows: marginRows,
    marginRowsBelow: marginRowsBelow,
    scrollPixelOffset: scrollPixelOffset,
    applicationCursor: false,
    cells: [
      TerminalScreenCell(
        row: cellRow,
        col: 0,
        text: 'r',
        width: 1,
        fg: {'kind': 'default'},
        bg: {'kind': 'default'},
        bold: false,
        dim: false,
        italic: false,
        underline: false,
        inverse: false,
        hidden: false,
        strikeout: false,
      ),
    ],
    cursor: TerminalScreenCursor(
      row: cursorRow,
      col: 1,
      visible: true,
      shape: TerminalScreenCursorShape.block,
    ),
  );
}
