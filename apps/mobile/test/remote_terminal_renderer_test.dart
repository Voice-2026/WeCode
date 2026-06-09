import 'dart:async';

import 'package:codux_flutter/services/remote_terminal_renderer.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('filters standalone shell prompt artifacts', () {
    expect(filterStandalonePromptLines('one\n%\ntwo\n'), 'one\ntwo\n');
    expect(filterStandalonePromptLines('\x1b[32m%\x1b[0m\r\nok'), 'ok');
    expect(filterStandalonePromptLines('100% done\n'), '100% done\n');
  });

  test('stores output until a native terminal is attached', () async {
    final renderer = RemoteTerminalRenderer();
    final port = _FakeTerminalPort();

    renderer.write('hello', replayingBuffer: false);
    renderer.attach(port);

    expect(renderer.restoreControllerWithCached(null), isTrue);
    await _flushNativeQueue();
    expect(port.calls, ['write:hello']);
  });

  test('cached snapshot replaces pending output on restore', () async {
    final renderer = RemoteTerminalRenderer();
    final port = _FakeTerminalPort();

    renderer.write('stale', replayingBuffer: false);
    renderer.attach(port);

    expect(renderer.restoreControllerWithCached('snapshot'), isTrue);
    await _flushNativeQueue();
    expect(port.calls, ['replace:snapshot']);
  });

  test(
    'replace clears pending output and writes snapshot to native terminal',
    () async {
      final renderer = RemoteTerminalRenderer();
      final port = _FakeTerminalPort();

      renderer.write('pending', replayingBuffer: false);
      await renderer.replace('snapshot', replayingBuffer: true);
      renderer.attach(port);

      expect(renderer.restoreControllerWithCached(null), isTrue);
      await _flushNativeQueue();
      expect(port.calls, ['write:snapshot']);

      await renderer.replace('next', replayingBuffer: true);
      expect(port.calls, ['write:snapshot', 'replace:next']);
    },
  );

  test('detach ignores stale ports', () async {
    final renderer = RemoteTerminalRenderer();
    final first = _FakeTerminalPort();
    final second = _FakeTerminalPort();

    renderer.attach(first);
    renderer.detach(second);
    renderer.write('data', replayingBuffer: false);

    await _flushNativeQueue();
    expect(first.calls, ['write:data']);
    expect(second.calls, isEmpty);
  });

  test(
    'clear invalidates queued stale writes before rendering new output',
    () async {
      final renderer = RemoteTerminalRenderer();
      final port = _FakeTerminalPort(delay: const Duration(milliseconds: 1));
      renderer.attach(port);

      renderer.write('stale', replayingBuffer: false);
      unawaited(renderer.clear(sessionId: 'session-1'));
      renderer.write('fresh', replayingBuffer: false);

      await _flushNativeQueue();
      await Future<void>.delayed(const Duration(milliseconds: 5));

      expect(port.calls, ['clear', 'write:fresh']);
    },
  );
}

final class _FakeTerminalPort implements NativeTerminalPort {
  _FakeTerminalPort({this.delay = Duration.zero});

  final Duration delay;
  final List<String> calls = [];
  var disposed = false;

  @override
  Future<void> clear() async {
    if (delay > Duration.zero) await Future<void>.delayed(delay);
    calls.add('clear');
  }

  @override
  Future<void> dispose() async {
    if (delay > Duration.zero) await Future<void>.delayed(delay);
    disposed = true;
    calls.add('dispose');
  }

  @override
  Future<void> replace(String data) async {
    if (delay > Duration.zero) await Future<void>.delayed(delay);
    calls.add('replace:$data');
  }

  @override
  Future<void> write(String data) async {
    if (delay > Duration.zero) await Future<void>.delayed(delay);
    calls.add('write:$data');
  }
}

Future<void> _flushNativeQueue() async {
  await Future<void>.delayed(Duration.zero);
  await Future<void>.delayed(Duration.zero);
}
