import 'package:codux_flutter/services/remote_pty_session.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test(
    'remote pty session restores baseline before replaying held live output',
    () {
      final session = RemotePtySession<String>(
        'session-1',
        maxCachedChars: 1000,
      );

      session.requireBaseline();
      expect(session.holdLive(sequence: 11, output: 'new'), isTrue);

      final first = session.acceptBaselinePage(
        data: 'old-',
        offset: 0,
        bufferLength: 8,
        truncated: true,
      );
      expect(first.ready, isFalse);
      expect(session.content, '');
      expect(session.bufferLength, 4);

      final second = session.acceptBaselinePage(
        data: 'data',
        offset: 4,
        bufferLength: 8,
        truncated: false,
      );
      expect(second.ready, isTrue);

      final replay = session.replaceFromBaseline(
        content: second.data,
        bufferLength: 8,
        sequence: 10,
      );

      expect(session.content, 'old-data');
      expect(session.bufferLength, 8);
      expect(session.sequence, 10);
      expect(replay, ['new']);
    },
  );

  test(
    'remote pty session trims cached content without changing remote offset',
    () {
      final session = RemotePtySession<String>('session-1', maxCachedChars: 5);

      session.appendLive(data: 'abcdef', bufferLength: 6, sequence: 1);

      expect(session.content, 'bcdef');
      expect(session.bufferLength, 6);
      expect(session.sequence, 1);
    },
  );

  test('remote pty session trims cache on rune boundaries', () {
    final session = RemotePtySession<String>('session-1', maxCachedChars: 4);

    session.appendLive(data: 'a你好bcd', bufferLength: 7, sequence: 2);

    expect(session.content, '好bcd');
    expect(session.bufferLength, 7);
    expect(session.sequence, 2);
  });
}
