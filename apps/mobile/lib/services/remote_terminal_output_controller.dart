import '../models/remote_models.dart';
import 'log_service.dart';
import 'remote_pty_session.dart';
import 'terminal_buffer_assembler.dart';
import 'terminal_output_resync.dart';
import 'terminal_output_sequencer.dart';
import 'terminal_payload_codec.dart';

enum RemoteTerminalBufferPhase { idle, requesting, receiving, rendering }

enum RemoteTerminalOutputEffectKind {
  loading,
  ack,
  requestFullBuffer,
  requestBufferPage,
  markBufferReceived,
  renderSnapshot,
  writeData,
}

class RemoteTerminalOutputEffect {
  const RemoteTerminalOutputEffect._({
    required this.kind,
    this.sessionId,
    this.outputSeq,
    this.bufferLength,
    this.offset,
    this.data,
    this.progress,
    this.phase,
    this.replayingBuffer = false,
    this.loading = false,
  });

  factory RemoteTerminalOutputEffect.loading({
    required bool loading,
    RemoteTerminalBufferPhase phase = RemoteTerminalBufferPhase.requesting,
    double? progress,
  }) => RemoteTerminalOutputEffect._(
    kind: RemoteTerminalOutputEffectKind.loading,
    loading: loading,
    phase: phase,
    progress: progress,
  );

  factory RemoteTerminalOutputEffect.ack({
    required String sessionId,
    required int? outputSeq,
    required int? bufferLength,
  }) => RemoteTerminalOutputEffect._(
    kind: RemoteTerminalOutputEffectKind.ack,
    sessionId: sessionId,
    outputSeq: outputSeq,
    bufferLength: bufferLength,
  );

  factory RemoteTerminalOutputEffect.requestFullBuffer(String sessionId) =>
      RemoteTerminalOutputEffect._(
        kind: RemoteTerminalOutputEffectKind.requestFullBuffer,
        sessionId: sessionId,
      );

  factory RemoteTerminalOutputEffect.requestBufferPage({
    required String sessionId,
    required int offset,
  }) => RemoteTerminalOutputEffect._(
    kind: RemoteTerminalOutputEffectKind.requestBufferPage,
    sessionId: sessionId,
    offset: offset,
  );

  factory RemoteTerminalOutputEffect.markBufferReceived(String sessionId) =>
      RemoteTerminalOutputEffect._(
        kind: RemoteTerminalOutputEffectKind.markBufferReceived,
        sessionId: sessionId,
      );

  factory RemoteTerminalOutputEffect.renderSnapshot({
    required String sessionId,
    required String data,
  }) => RemoteTerminalOutputEffect._(
    kind: RemoteTerminalOutputEffectKind.renderSnapshot,
    sessionId: sessionId,
    data: data,
  );

  factory RemoteTerminalOutputEffect.writeData({
    required String sessionId,
    required String data,
    required bool replayingBuffer,
  }) => RemoteTerminalOutputEffect._(
    kind: RemoteTerminalOutputEffectKind.writeData,
    sessionId: sessionId,
    data: data,
    replayingBuffer: replayingBuffer,
  );

  final RemoteTerminalOutputEffectKind kind;
  final String? sessionId;
  final int? outputSeq;
  final int? bufferLength;
  final int? offset;
  final String? data;
  final double? progress;
  final RemoteTerminalBufferPhase? phase;
  final bool replayingBuffer;
  final bool loading;
}

class RemoteTerminalOutputController {
  RemoteTerminalOutputController({
    int maxBufferChars = 200000,
    int maxCachedChars = 2000000,
  }) : _ptySessions = RemotePtySessionStore<RelayEnvelope>(
         maxCachedChars: maxCachedChars,
       ),
       _assembler = TerminalBufferAssembler(maxChars: maxBufferChars);

  final RemotePtySessionStore<RelayEnvelope> _ptySessions;
  final TerminalBufferAssembler _assembler;
  final TerminalOutputSequencer _sequencer = TerminalOutputSequencer();
  final Map<String, String> _activeBufferRequestBySession = {};

  String? cachedOutput(String sessionId) => _ptySessions.content(sessionId);

  bool hasCachedOutput(String sessionId) =>
      _ptySessions.content(sessionId) != null;

  int bufferOffset(String sessionId) => _ptySessions.bufferLength(sessionId);

  int sequenceFor(String sessionId) => _ptySessions.sequence(sessionId);

  String? activeBufferRequestId(String sessionId) =>
      _activeBufferRequestBySession[sessionId];

  void startBufferRequest(
    String sessionId,
    String requestId, {
    bool requireSnapshot = false,
  }) {
    if (sessionId.trim().isEmpty || requestId.trim().isEmpty) return;
    _activeBufferRequestBySession[sessionId] = requestId;
    _assembler.remove(sessionId);
    if (requireSnapshot) {
      _ptySessions.session(sessionId).requireSnapshot();
    }
  }

  void bindSession(String sessionId, {required bool requireSnapshot}) {
    if (sessionId.trim().isEmpty) return;
    _sequencer.remove(sessionId);
    _assembler.remove(sessionId);
    final session = _ptySessions.session(sessionId);
    if (requireSnapshot) {
      session.requireSnapshot();
    } else {
      session.resetTransient();
    }
  }

  void removeSession(String sessionId) {
    _ptySessions.remove(sessionId);
    _activeBufferRequestBySession.remove(sessionId);
    _assembler.remove(sessionId);
    _sequencer.remove(sessionId);
  }

  void resetTransient() {
    _assembler.reset();
    _activeBufferRequestBySession.clear();
  }

  void resetSessionTransient(String sessionId, {bool resetSequence = false}) {
    _assembler.remove(sessionId);
    if (resetSequence) _sequencer.remove(sessionId);
    _ptySessions
        .session(sessionId)
        .resetTransient(resetSequence: resetSequence);
  }

  void resetAll() {
    _ptySessions.clear();
    _activeBufferRequestBySession.clear();
    _assembler.reset();
    _sequencer.reset();
  }

  List<RemoteTerminalOutputEffect> accept(
    RelayEnvelope message, {
    required String? activeSessionId,
  }) {
    return _accept(
      message,
      activeSessionId: activeSessionId,
      replayingHeldLive: false,
    );
  }

  List<RemoteTerminalOutputEffect> _accept(
    RelayEnvelope message, {
    required String? activeSessionId,
    required bool replayingHeldLive,
  }) {
    var payload = message.payload;
    if (payload is! Map || payload['data'] == null) return const [];
    final sessionId = message.sessionId;
    if (sessionId == null || sessionId.trim().isEmpty) {
      return const [];
    }
    final isActiveSession = sessionId == activeSessionId;
    if (!isActiveSession) {
      CoduxLog.debug(
        '[codux-flutter-output] cache inactive session=${message.sessionId ?? ''} active=${activeSessionId ?? ''}',
      );
    }
    final incomingRequestId = _payloadStringValue(payload['requestId']);
    final activeRequestId = _activeBufferRequestBySession[sessionId];
    if (payload['buffer'] == true &&
        activeRequestId != null &&
        incomingRequestId != null &&
        incomingRequestId != activeRequestId) {
      CoduxLog.debug(
        '[codux-flutter-output] skip stale buffer request=$incomingRequestId active=$activeRequestId session=$sessionId',
      );
      return const [];
    }

    final assembly = _assembler.accept(sessionId: sessionId, payload: payload);
    if (!assembly.ready) {
      CoduxLog.debug(
        '[codux-flutter-output] buffer chunk progress=${assembly.progress ?? 0} session=$sessionId',
      );
      if (assembly.progress == null) return const [];
      if (!isActiveSession) return const [];
      return [
        RemoteTerminalOutputEffect.loading(
          loading: true,
          phase: RemoteTerminalBufferPhase.receiving,
          progress: assembly.progress,
        ),
      ];
    }

    payload = assembly.payload ?? payload;
    final decoded = decodeTerminalOutputPayload(payload);
    final raw = decoded.data;
    final isBuffer = decoded.isBuffer;
    final outputSeq = _intPayloadValue(payload['outputSeq']);
    if (isBuffer) {
      CoduxLog.info(
        '[codux-flutter-output] buffer bytes=${raw.codeUnits.length} offset=${decoded.offset ?? 0} length=${decoded.bufferLength ?? 0} truncated=${decoded.truncated} seq=${outputSeq ?? 0} session=$sessionId',
      );
    }

    final ptySession = _ptySessions.session(sessionId);
    if (!replayingHeldLive &&
        !isBuffer &&
        ptySession.holdLive(sequence: outputSeq, output: message)) {
      CoduxLog.debug(
        '[codux-flutter-output] hold live output before snapshot seq=${outputSeq ?? 0} session=$sessionId',
      );
      return [
        RemoteTerminalOutputEffect.ack(
          sessionId: sessionId,
          outputSeq: outputSeq,
          bufferLength: decoded.bufferLength,
        ),
      ];
    }
    if (!isActiveSession && !isBuffer) {
      final resync = observeTerminalOutputForResync(
        sequencer: _sequencer,
        sessionId: sessionId,
        isBuffer: false,
        outputSeq: outputSeq,
        offset: null,
      );
      if (resync.render && raw.isNotEmpty) {
        _appendLiveToSession(sessionId, raw, decoded.bufferLength, resync.ack);
      }
      return [
        RemoteTerminalOutputEffect.ack(
          sessionId: sessionId,
          outputSeq: resync.ack,
          bufferLength: decoded.bufferLength,
        ),
      ];
    }

    final resync = observeTerminalOutputForResync(
      sequencer: _sequencer,
      sessionId: sessionId,
      isBuffer: isBuffer,
      outputSeq: outputSeq,
      offset: decoded.offset,
      resetsSequence: decoded.tail || decoded.screenSnapshot,
    );
    if (!resync.render && !resync.requestFullBuffer) {
      CoduxLog.debug(
        '[codux-flutter-output] drop duplicate seq=${resync.ack} session=$sessionId',
      );
      return [
        RemoteTerminalOutputEffect.ack(
          sessionId: sessionId,
          outputSeq: resync.ack,
          bufferLength: decoded.bufferLength,
        ),
      ];
    }
    if (resync.requestFullBuffer) {
      CoduxLog.warn(
        '[codux-flutter-output] gap seq=${resync.ack} session=$sessionId',
      );
      return [
        RemoteTerminalOutputEffect.ack(
          sessionId: sessionId,
          outputSeq: resync.ack,
          bufferLength: decoded.bufferLength,
        ),
        _prepareFullBufferRequest(sessionId),
      ];
    }

    CoduxLog.debug(
      '[codux-flutter-output] bytes=${raw.codeUnits.length} buffer=$isBuffer session=${message.sessionId ?? ''}',
    );

    final effects = <RemoteTerminalOutputEffect>[];
    var heldLive = const <RelayEnvelope>[];
    var skipBufferTailWrite = false;

    if (isBuffer) {
      final offset = decoded.offset ?? 0;
      final isPagedSnapshot =
          decoded.screenSnapshot ||
          decoded.tail ||
          ptySession.awaitingSnapshot ||
          offset == 0;
      var renderData = raw;
      if (isPagedSnapshot) {
        if (!decoded.tail && !decoded.screenSnapshot) {
          final page = ptySession.acceptSnapshotPage(
            data: raw,
            offset: offset,
            bufferLength: decoded.bufferLength,
            truncated: decoded.truncated,
          );
          if (!page.accepted) {
            if (!isActiveSession) {
              _assembler.remove(sessionId);
              _activeBufferRequestBySession.remove(sessionId);
              return [
                RemoteTerminalOutputEffect.ack(
                  sessionId: sessionId,
                  outputSeq: resync.ack,
                  bufferLength: decoded.bufferLength,
                ),
              ];
            }
            effects.add(
              RemoteTerminalOutputEffect.ack(
                sessionId: sessionId,
                outputSeq: resync.ack,
                bufferLength: decoded.bufferLength,
              ),
            );
            effects.add(_prepareFullBufferRequest(sessionId));
            return effects;
          }
          if (!page.ready) {
            _setSessionBufferLength(sessionId, page.nextOffset);
            if (!isActiveSession) {
              effects.add(
                RemoteTerminalOutputEffect.ack(
                  sessionId: sessionId,
                  outputSeq: resync.ack,
                  bufferLength: decoded.bufferLength,
                ),
              );
              return effects;
            }
            effects
              ..add(RemoteTerminalOutputEffect.markBufferReceived(sessionId))
              ..add(
                RemoteTerminalOutputEffect.loading(
                  loading: true,
                  phase: RemoteTerminalBufferPhase.receiving,
                  progress: page.progress,
                ),
              )
              ..add(
                RemoteTerminalOutputEffect.requestBufferPage(
                  sessionId: sessionId,
                  offset: page.nextOffset,
                ),
              )
              ..add(
                RemoteTerminalOutputEffect.ack(
                  sessionId: sessionId,
                  outputSeq: resync.ack,
                  bufferLength: decoded.bufferLength,
                ),
              );
            return effects;
          }
          renderData = page.data;
        }
        skipBufferTailWrite = true;
        heldLive = _replaceSessionFromSnapshot(
          sessionId,
          renderData,
          decoded.screenSnapshot
              ? renderData.runes.length
              : decoded.bufferLength,
          outputSeq,
        );
      }

      final localCacheEmpty = (_ptySessions.content(sessionId) ?? '').isEmpty;
      if (!isPagedSnapshot && localCacheEmpty) {
        _setSessionBufferLength(sessionId, 0);
        if (!isActiveSession) {
          effects.add(
            RemoteTerminalOutputEffect.ack(
              sessionId: sessionId,
              outputSeq: resync.ack,
              bufferLength: decoded.bufferLength,
            ),
          );
          return effects;
        }
        effects
          ..add(
            RemoteTerminalOutputEffect.ack(
              sessionId: sessionId,
              outputSeq: resync.ack,
              bufferLength: decoded.bufferLength,
            ),
          )
          ..add(_prepareFullBufferRequest(sessionId));
        return effects;
      }

      if (isPagedSnapshot || _ptySessions.content(sessionId) == null) {
        if (!isPagedSnapshot) {
          _replaceSessionFromSnapshot(
            sessionId,
            renderData,
            decoded.screenSnapshot
                ? renderData.runes.length
                : decoded.bufferLength,
            outputSeq,
          );
        }
        _activeBufferRequestBySession.remove(sessionId);
        if (isActiveSession) {
          effects.add(
            RemoteTerminalOutputEffect.renderSnapshot(
              sessionId: sessionId,
              data: renderData,
            ),
          );
        }
      } else {
        _appendLiveToSession(sessionId, raw, decoded.bufferLength, resync.ack);
        _activeBufferRequestBySession.remove(sessionId);
        if (isActiveSession) {
          effects.add(RemoteTerminalOutputEffect.markBufferReceived(sessionId));
        }
      }
    } else if (raw.isNotEmpty && isActiveSession) {
      effects.add(RemoteTerminalOutputEffect.loading(loading: false));
    }

    if (raw.isNotEmpty) {
      if (!isBuffer) {
        _appendLiveToSession(sessionId, raw, decoded.bufferLength, resync.ack);
        if (isActiveSession) {
          effects.add(
            RemoteTerminalOutputEffect.writeData(
              sessionId: sessionId,
              data: raw,
              replayingBuffer: false,
            ),
          );
        }
      } else if (!skipBufferTailWrite &&
          (decoded.offset ?? 0) > 0 &&
          !ptySession.awaitingSnapshot) {
        if (isActiveSession) {
          effects.add(
            RemoteTerminalOutputEffect.writeData(
              sessionId: sessionId,
              data: raw,
              replayingBuffer: true,
            ),
          );
        }
      }
    }

    effects.add(
      RemoteTerminalOutputEffect.ack(
        sessionId: sessionId,
        outputSeq: resync.ack,
        bufferLength: decoded.bufferLength,
      ),
    );

    if (isBuffer && heldLive.isNotEmpty) {
      for (final held in heldLive) {
        effects.addAll(
          _accept(
            held,
            activeSessionId: activeSessionId,
            replayingHeldLive: true,
          ),
        );
      }
    }

    return effects;
  }

  RemoteTerminalOutputEffect _prepareFullBufferRequest(String sessionId) {
    _setSessionBufferLength(sessionId, 0);
    _activeBufferRequestBySession.remove(sessionId);
    _ptySessions.session(sessionId).requireSnapshot();
    return RemoteTerminalOutputEffect.requestFullBuffer(sessionId);
  }

  List<RelayEnvelope> _replaceSessionFromSnapshot(
    String sessionId,
    String data,
    int? bufferLength,
    int? outputSeq,
  ) {
    return _ptySessions
        .session(sessionId)
        .replaceFromSnapshot(
          content: data,
          bufferLength: bufferLength,
          sequence: outputSeq,
        );
  }

  void _appendLiveToSession(
    String sessionId,
    String data,
    int? bufferLength,
    int? outputSeq,
  ) {
    _ptySessions
        .session(sessionId)
        .appendLive(
          data: data,
          bufferLength: bufferLength,
          sequence: outputSeq,
        );
  }

  void _setSessionBufferLength(String sessionId, int bufferLength) {
    _ptySessions
        .session(sessionId)
        .appendLive(data: '', bufferLength: bufferLength, sequence: null);
  }
}

int? _intPayloadValue(Object? value) {
  if (value is int) return value;
  if (value is num) return value.toInt();
  return int.tryParse('${value ?? ''}');
}

String? _payloadStringValue(Object? value) {
  final text = value?.toString().trim();
  return text == null || text.isEmpty ? null : text;
}
