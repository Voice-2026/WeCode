import 'dart:async';

import '../models/remote_models.dart';
import 'log_service.dart';
import 'remote_protocol.dart';

typedef TerminalInputReliableSend = bool Function(RelayEnvelope message);
typedef TerminalInputActiveSession = String? Function();
typedef TerminalInputReliableTimerFactory =
    Timer Function(Duration delay, void Function() callback);

class TerminalInputReliableSender {
  TerminalInputReliableSender({
    required TerminalInputReliableSend send,
    TerminalInputActiveSession? activeSessionId,
    this.maxAttempts = 4,
    this.retryBaseDelay = const Duration(milliseconds: 700),
    TerminalInputReliableTimerFactory? timerFactory,
  }) : _send = send,
       _activeSessionId = activeSessionId,
       _timerFactory = timerFactory ?? Timer.new;

  final TerminalInputReliableSend _send;
  final TerminalInputActiveSession? _activeSessionId;
  final TerminalInputReliableTimerFactory _timerFactory;
  final int maxAttempts;
  final Duration retryBaseDelay;

  final Map<String, _PendingTerminalInput> _pending = {};
  int _seq = 0;

  int get pendingCount => _pending.length;

  bool send({
    required String sessionId,
    required String data,
    required String source,
    bool retry = true,
  }) {
    if (sessionId.isEmpty || data.isEmpty) return false;
    final inputId = '${DateTime.now().microsecondsSinceEpoch}-${++_seq}';
    WeCodeLog.info(
      '[wecode-flutter-input] source=$source id=$inputId bytes=${data.codeUnits.length} session=$sessionId',
    );
    _pending[inputId] = _PendingTerminalInput(
      inputId: inputId,
      sessionId: sessionId,
      data: data,
      source: source,
      retry: retry,
    );
    _sendPending(inputId);
    return true;
  }

  void handleAck(RelayEnvelope message) {
    final payload = message.payload;
    if (payload is! Map) return;
    final inputId = payload['inputId']?.toString();
    if (inputId == null || inputId.isEmpty) return;
    final pending = _pending.remove(inputId);
    pending?.retryTimer?.cancel();
    if (WeCodeLog.isDebugEnabled) {
      WeCodeLog.debug(
        '[wecode-flutter-input] ack id=$inputId ok=${payload['ok'] ?? true}',
      );
    }
  }

  void clear({String? sessionId}) {
    final entries = _pending.entries.toList();
    for (final entry in entries) {
      if (sessionId != null && entry.value.sessionId != sessionId) continue;
      entry.value.retryTimer?.cancel();
      _pending.remove(entry.key);
    }
  }

  void dispose() {
    clear();
  }

  void _sendPending(String inputId) {
    final pending = _pending[inputId];
    if (pending == null) return;
    final activeSessionId = _activeSessionId?.call();
    if (activeSessionId != null && pending.sessionId != activeSessionId) {
      return;
    }
    pending.retryTimer?.cancel();
    final sent = _send(
      RelayEnvelope(
        type: RemoteMessageType.terminalInput,
        sessionId: pending.sessionId,
        payload: {
          'data': pending.data,
          'inputId': pending.inputId,
          'source': pending.source,
        },
      ),
    );
    if (!sent) {
      WeCodeLog.warn('[wecode-flutter-input] send failed id=$inputId');
    }
    if (!pending.retry) {
      _pending.remove(inputId);
      return;
    }
    pending.attempt += 1;
    if (pending.attempt >= maxAttempts) {
      _pending.remove(inputId);
      WeCodeLog.warn('[wecode-flutter-input] ack exhausted id=$inputId');
      return;
    }
    pending.retryTimer = _timerFactory(
      Duration(milliseconds: retryBaseDelay.inMilliseconds * pending.attempt),
      () => _sendPending(inputId),
    );
  }
}

class _PendingTerminalInput {
  _PendingTerminalInput({
    required this.inputId,
    required this.sessionId,
    required this.data,
    required this.source,
    required this.retry,
  });

  final String inputId;
  final String sessionId;
  final String data;
  final String source;
  final bool retry;
  int attempt = 0;
  Timer? retryTimer;
}
