import 'dart:convert';
import 'dart:io';

import 'package:codux_flutter/services/terminal_payload_codec.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('decodes plain terminal payloads', () {
    final payload = decodeTerminalOutputPayload({
      'data': 'hello',
      'buffer': true,
    });

    expect(payload.data, 'hello');
    expect(payload.isBuffer, isTrue);
    expect(payload.offset, isNull);
  });

  test('decodes deflate compressed terminal payloads', () {
    final compressed = base64Url.encode(
      ZLibCodec(raw: true).encode(utf8.encode('deflate history')),
    );
    final payload = decodeTerminalOutputPayload({
      'data': compressed,
      'compressed': true,
      'encoding': 'base64+deflate+utf8',
      'buffer': true,
      'offset': 12,
      'bufferLength': 24,
      'truncated': true,
    });

    expect(payload.data, 'deflate history');
    expect(payload.isBuffer, isTrue);
    expect(payload.offset, 12);
    expect(payload.bufferLength, 24);
    expect(payload.truncated, isTrue);
  });

  test('decodes terminal buffer protocol metadata', () {
    final payload = decodeTerminalOutputPayload({
      'data': 'tail',
      'buffer': true,
      'requestId': 'request-1',
      'tail': true,
      'hasPrevious': true,
    });

    expect(payload.requestId, 'request-1');
    expect(payload.tail, isTrue);
    expect(payload.hasPrevious, isTrue);
  });
}
