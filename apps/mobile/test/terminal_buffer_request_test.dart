import 'package:codux_flutter/services/terminal_buffer_request.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test(
    'history restore requests bounded tail history for mobile first paint',
    () {
      final payload = buildTerminalBufferRequestPayload(
        requestId: 'request-1',
        mode: TerminalBufferRequestMode.historyRestore,
        offset: 1200,
        maxChars: 65536,
        chunking: true,
        chunkChars: 16384,
        resumeFromSeq: 42,
      );

      expect(payload['tail'], isTrue);
      expect(payload['offset'], 0);
      expect(payload['maxChars'], 65536);
      expect(payload['chunkChars'], 16384);
      expect(payload.containsKey('resumeFromSeq'), isFalse);
    },
  );

  test('live resume keeps the retained history offset and sequence', () {
    final payload = buildTerminalBufferRequestPayload(
      requestId: 'request-2',
      mode: TerminalBufferRequestMode.liveResume,
      offset: 4096,
      maxChars: 65536,
      resumeFromSeq: 77,
    );

    expect(payload['tail'], isFalse);
    expect(payload['offset'], 4096);
    expect(payload['resumeFromSeq'], 77);
  });
}
