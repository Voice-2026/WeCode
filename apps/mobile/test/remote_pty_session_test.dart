import 'package:codux_flutter/services/remote_pty_session.dart';
import 'package:flutter_test/flutter_test.dart';

String _scrollableHistory(String prefix) {
  return List.generate(
    12,
    (index) => '$prefix ${(index + 1).toString().padLeft(2, '0')}',
  ).join('\r\n');
}

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

  test('remote pty session maintains headless terminal screen', () {
    final session = RemotePtySession<String>('session-1', maxCachedChars: 200);

    session.replaceFromBaseline(
      content: 'old line\n\u001b[2J\u001b[Htop',
      bufferLength: 20,
      sequence: 1,
    );
    session.appendLive(
      data: '\u001b[3;5Hbottom',
      bufferLength: 26,
      sequence: 2,
    );

    final screen = session.screenSnapshot();

    expect(screen.data, contains('top'));
    expect(screen.data, contains('bottom'));
    expect(screen.data, isNot(contains('old line')));
    expect(screen.cells.any((cell) => cell.text == 't'), isTrue);
  });

  test('remote pty session restores visible screen from screen baseline', () {
    final session = RemotePtySession<String>('session-1', maxCachedChars: 200);

    session.replaceFromBaseline(
      content: 'raw history fragment that stays cached',
      screenData: '\u001b[2J\u001b[Hvisible tui',
      bufferLength: 38,
      sequence: 3,
    );

    final screen = session.screenSnapshot();

    expect(session.content, 'raw history fragment that stays cached');
    expect(session.bufferLength, 38);
    expect(session.sequence, 3);
    expect(screen.data, contains('visible tui'));
    expect(screen.data, isNot(contains('raw history')));
  });

  test('remote pty session scrolls raw history behind screen baseline', () {
    final session = RemotePtySession<String>('session-1', maxCachedChars: 512);
    session.resizeScreen(cols: 20, rows: 8);
    final history = _scrollableHistory('raw history');

    session.replaceFromBaseline(
      content: history,
      screenData: '\u001b[2J\u001b[Hvisible tui',
      bufferLength: history.length,
      sequence: 3,
    );

    var screen = session.screenSnapshot();
    expect(session.content, history);
    expect(screen.data, contains('visible tui'));
    expect(screen.data, isNot(contains('raw history 01')));

    session.scrollScreenLines(8);
    screen = session.screenSnapshot();
    expect(screen.displayOffset, greaterThan(0));
    expect(screen.data, contains('raw history'));
    expect(screen.data, isNot(contains('visible tui')));

    session.scrollScreenToBottom();
    screen = session.screenSnapshot();
    expect(screen.displayOffset, 0);
    expect(screen.data, contains('visible tui'));
    expect(screen.data, isNot(contains('raw history 01')));
  });

  test('remote pty session applies live screen keyframe', () {
    final session = RemotePtySession<String>('session-1', maxCachedChars: 200);

    session.replaceFromBaseline(
      content: 'cached raw history',
      screenData: '\u001b[2J\u001b[Hold screen',
      bufferLength: 18,
      sequence: 3,
    );
    session.appendLive(
      data: 'partial live raw',
      screenData: '\u001b[2J\u001b[Hrestored tui\n\u001b[3;1Hinput box',
      bufferLength: 32,
      sequence: 4,
    );

    final screen = session.screenSnapshot();

    expect(session.content, 'cached raw historypartial live raw');
    expect(session.bufferLength, 32);
    expect(session.sequence, 4);
    expect(screen.data, contains('restored tui'));
    expect(screen.data, contains('input box'));
    expect(screen.data, isNot(contains('partial live raw')));
    expect(screen.data, isNot(contains('old screen')));
  });

  test('remote pty session keeps live keyframe out of raw history', () {
    final session = RemotePtySession<String>('session-1', maxCachedChars: 512);
    session.resizeScreen(cols: 20, rows: 8);
    final history = _scrollableHistory('history');

    session.replaceFromBaseline(
      content: history,
      screenData: '\u001b[2J\u001b[Hold screen\r\nold input',
      bufferLength: history.length,
      sequence: 3,
    );
    session.appendLive(
      data: 'live raw',
      screenData: '\u001b[2J\u001b[Hnew screen\r\n\u001b[3;1Hnew input',
      bufferLength: history.length + 8,
      sequence: 4,
    );

    var screen = session.screenSnapshot();
    expect(screen.data, contains('new screen'));
    expect(screen.data, contains('new input'));
    expect(screen.data, isNot(contains('old screen')));
    expect(screen.data, isNot(contains('history 01')));

    session.scrollScreenLines(8);
    screen = session.screenSnapshot();
    expect(screen.displayOffset, greaterThan(0));
    expect(screen.data, contains('history'));
    expect(screen.data, isNot(contains('old screen')));
    expect(screen.data, isNot(contains('new screen')));
  });

  test('remote pty session owns pixel viewport and overscan state', () {
    final session = RemotePtySession<String>('session-1', maxCachedChars: 200);
    session.resizeScreen(cols: 20, rows: 8);
    final content = List.generate(
      12,
      (index) => 'line ${index + 1}',
    ).join('\r\n');

    session.replaceFromBaseline(
      content: content,
      bufferLength: content.length,
      sequence: 1,
    );
    session.scrollScreenPixels(pixels: 7, cellHeight: 10);

    var screen = session.screenSnapshot();
    expect(screen.displayOffset, 0);
    expect(screen.scrollPixelOffset, 7);

    session.scrollScreenPixels(pixels: 6, cellHeight: 10);
    screen = session.screenSnapshot();

    expect(screen.displayOffset, greaterThan(0));
    expect(screen.scrollPixelOffset, 3);
    expect(
      screen.cells.any((cell) => cell.row < 0 || cell.row >= screen.rows),
      isTrue,
    );

    session.settleScreenPixelScroll();
    expect(session.screenSnapshot().scrollPixelOffset, 3);
  });
}
