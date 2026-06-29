import 'dart:ui' as ui;

import 'package:codux_flutter/widgets/components/terminal_builtin_glyphs.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('terminal built-in graphics cover box and block ranges only', () {
    expect(terminalBuiltinGraphic(0x2502), isNotNull);
    expect(terminalBuiltinGraphic(0x2588), isNotNull);
    expect(terminalBuiltinGraphic(0x2595), isNotNull);
    expect(terminalBuiltinGraphic('a'.runes.single), isNull);
    expect(terminalCellCodepoint('│'), 0x2502);
    expect(terminalCellCodepoint('ab'), isNull);
  });

  test('terminal built-in graphics paint without font shaping', () {
    final graphic = terminalBuiltinGraphic(0x2502);
    expect(graphic, isNotNull);

    final recorder = ui.PictureRecorder();
    final canvas = Canvas(recorder);
    paintTerminalBuiltinGraphic(
      canvas,
      const Rect.fromLTWH(0, 0, 12, 24),
      Colors.white,
      graphic!,
    );
    final picture = recorder.endRecording();
    addTearDown(picture.dispose);
    expect(picture.approximateBytesUsed, greaterThan(0));
  });
}
