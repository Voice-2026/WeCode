import 'package:wecode_protocol_ffi/wecode_protocol_ffi.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('keeps single control keys raw', () {
    expect(terminalInsertInput('\r'), '\r');
    expect(terminalInsertInput('\u007f'), '\u007f');
  });

  test('wraps bulk inserted text in bracketed paste markers', () {
    expect(terminalInsertInput('BREW。'), '\u001b[200~BREW。\u001b[201~');
    expect(terminalInsertInput('a\nb'), '\u001b[200~a\nb\u001b[201~');
  });
}
