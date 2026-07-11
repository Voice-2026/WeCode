import 'dart:async';

import 'package:wecode_flutter/models/remote_models.dart';
import 'package:wecode_flutter/services/terminal_input_reliable_sender.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('sends terminal input with input id and retries until ack', () {
    final timers = <_FakeTimer>[];
    final sent = <RelayEnvelope>[];
    final sender = TerminalInputReliableSender(
      activeSessionId: () => 'session-1',
      retryBaseDelay: const Duration(milliseconds: 10),
      send: (message) {
        sent.add(message);
        return true;
      },
      timerFactory: (delay, callback) {
        final timer = _FakeTimer(callback);
        timers.add(timer);
        return timer;
      },
    );

    expect(
      sender.send(sessionId: 'session-1', data: 'a', source: 'key'),
      isTrue,
    );
    expect(sent.single.type, 'terminal.input');
    expect(sent.single.sessionId, 'session-1');
    expect((sent.single.payload as Map)['data'], 'a');
    expect((sent.single.payload as Map)['source'], 'key');

    timers.single.fire();

    expect(sent.length, 2);
    sender.handleAck(
      RelayEnvelope(
        type: 'terminal.input.ack',
        payload: {'inputId': (sent.last.payload as Map)['inputId']},
      ),
    );
    expect(sender.pendingCount, 0);
  });

  test('does not resend when active session changes', () {
    final timers = <_FakeTimer>[];
    final sent = <RelayEnvelope>[];
    var activeSession = 'session-1';
    final sender = TerminalInputReliableSender(
      activeSessionId: () => activeSession,
      send: (message) {
        sent.add(message);
        return true;
      },
      timerFactory: (delay, callback) {
        final timer = _FakeTimer(callback);
        timers.add(timer);
        return timer;
      },
    );

    sender.send(sessionId: 'session-1', data: 'a', source: 'key');
    activeSession = 'session-2';
    timers.single.fire();

    expect(sent.length, 1);
    expect(sender.pendingCount, 1);
  });

  test('clear cancels pending retry timers', () {
    final timers = <_FakeTimer>[];
    final sender = TerminalInputReliableSender(
      activeSessionId: () => 'session-1',
      send: (_) => true,
      timerFactory: (delay, callback) {
        final timer = _FakeTimer(callback);
        timers.add(timer);
        return timer;
      },
    );

    sender.send(sessionId: 'session-1', data: 'a', source: 'key');
    sender.clear(sessionId: 'session-1');

    expect(sender.pendingCount, 0);
    expect(timers.single.isActive, isFalse);
  });

  test('non retrying control input is sent once and not retained', () {
    final timers = <_FakeTimer>[];
    final sent = <RelayEnvelope>[];
    final sender = TerminalInputReliableSender(
      activeSessionId: () => 'session-1',
      send: (message) {
        sent.add(message);
        return true;
      },
      timerFactory: (delay, callback) {
        final timer = _FakeTimer(callback);
        timers.add(timer);
        return timer;
      },
    );

    final ok = sender.send(
      sessionId: 'session-1',
      data: '\u0003',
      source: 'key',
      retry: false,
    );

    expect(ok, isTrue);
    expect(sent, hasLength(1));
    expect((sent.single.payload as Map)['data'], '\u0003');
    expect(sender.pendingCount, 0);
    expect(timers, isEmpty);
  });
}

final class _FakeTimer implements Timer {
  _FakeTimer(this._callback);

  final void Function() _callback;
  var _active = true;

  void fire() {
    if (!_active) return;
    _active = false;
    _callback();
  }

  @override
  void cancel() {
    _active = false;
  }

  @override
  bool get isActive => _active;

  @override
  int get tick => 0;
}
