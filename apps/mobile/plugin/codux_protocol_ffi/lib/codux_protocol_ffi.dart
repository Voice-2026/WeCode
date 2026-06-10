import 'dart:convert';
import 'dart:ffi';
import 'dart:io';

import 'package:ffi/ffi.dart';

const String _libName = 'codux_protocol_ffi';

final DynamicLibrary _dylib = _loadLibrary();

DynamicLibrary _loadLibrary() {
  if (Platform.isMacOS || Platform.isIOS) {
    final process = DynamicLibrary.process();
    if (_hasRequiredSymbols(process)) return process;
    if (!Platform.isIOS) {
      final localPath = _localDevelopmentLibraryPath();
      if (localPath != null) return DynamicLibrary.open(localPath);
    }
    return process;
  }
  if (Platform.isAndroid || Platform.isLinux) {
    return DynamicLibrary.open('lib$_libName.so');
  }
  if (Platform.isWindows) {
    return DynamicLibrary.open('$_libName.dll');
  }
  throw UnsupportedError('Unsupported platform: ${Platform.operatingSystem}');
}

bool _hasRequiredSymbols(DynamicLibrary library) {
  try {
    library.lookup<NativeFunction<Pointer<Utf8> Function()>>(
      'codux_protocol_version',
    );
    library
        .lookup<NativeFunction<Pointer<Void> Function(Pointer<Utf8>, Int64)>>(
          'codux_terminal_session_new',
        );
    library.lookup<
      NativeFunction<
        Pointer<Utf8> Function(Pointer<Void>, Pointer<Utf8>, Int64, Int64)
      >
    >('codux_terminal_session_replace_from_baseline_json');
    library.lookup<NativeFunction<Pointer<Utf8> Function(Pointer<Utf8>)>>(
      'codux_protocol_transport_kind',
    );
    library.lookup<
      NativeFunction<Pointer<Utf8> Function(Pointer<Utf8>, Pointer<Utf8>)>
    >('codux_transport_pairing_ticket_url');
    library.lookup<NativeFunction<Pointer<Void> Function()>>(
      'codux_terminal_output_sequencer_new',
    );
    return true;
  } catch (_) {
    return false;
  }
}

String? _localDevelopmentLibraryPath() {
  final candidates = [
    '../../target/debug/lib$_libName.dylib',
    '../../target/release/lib$_libName.dylib',
    '../target/debug/lib$_libName.dylib',
    '../target/release/lib$_libName.dylib',
    'target/debug/lib$_libName.dylib',
    'target/release/lib$_libName.dylib',
  ];
  for (final candidate in candidates) {
    final file = File(candidate);
    if (file.existsSync()) return file.absolute.path;
  }
  return null;
}

final _version = _dylib
    .lookupFunction<Pointer<Utf8> Function(), Pointer<Utf8> Function()>(
      'codux_protocol_version',
    );
final _messageType = _dylib
    .lookupFunction<
      Pointer<Utf8> Function(Pointer<Utf8>),
      Pointer<Utf8> Function(Pointer<Utf8>)
    >('codux_protocol_message_type');
final _resourceType = _dylib
    .lookupFunction<
      Pointer<Utf8> Function(Pointer<Utf8>),
      Pointer<Utf8> Function(Pointer<Utf8>)
    >('codux_protocol_resource_type');
final _transportKind = _dylib
    .lookupFunction<
      Pointer<Utf8> Function(Pointer<Utf8>),
      Pointer<Utf8> Function(Pointer<Utf8>)
    >('codux_protocol_transport_kind');
final _relayBlocks = _dylib
    .lookupFunction<Bool Function(Pointer<Utf8>), bool Function(Pointer<Utf8>)>(
      'codux_protocol_relay_blocks_message',
    );
final _transportServerUrl = _dylib
    .lookupFunction<
      Pointer<Utf8> Function(Pointer<Utf8>),
      Pointer<Utf8> Function(Pointer<Utf8>)
    >('codux_transport_server_url');
final _transportPairingTicketUrl = _dylib
    .lookupFunction<
      Pointer<Utf8> Function(Pointer<Utf8>, Pointer<Utf8>),
      Pointer<Utf8> Function(Pointer<Utf8>, Pointer<Utf8>)
    >('codux_transport_pairing_ticket_url');
final _transportPairingWebSocketUrl = _dylib
    .lookupFunction<
      Pointer<Utf8> Function(Pointer<Utf8>, Pointer<Utf8>, Pointer<Utf8>),
      Pointer<Utf8> Function(Pointer<Utf8>, Pointer<Utf8>, Pointer<Utf8>)
    >('codux_transport_pairing_websocket_url');
final _transportClientWebSocketUrl = _dylib
    .lookupFunction<
      Pointer<Utf8> Function(
        Pointer<Utf8>,
        Pointer<Utf8>,
        Pointer<Utf8>,
        Pointer<Utf8>,
      ),
      Pointer<Utf8> Function(
        Pointer<Utf8>,
        Pointer<Utf8>,
        Pointer<Utf8>,
        Pointer<Utf8>,
      )
    >('codux_transport_client_websocket_url');
final _transportDefaultIceServersJson = _dylib
    .lookupFunction<Pointer<Utf8> Function(), Pointer<Utf8> Function()>(
      'codux_transport_default_ice_servers_json',
    );
final _transportPreferredKind = _dylib
    .lookupFunction<
      Pointer<Utf8> Function(Pointer<Utf8>, Bool),
      Pointer<Utf8> Function(Pointer<Utf8>, bool)
    >('codux_transport_preferred_kind');
final _resourceSubscribeJson = _dylib
    .lookupFunction<
      Pointer<Utf8> Function(
        Pointer<Utf8>,
        Pointer<Utf8>,
        Pointer<Utf8>,
        Bool,
        Int32,
        Int32,
      ),
      Pointer<Utf8> Function(
        Pointer<Utf8>,
        Pointer<Utf8>,
        Pointer<Utf8>,
        bool,
        int,
        int,
      )
    >('codux_protocol_resource_subscribe_json');
final _resourceUnsubscribeJson = _dylib
    .lookupFunction<
      Pointer<Utf8> Function(Pointer<Utf8>, Pointer<Utf8>, Pointer<Utf8>),
      Pointer<Utf8> Function(Pointer<Utf8>, Pointer<Utf8>, Pointer<Utf8>)
    >('codux_protocol_resource_unsubscribe_json');
final _terminalSessionNew = _dylib
    .lookupFunction<
      Pointer<Void> Function(Pointer<Utf8>, Int64),
      Pointer<Void> Function(Pointer<Utf8>, int)
    >('codux_terminal_session_new');
final _terminalSessionFree = _dylib
    .lookupFunction<Void Function(Pointer<Void>), void Function(Pointer<Void>)>(
      'codux_terminal_session_free',
    );
final _terminalSessionSnapshotJson = _dylib
    .lookupFunction<
      Pointer<Utf8> Function(Pointer<Void>),
      Pointer<Utf8> Function(Pointer<Void>)
    >('codux_terminal_session_snapshot_json');
final _terminalSessionContent = _dylib
    .lookupFunction<
      Pointer<Utf8> Function(Pointer<Void>),
      Pointer<Utf8> Function(Pointer<Void>)
    >('codux_terminal_session_content');
final _terminalSessionBufferLength = _dylib
    .lookupFunction<Int64 Function(Pointer<Void>), int Function(Pointer<Void>)>(
      'codux_terminal_session_buffer_length',
    );
final _terminalSessionSequence = _dylib
    .lookupFunction<Int64 Function(Pointer<Void>), int Function(Pointer<Void>)>(
      'codux_terminal_session_sequence',
    );
final _terminalSessionIsRestoringBaseline = _dylib
    .lookupFunction<Bool Function(Pointer<Void>), bool Function(Pointer<Void>)>(
      'codux_terminal_session_is_restoring_baseline',
    );
final _terminalSessionRequireBaseline = _dylib
    .lookupFunction<Void Function(Pointer<Void>), void Function(Pointer<Void>)>(
      'codux_terminal_session_require_baseline',
    );
final _terminalSessionResetTransient = _dylib
    .lookupFunction<
      Void Function(Pointer<Void>, Bool),
      void Function(Pointer<Void>, bool)
    >('codux_terminal_session_reset_transient');
final _terminalSessionSetSequence = _dylib
    .lookupFunction<
      Void Function(Pointer<Void>, Int64),
      void Function(Pointer<Void>, int)
    >('codux_terminal_session_set_sequence');
final _terminalSessionHoldLiveToken = _dylib
    .lookupFunction<
      Bool Function(Pointer<Void>, Int64, Int64),
      bool Function(Pointer<Void>, int, int)
    >('codux_terminal_session_hold_live_token');
final _terminalSessionAcceptBaselinePageJson = _dylib
    .lookupFunction<
      Pointer<Utf8> Function(Pointer<Void>, Pointer<Utf8>, Int64, Int64, Bool),
      Pointer<Utf8> Function(Pointer<Void>, Pointer<Utf8>, int, int, bool)
    >('codux_terminal_session_accept_baseline_page_json');
final _terminalSessionReplaceFromBaselineJson = _dylib
    .lookupFunction<
      Pointer<Utf8> Function(Pointer<Void>, Pointer<Utf8>, Int64, Int64),
      Pointer<Utf8> Function(Pointer<Void>, Pointer<Utf8>, int, int)
    >('codux_terminal_session_replace_from_baseline_json');
final _terminalSessionAppendLive = _dylib
    .lookupFunction<
      Void Function(Pointer<Void>, Pointer<Utf8>, Int64, Int64),
      void Function(Pointer<Void>, Pointer<Utf8>, int, int)
    >('codux_terminal_session_append_live');
final _terminalSessionClear = _dylib
    .lookupFunction<Void Function(Pointer<Void>), void Function(Pointer<Void>)>(
      'codux_terminal_session_clear',
    );
final _terminalOutputSequencerNew = _dylib
    .lookupFunction<Pointer<Void> Function(), Pointer<Void> Function()>(
      'codux_terminal_output_sequencer_new',
    );
final _terminalOutputSequencerFree = _dylib
    .lookupFunction<Void Function(Pointer<Void>), void Function(Pointer<Void>)>(
      'codux_terminal_output_sequencer_free',
    );
final _terminalOutputSequencerSequenceFor = _dylib
    .lookupFunction<
      Int64 Function(Pointer<Void>, Pointer<Utf8>),
      int Function(Pointer<Void>, Pointer<Utf8>)
    >('codux_terminal_output_sequencer_sequence_for');
final _terminalOutputSequencerIsResyncing = _dylib
    .lookupFunction<
      Bool Function(Pointer<Void>, Pointer<Utf8>),
      bool Function(Pointer<Void>, Pointer<Utf8>)
    >('codux_terminal_output_sequencer_is_resyncing');
final _terminalOutputSequencerObserveJson = _dylib
    .lookupFunction<
      Pointer<Utf8> Function(
        Pointer<Void>,
        Pointer<Utf8>,
        Bool,
        Int64,
        Int64,
        Bool,
      ),
      Pointer<Utf8> Function(Pointer<Void>, Pointer<Utf8>, bool, int, int, bool)
    >('codux_terminal_output_sequencer_observe_json');
final _terminalOutputSequencerRemove = _dylib
    .lookupFunction<
      Void Function(Pointer<Void>, Pointer<Utf8>),
      void Function(Pointer<Void>, Pointer<Utf8>)
    >('codux_terminal_output_sequencer_remove');
final _terminalOutputSequencerReset = _dylib
    .lookupFunction<Void Function(Pointer<Void>), void Function(Pointer<Void>)>(
      'codux_terminal_output_sequencer_reset',
    );
final _stringFree = _dylib
    .lookupFunction<Void Function(Pointer<Utf8>), void Function(Pointer<Utf8>)>(
      'codux_protocol_string_free',
    );

String protocolVersion() => _takeString(_version());

String messageType(String name) {
  final pointer = name.toNativeUtf8();
  try {
    return _takeString(_messageType(pointer));
  } finally {
    malloc.free(pointer);
  }
}

String resourceType(String name) {
  final pointer = name.toNativeUtf8();
  try {
    return _takeString(_resourceType(pointer));
  } finally {
    malloc.free(pointer);
  }
}

String transportKind(String name) {
  final pointer = name.toNativeUtf8();
  try {
    return _takeString(_transportKind(pointer));
  } finally {
    malloc.free(pointer);
  }
}

bool relayBlocksMessage(String kind) {
  final pointer = kind.toNativeUtf8();
  try {
    return _relayBlocks(pointer);
  } finally {
    malloc.free(pointer);
  }
}

String transportServerUrl(String base) {
  final basePtr = base.toNativeUtf8();
  try {
    return _takeString(_transportServerUrl(basePtr));
  } finally {
    malloc.free(basePtr);
  }
}

String transportPairingTicketUrl({
  required String base,
  required String ticket,
}) {
  final basePtr = base.toNativeUtf8();
  final ticketPtr = ticket.toNativeUtf8();
  try {
    return _takeString(_transportPairingTicketUrl(basePtr, ticketPtr));
  } finally {
    malloc.free(basePtr);
    malloc.free(ticketPtr);
  }
}

String transportPairingWebSocketUrl({
  required String base,
  required String hostId,
  required String devicePublicKey,
}) {
  final basePtr = base.toNativeUtf8();
  final hostPtr = hostId.toNativeUtf8();
  final devicePtr = devicePublicKey.toNativeUtf8();
  try {
    return _takeString(
      _transportPairingWebSocketUrl(basePtr, hostPtr, devicePtr),
    );
  } finally {
    malloc.free(basePtr);
    malloc.free(hostPtr);
    malloc.free(devicePtr);
  }
}

String transportClientWebSocketUrl({
  required String base,
  required String hostId,
  required String deviceId,
  String token = '',
}) {
  final basePtr = base.toNativeUtf8();
  final hostPtr = hostId.toNativeUtf8();
  final devicePtr = deviceId.toNativeUtf8();
  final tokenPtr = token.toNativeUtf8();
  try {
    return _takeString(
      _transportClientWebSocketUrl(basePtr, hostPtr, devicePtr, tokenPtr),
    );
  } finally {
    malloc.free(basePtr);
    malloc.free(hostPtr);
    malloc.free(devicePtr);
    malloc.free(tokenPtr);
  }
}

List<Map<String, dynamic>> transportDefaultIceServers() {
  final decoded = _decodeJson(_transportDefaultIceServersJson());
  if (decoded is! List) return const [];
  return [
    for (final item in decoded)
      if (item is Map) Map<String, dynamic>.from(item),
  ];
}

String preferredTransportKind(
  List<Map<String, dynamic>> transports, {
  required bool pairing,
}) {
  final transportsPtr = jsonEncode(transports).toNativeUtf8();
  try {
    return _takeString(_transportPreferredKind(transportsPtr, pairing));
  } finally {
    malloc.free(transportsPtr);
  }
}

Map<String, dynamic> resourceSubscribeEnvelope({
  required String resource,
  String? projectId,
  String? sessionId,
  bool baseline = true,
  int? maxChars,
  int? chunkChars,
}) {
  final resourcePtr = resource.toNativeUtf8();
  final projectPtr = (projectId ?? '').toNativeUtf8();
  final sessionPtr = (sessionId ?? '').toNativeUtf8();
  try {
    return _decodeEnvelope(
      _resourceSubscribeJson(
        resourcePtr,
        projectPtr,
        sessionPtr,
        baseline,
        maxChars ?? 0,
        chunkChars ?? 0,
      ),
    );
  } finally {
    malloc.free(resourcePtr);
    malloc.free(projectPtr);
    malloc.free(sessionPtr);
  }
}

Map<String, dynamic> resourceUnsubscribeEnvelope({
  required String resource,
  String? projectId,
  String? sessionId,
}) {
  final resourcePtr = resource.toNativeUtf8();
  final projectPtr = (projectId ?? '').toNativeUtf8();
  final sessionPtr = (sessionId ?? '').toNativeUtf8();
  try {
    return _decodeEnvelope(
      _resourceUnsubscribeJson(resourcePtr, projectPtr, sessionPtr),
    );
  } finally {
    malloc.free(resourcePtr);
    malloc.free(projectPtr);
    malloc.free(sessionPtr);
  }
}

class TerminalSessionSnapshot {
  const TerminalSessionSnapshot({
    required this.sessionId,
    required this.content,
    required this.bufferLength,
    required this.sequence,
  });

  final String sessionId;
  final String content;
  final int bufferLength;
  final int sequence;

  factory TerminalSessionSnapshot.fromJson(Map<String, dynamic> json) {
    return TerminalSessionSnapshot(
      sessionId: '${json['sessionId'] ?? ''}',
      content: '${json['content'] ?? ''}',
      bufferLength: _jsonInt(json['bufferLength']),
      sequence: _jsonInt(json['sequence']),
    );
  }
}

class TerminalSnapshotPageResult {
  const TerminalSnapshotPageResult({
    required this.accepted,
    required this.duplicate,
    required this.ready,
    required this.data,
    required this.nextOffset,
    required this.progress,
  });

  final bool accepted;
  final bool duplicate;
  final bool ready;
  final String data;
  final int nextOffset;
  final double? progress;

  factory TerminalSnapshotPageResult.fromJson(Map<String, dynamic> json) {
    final progress = json['progress'];
    return TerminalSnapshotPageResult(
      accepted: json['accepted'] == true,
      duplicate: json['duplicate'] == true,
      ready: json['ready'] == true,
      data: '${json['data'] ?? ''}',
      nextOffset: _jsonInt(json['nextOffset']),
      progress: progress is num ? progress.toDouble() : null,
    );
  }
}

typedef TerminalBaselinePageResult = TerminalSnapshotPageResult;

class TerminalCoreSession {
  TerminalCoreSession({required String sessionId, required int maxCachedChars})
    : _handle = _newSession(sessionId, maxCachedChars);

  Pointer<Void> _handle;

  bool get isDisposed => _handle == nullptr;
  String get content => _takeString(_terminalSessionContent(_liveHandle()));
  int get bufferLength => _terminalSessionBufferLength(_liveHandle());
  int get sequence => _terminalSessionSequence(_liveHandle());
  bool get isRestoringBaseline =>
      _terminalSessionIsRestoringBaseline(_liveHandle());

  TerminalSessionSnapshot snapshot() {
    return TerminalSessionSnapshot.fromJson(
      _decodeEnvelope(_terminalSessionSnapshotJson(_liveHandle())),
    );
  }

  void requireBaseline() {
    _terminalSessionRequireBaseline(_liveHandle());
  }

  void resetTransient({bool resetSequence = false}) {
    _terminalSessionResetTransient(_liveHandle(), resetSequence);
  }

  void setSequence(int sequence) {
    _terminalSessionSetSequence(_liveHandle(), sequence);
  }

  bool holdLiveToken({required int? sequence, required int token}) {
    return _terminalSessionHoldLiveToken(_liveHandle(), sequence ?? -1, token);
  }

  TerminalBaselinePageResult acceptBaselinePage({
    required String data,
    required int offset,
    required int? bufferLength,
    required bool truncated,
  }) {
    final dataPtr = data.toNativeUtf8();
    try {
      return TerminalSnapshotPageResult.fromJson(
        _decodeEnvelope(
          _terminalSessionAcceptBaselinePageJson(
            _liveHandle(),
            dataPtr,
            offset,
            bufferLength ?? -1,
            truncated,
          ),
        ),
      );
    } finally {
      malloc.free(dataPtr);
    }
  }

  List<int> replaceFromBaseline({
    required String content,
    required int? bufferLength,
    required int? sequence,
  }) {
    final contentPtr = content.toNativeUtf8();
    try {
      final decoded = _decodeEnvelope(
        _terminalSessionReplaceFromBaselineJson(
          _liveHandle(),
          contentPtr,
          bufferLength ?? -1,
          sequence ?? -1,
        ),
      );
      final tokens = decoded['replayTokens'];
      if (tokens is! List) {
        throw const FormatException(
          'Terminal core FFI did not return replay tokens',
        );
      }
      return [
        for (final token in tokens)
          if (token is num) token.toInt(),
      ];
    } finally {
      malloc.free(contentPtr);
    }
  }

  void appendLive({
    required String data,
    required int? bufferLength,
    required int? sequence,
  }) {
    final dataPtr = data.toNativeUtf8();
    try {
      _terminalSessionAppendLive(
        _liveHandle(),
        dataPtr,
        bufferLength ?? -1,
        sequence ?? -1,
      );
    } finally {
      malloc.free(dataPtr);
    }
  }

  void clear() {
    _terminalSessionClear(_liveHandle());
  }

  void dispose() {
    final handle = _handle;
    if (handle == nullptr) return;
    _handle = nullptr;
    _terminalSessionFree(handle);
  }

  Pointer<Void> _liveHandle() {
    final handle = _handle;
    if (handle == nullptr) {
      throw StateError('TerminalCoreSession is disposed');
    }
    return handle;
  }
}

class TerminalOutputSequenceObservation {
  const TerminalOutputSequenceObservation({
    required this.action,
    required this.previousSeq,
    required this.shouldRender,
  });

  final String action;
  final int previousSeq;
  final bool shouldRender;

  factory TerminalOutputSequenceObservation.fromJson(
    Map<String, dynamic> json,
  ) {
    return TerminalOutputSequenceObservation(
      action: '${json['action'] ?? ''}',
      previousSeq: _jsonInt(json['previousSeq']),
      shouldRender: json['shouldRender'] == true,
    );
  }
}

class TerminalOutputSequencerCore {
  TerminalOutputSequencerCore() : _handle = _newOutputSequencer();

  Pointer<Void> _handle;

  bool get isDisposed => _handle == nullptr;

  int sequenceFor(String sessionId) {
    final sessionPtr = sessionId.toNativeUtf8();
    try {
      return _terminalOutputSequencerSequenceFor(_liveHandle(), sessionPtr);
    } finally {
      malloc.free(sessionPtr);
    }
  }

  bool isResyncing(String sessionId) {
    final sessionPtr = sessionId.toNativeUtf8();
    try {
      return _terminalOutputSequencerIsResyncing(_liveHandle(), sessionPtr);
    } finally {
      malloc.free(sessionPtr);
    }
  }

  TerminalOutputSequenceObservation observe({
    required String sessionId,
    required bool isBuffer,
    required int? outputSeq,
    required int? offset,
    required bool resetsSequence,
  }) {
    final sessionPtr = sessionId.toNativeUtf8();
    try {
      return TerminalOutputSequenceObservation.fromJson(
        _decodeEnvelope(
          _terminalOutputSequencerObserveJson(
            _liveHandle(),
            sessionPtr,
            isBuffer,
            outputSeq ?? -1,
            offset ?? -1,
            resetsSequence,
          ),
        ),
      );
    } finally {
      malloc.free(sessionPtr);
    }
  }

  void remove(String sessionId) {
    final sessionPtr = sessionId.toNativeUtf8();
    try {
      _terminalOutputSequencerRemove(_liveHandle(), sessionPtr);
    } finally {
      malloc.free(sessionPtr);
    }
  }

  void reset() {
    _terminalOutputSequencerReset(_liveHandle());
  }

  void dispose() {
    final handle = _handle;
    if (handle == nullptr) return;
    _handle = nullptr;
    _terminalOutputSequencerFree(handle);
  }

  Pointer<Void> _liveHandle() {
    final handle = _handle;
    if (handle == nullptr) {
      throw StateError('TerminalOutputSequencerCore is disposed');
    }
    return handle;
  }
}

Pointer<Void> _newSession(String sessionId, int maxCachedChars) {
  final sessionPtr = sessionId.toNativeUtf8();
  try {
    final handle = _terminalSessionNew(sessionPtr, maxCachedChars);
    if (handle == nullptr) {
      throw StateError('Failed to create terminal core session');
    }
    return handle;
  } finally {
    malloc.free(sessionPtr);
  }
}

Pointer<Void> _newOutputSequencer() {
  final handle = _terminalOutputSequencerNew();
  if (handle == nullptr) {
    throw StateError('Failed to create terminal output sequencer');
  }
  return handle;
}

Map<String, dynamic> _decodeEnvelope(Pointer<Utf8> pointer) {
  final decoded = _decodeJson(pointer);
  if (decoded is Map<String, dynamic>) return decoded;
  if (decoded is Map) return Map<String, dynamic>.from(decoded);
  throw const FormatException('Protocol FFI did not return a JSON object');
}

Object? _decodeJson(Pointer<Utf8> pointer) {
  final text = _takeString(pointer);
  return jsonDecode(text);
}

String _takeString(Pointer<Utf8> pointer) {
  if (pointer == nullptr) return '';
  try {
    return pointer.toDartString();
  } finally {
    _stringFree(pointer);
  }
}

int _jsonInt(Object? value) {
  if (value is int) return value;
  if (value is num) return value.toInt();
  return int.tryParse('${value ?? ''}') ?? 0;
}
