import 'package:codux_flutter/models/remote_models.dart';
import 'package:codux_flutter/services/remote_terminal_output_controller.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test(
    'paged snapshot renders only after all retained history pages arrive',
    () {
      final controller = RemoteTerminalOutputController(maxBufferChars: 4);

      controller.bindSession('session-1', requireSnapshot: true);

      final first = controller.accept(
        _terminalBuffer('abcd', offset: 0, bufferLength: 8, truncated: true),
        activeSessionId: 'session-1',
      );

      expect(_kinds(first), [
        RemoteTerminalOutputEffectKind.markBufferReceived,
        RemoteTerminalOutputEffectKind.loading,
        RemoteTerminalOutputEffectKind.requestBufferPage,
        RemoteTerminalOutputEffectKind.ack,
      ]);
      expect(first[1].progress, 0.5);
      expect(first[2].offset, 4);
      expect(controller.cachedOutput('session-1'), isNull);
      expect(controller.bufferOffset('session-1'), 4);

      final second = controller.accept(
        _terminalBuffer('efgh', offset: 4, bufferLength: 8, truncated: false),
        activeSessionId: 'session-1',
      );

      expect(_kinds(second), [
        RemoteTerminalOutputEffectKind.renderSnapshot,
        RemoteTerminalOutputEffectKind.ack,
      ]);
      expect(second.first.data, 'abcdefgh');
      expect(controller.cachedOutput('session-1'), 'abcdefgh');
      expect(controller.bufferOffset('session-1'), 8);
    },
  );

  test('retained tail history can start from a non-zero safe offset', () {
    final controller = RemoteTerminalOutputController(maxBufferChars: 4);

    controller.bindSession('session-1', requireSnapshot: true);

    final first = controller.accept(
      _terminalBuffer('tail', offset: 96, bufferLength: 104, truncated: true),
      activeSessionId: 'session-1',
    );

    expect(_kinds(first), [
      RemoteTerminalOutputEffectKind.markBufferReceived,
      RemoteTerminalOutputEffectKind.loading,
      RemoteTerminalOutputEffectKind.requestBufferPage,
      RemoteTerminalOutputEffectKind.ack,
    ]);
    expect(first[2].offset, 100);
    expect(controller.bufferOffset('session-1'), 100);

    final second = controller.accept(
      _terminalBuffer('next', offset: 100, bufferLength: 104, truncated: false),
      activeSessionId: 'session-1',
    );

    expect(_kinds(second), [
      RemoteTerminalOutputEffectKind.renderSnapshot,
      RemoteTerminalOutputEffectKind.ack,
    ]);
    expect(second.first.data, 'tailnext');
    expect(controller.cachedOutput('session-1'), 'tailnext');
    expect(controller.bufferOffset('session-1'), 104);
  });

  test('out of order snapshot page asks for a fresh full buffer', () {
    final controller = RemoteTerminalOutputController(maxBufferChars: 4);

    controller.bindSession('session-1', requireSnapshot: true);
    controller.accept(
      _terminalBuffer('abcd', offset: 0, bufferLength: 8, truncated: true),
      activeSessionId: 'session-1',
    );

    final result = controller.accept(
      _terminalBuffer('gh', offset: 6, bufferLength: 8, truncated: false),
      activeSessionId: 'session-1',
    );

    expect(_kinds(result), [
      RemoteTerminalOutputEffectKind.ack,
      RemoteTerminalOutputEffectKind.requestFullBuffer,
    ]);
    expect(controller.bufferOffset('session-1'), 0);
  });

  test('live output is held until the full snapshot is restored', () {
    final controller = RemoteTerminalOutputController(maxBufferChars: 4);

    controller.bindSession('session-1', requireSnapshot: true);

    final held = controller.accept(
      _liveOutput('new', outputSeq: 11),
      activeSessionId: 'session-1',
    );

    expect(_kinds(held), [RemoteTerminalOutputEffectKind.ack]);
    expect(controller.cachedOutput('session-1'), isNull);

    final first = controller.accept(
      _terminalBuffer('old-', offset: 0, bufferLength: 8, truncated: true),
      activeSessionId: 'session-1',
    );

    expect(_kinds(first), [
      RemoteTerminalOutputEffectKind.markBufferReceived,
      RemoteTerminalOutputEffectKind.loading,
      RemoteTerminalOutputEffectKind.requestBufferPage,
      RemoteTerminalOutputEffectKind.ack,
    ]);

    final second = controller.accept(
      _terminalBuffer('data', offset: 4, bufferLength: 8, truncated: false),
      activeSessionId: 'session-1',
    );

    expect(_kinds(second), [
      RemoteTerminalOutputEffectKind.renderSnapshot,
      RemoteTerminalOutputEffectKind.ack,
      RemoteTerminalOutputEffectKind.loading,
      RemoteTerminalOutputEffectKind.writeData,
      RemoteTerminalOutputEffectKind.ack,
    ]);
    expect(second.first.data, 'old-data');
    expect(second[3].data, 'new');
    expect(controller.cachedOutput('session-1'), 'old-datanew');
  });

  test('live sequence gaps are accepted without full buffer recovery', () {
    final controller = RemoteTerminalOutputController(maxBufferChars: 4);

    controller.bindSession('session-1', requireSnapshot: false);
    controller.accept(
      _liveOutput('one', outputSeq: 1),
      activeSessionId: 'session-1',
    );
    final skipped = controller.accept(
      _liveOutput('three', outputSeq: 3),
      activeSessionId: 'session-1',
    );

    expect(_kinds(skipped), [
      RemoteTerminalOutputEffectKind.loading,
      RemoteTerminalOutputEffectKind.writeData,
      RemoteTerminalOutputEffectKind.ack,
    ]);
    expect(controller.cachedOutput('session-1'), 'onethree');
  });

  test('stale request id snapshot cannot replace current terminal state', () {
    final controller = RemoteTerminalOutputController(maxBufferChars: 4);

    controller.bindSession('session-1', requireSnapshot: true);
    controller.startBufferRequest('session-1', 'request-new');

    final stale = controller.accept(
      _terminalBuffer(
        'old',
        offset: 0,
        bufferLength: 3,
        truncated: false,
        requestId: 'request-old',
      ),
      activeSessionId: 'session-1',
    );
    expect(stale, isEmpty);
    expect(controller.cachedOutput('session-1'), isNull);

    final current = controller.accept(
      _terminalBuffer(
        'new',
        offset: 0,
        bufferLength: 3,
        truncated: false,
        requestId: 'request-new',
      ),
      activeSessionId: 'session-1',
    );

    expect(_kinds(current), [
      RemoteTerminalOutputEffectKind.renderSnapshot,
      RemoteTerminalOutputEffectKind.ack,
    ]);
    expect(controller.cachedOutput('session-1'), 'new');
  });

  test(
    'full buffer request replaces cache even when recent history offset is non-zero',
    () {
      final controller = RemoteTerminalOutputController(maxBufferChars: 4);

      controller.bindSession('session-1', requireSnapshot: false);
      controller.accept(
        _liveOutput('stale', outputSeq: 1),
        activeSessionId: 'session-1',
      );
      controller.startBufferRequest(
        'session-1',
        'request-1',
        requireSnapshot: true,
      );

      final result = controller.accept(
        _terminalBuffer(
          'tail',
          offset: 96,
          bufferLength: 100,
          truncated: false,
          requestId: 'request-1',
        ),
        activeSessionId: 'session-1',
      );

      expect(_kinds(result), [
        RemoteTerminalOutputEffectKind.renderSnapshot,
        RemoteTerminalOutputEffectKind.ack,
      ]);
      expect(controller.cachedOutput('session-1'), 'tail');
      expect(controller.bufferOffset('session-1'), 100);
    },
  );

  test('tail snapshot renders as current state without requesting pages', () {
    final controller = RemoteTerminalOutputController(maxBufferChars: 4);

    controller.bindSession('session-1', requireSnapshot: true);
    controller.startBufferRequest('session-1', 'request-1');

    final result = controller.accept(
      _terminalBuffer(
        'tail',
        offset: 96,
        bufferLength: 100,
        truncated: false,
        requestId: 'request-1',
        tail: true,
        hasPrevious: true,
      ),
      activeSessionId: 'session-1',
    );

    expect(_kinds(result), [
      RemoteTerminalOutputEffectKind.renderSnapshot,
      RemoteTerminalOutputEffectKind.ack,
    ]);
    expect(controller.cachedOutput('session-1'), 'tail');
    expect(controller.bufferOffset('session-1'), 100);
  });

  test('tail snapshot realigns live output after a sequence gap', () {
    final controller = RemoteTerminalOutputController(maxBufferChars: 4);

    controller.bindSession('session-1', requireSnapshot: false);
    controller.accept(
      _liveOutput('one', outputSeq: 1),
      activeSessionId: 'session-1',
    );
    final gap = controller.accept(
      _liveOutput('gap', outputSeq: 10),
      activeSessionId: 'session-1',
    );
    expect(_kinds(gap), [
      RemoteTerminalOutputEffectKind.loading,
      RemoteTerminalOutputEffectKind.writeData,
      RemoteTerminalOutputEffectKind.ack,
    ]);

    controller.startBufferRequest('session-1', 'request-1');
    final snapshot = controller.accept(
      _terminalBuffer(
        'tail',
        offset: 96,
        bufferLength: 100,
        truncated: false,
        outputSeq: 10,
        requestId: 'request-1',
        tail: true,
        hasPrevious: true,
      ),
      activeSessionId: 'session-1',
    );
    final live = controller.accept(
      _liveOutput('next', outputSeq: 11),
      activeSessionId: 'session-1',
    );

    expect(_kinds(snapshot), [
      RemoteTerminalOutputEffectKind.renderSnapshot,
      RemoteTerminalOutputEffectKind.ack,
    ]);
    expect(_kinds(live), [
      RemoteTerminalOutputEffectKind.loading,
      RemoteTerminalOutputEffectKind.writeData,
      RemoteTerminalOutputEffectKind.ack,
    ]);
    expect(controller.cachedOutput('session-1'), 'tailnext');
  });

  test('screen snapshot replaces current screen without paging', () {
    final controller = RemoteTerminalOutputController(maxBufferChars: 4);

    controller.bindSession('session-1', requireSnapshot: true);
    controller.startBufferRequest('session-1', 'request-1');

    final result = controller.accept(
      _terminalBuffer(
        '\x1b[H\x1b[2Jready',
        offset: 0,
        bufferLength: 12,
        truncated: false,
        requestId: 'request-1',
        tail: true,
        screenSnapshot: true,
      ),
      activeSessionId: 'session-1',
    );

    expect(_kinds(result), [
      RemoteTerminalOutputEffectKind.renderSnapshot,
      RemoteTerminalOutputEffectKind.ack,
    ]);
    expect(controller.cachedOutput('session-1'), '\x1b[H\x1b[2Jready');
  });

  test('inactive live output updates cache without rendering to ui', () {
    final controller = RemoteTerminalOutputController(maxBufferChars: 4);

    final result = controller.accept(
      _liveOutputForSession('session-2', 'background', outputSeq: 1),
      activeSessionId: 'session-1',
    );

    expect(_kinds(result), [RemoteTerminalOutputEffectKind.ack]);
    expect(controller.cachedOutput('session-2'), 'background');
    expect(controller.cachedOutput('session-1'), isNull);
  });

  test('inactive screen snapshot updates cache without rendering to ui', () {
    final controller = RemoteTerminalOutputController(maxBufferChars: 4);

    controller.bindSession('session-2', requireSnapshot: true);
    controller.startBufferRequest('session-2', 'request-2');

    final result = controller.accept(
      _terminalBufferForSession(
        'session-2',
        '\x1b[H\x1b[2Jbackground',
        offset: 0,
        bufferLength: 20,
        truncated: false,
        requestId: 'request-2',
        tail: true,
        screenSnapshot: true,
      ),
      activeSessionId: 'session-1',
    );

    expect(_kinds(result), [RemoteTerminalOutputEffectKind.ack]);
    expect(controller.cachedOutput('session-2'), '\x1b[H\x1b[2Jbackground');
  });
}

RelayEnvelope _terminalBuffer(
  String data, {
  required int offset,
  required int bufferLength,
  required bool truncated,
  int outputSeq = 10,
  String? requestId,
  bool tail = false,
  bool screenSnapshot = false,
  bool hasPrevious = false,
}) {
  final payload = <String, Object?>{
    'data': data,
    'buffer': true,
    'offset': offset,
    'bufferLength': bufferLength,
    'truncated': truncated,
    'outputSeq': outputSeq,
  };
  if (requestId != null) payload['requestId'] = requestId;
  if (tail) payload['tail'] = true;
  if (screenSnapshot) payload['screenSnapshot'] = true;
  if (hasPrevious) payload['hasPrevious'] = true;
  return RelayEnvelope(
    type: 'terminal.output',
    sessionId: 'session-1',
    payload: payload,
  );
}

RelayEnvelope _terminalBufferForSession(
  String sessionId,
  String data, {
  required int offset,
  required int bufferLength,
  required bool truncated,
  int outputSeq = 10,
  String? requestId,
  bool tail = false,
  bool screenSnapshot = false,
  bool hasPrevious = false,
}) {
  final payload = <String, Object?>{
    'data': data,
    'buffer': true,
    'offset': offset,
    'bufferLength': bufferLength,
    'truncated': truncated,
    'outputSeq': outputSeq,
  };
  if (requestId != null) payload['requestId'] = requestId;
  if (tail) payload['tail'] = true;
  if (screenSnapshot) payload['screenSnapshot'] = true;
  if (hasPrevious) payload['hasPrevious'] = true;
  return RelayEnvelope(
    type: 'terminal.output',
    sessionId: sessionId,
    payload: payload,
  );
}

RelayEnvelope _liveOutput(String data, {required int outputSeq}) {
  return _liveOutputForSession('session-1', data, outputSeq: outputSeq);
}

RelayEnvelope _liveOutputForSession(
  String sessionId,
  String data, {
  required int outputSeq,
}) {
  return RelayEnvelope(
    type: 'terminal.output',
    sessionId: sessionId,
    payload: {'data': data, 'outputSeq': outputSeq},
  );
}

List<RemoteTerminalOutputEffectKind> _kinds(
  List<RemoteTerminalOutputEffect> effects,
) => effects.map((effect) => effect.kind).toList();
