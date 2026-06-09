import 'package:codux_flutter/services/terminal_input_payload.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('keeps single control keys raw', () {
    expect(terminalPastePayload('\r'), '\r');
    expect(terminalPastePayload('\u007f'), '\u007f');
  });

  test('wraps bulk inserted text in bracketed paste markers', () {
    expect(terminalPastePayload('BREW。'), '\u001b[200~BREW。\u001b[201~');
    expect(terminalPastePayload('a\nb'), '\u001b[200~a\nb\u001b[201~');
  });
}
