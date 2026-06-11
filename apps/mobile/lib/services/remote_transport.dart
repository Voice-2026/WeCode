import 'dart:async';
import 'dart:convert';

import 'package:codux_protocol_ffi/codux_protocol_ffi.dart'
    as codux_protocol_ffi;

import '../models/remote_models.dart';
import 'log_service.dart';

typedef RemoteTransportStateHandler = void Function(String state);
typedef RemoteTransportEnvelopeHandler =
    void Function(Map<String, dynamic> envelope);
typedef RemoteTransportFactory = RemoteTransport Function(StoredDevice device);
typedef ControllerTransportHandleFactory =
    ControllerTransportEventHandle? Function(Map<String, dynamic> config);

class RemoteTransportStateEvent {
  const RemoteTransportStateEvent({
    required this.state,
    required this.detail,
    required this.path,
  });

  final String state;
  final String detail;
  final String? path;

  bool get isPathUpdate => state == 'path' || path != null;
  bool get isConnected => state == 'connected';
  bool get isClosed => state == 'failed' || state == 'closed';

  static RemoteTransportStateEvent parse(String rawState) {
    final state = rawState.split(':').first.trim();
    final detail = rawState.length > state.length
        ? rawState.substring(state.length + 1).trim()
        : '';
    return RemoteTransportStateEvent(
      state: state,
      detail: detail,
      path: parseTransportPath(detail),
    );
  }
}

abstract interface class ControllerTransportEventHandle {
  bool get isClosed;
  bool send(Map<String, dynamic> envelope);
  bool reportPingTimeout({required String path});
  bool probePreferredRoute();
  Map<String, dynamic>? pollEvent();
  void close();
}

abstract interface class RemoteTransport {
  String get kind;
  set onState(RemoteTransportStateHandler? handler);
  set onEnvelope(RemoteTransportEnvelopeHandler? handler);
  Future<void> connect(StoredDevice device);
  Future<bool> send(Map<String, dynamic> envelope);
  Future<bool> reportPingTimeout({required String path});
  Future<bool> probePreferredRoute(StoredDevice device);
  Future<void> close();
}

String? parseTransportPath(String detail) {
  for (final part in detail.split(';')) {
    final trimmed = part.trim();
    if (!trimmed.startsWith('path=')) continue;
    final value = trimmed.substring(5).trim();
    if (value == 'direct' ||
        value == 'relay' ||
        value == 'mixed' ||
        value == 'none') {
      return value;
    }
  }
  return null;
}

RemoteTransport createRemoteTransport(StoredDevice device) {
  return RustControllerTransport();
}

class RustControllerTransport implements RemoteTransport {
  RustControllerTransport({ControllerTransportHandleFactory? handleFactory})
    : _handleFactory = handleFactory ?? _connectFfiTransport;

  final ControllerTransportHandleFactory _handleFactory;
  ControllerTransportEventHandle? _handle;
  Timer? _pollTimer;
  RemoteTransportStateHandler? _onState;
  RemoteTransportEnvelopeHandler? _onEnvelope;
  String _kind = RemoteTransportKind.websocketRelay;

  @override
  String get kind => _kind;

  @override
  set onState(RemoteTransportStateHandler? handler) => _onState = handler;

  @override
  set onEnvelope(RemoteTransportEnvelopeHandler? handler) =>
      _onEnvelope = handler;

  @override
  Future<void> connect(StoredDevice device) async {
    await close();
    final config = _controllerTransportConfig(device);
    final summary = codux_protocol_ffi.controllerTransportConfigSummary(config);
    _kind = '${summary['transportKind'] ?? RemoteTransportKind.websocketRelay}';
    _onState?.call('connecting');
    final handle = _handleFactory(config);
    if (handle == null) {
      final error = codux_protocol_ffi.lastError();
      final detail = error.isEmpty ? 'transport-connect' : error;
      _onState?.call('failed:$detail');
      throw StateError('Failed to connect remote transport: $detail');
    }
    _handle = handle;
    _pollTimer = Timer.periodic(const Duration(milliseconds: 16), (_) {
      _drainEvents();
    });
    _drainEvents();
  }

  @override
  Future<bool> send(Map<String, dynamic> envelope) async {
    return _handle?.send(envelope) ?? false;
  }

  @override
  Future<bool> reportPingTimeout({required String path}) async {
    return _handle?.reportPingTimeout(path: path) ?? false;
  }

  @override
  Future<bool> probePreferredRoute(StoredDevice device) async {
    return _handle?.probePreferredRoute() ?? false;
  }

  @override
  Future<void> close() async {
    _pollTimer?.cancel();
    _pollTimer = null;
    final handle = _handle;
    _handle = null;
    handle?.close();
  }

  void _drainEvents() {
    final handle = _handle;
    if (handle == null || handle.isClosed) return;
    for (var i = 0; i < 128; i++) {
      if (!identical(_handle, handle) || handle.isClosed) return;
      Map<String, dynamic>? event;
      try {
        event = handle.pollEvent();
      } on StateError {
        if (identical(_handle, handle)) {
          _pollTimer?.cancel();
          _pollTimer = null;
          _handle = null;
        }
        return;
      }
      if (event == null) return;
      final kind = '${event['kind'] ?? ''}';
      if (kind == 'state') {
        _onState?.call('${event['state'] ?? ''}');
      } else if (kind == 'message') {
        final data = '${event['data'] ?? ''}';
        final decoded = jsonDecode(data);
        if (decoded is Map<String, dynamic>) {
          _onEnvelope?.call(decoded);
        } else if (decoded is Map) {
          _onEnvelope?.call(Map<String, dynamic>.from(decoded));
        }
      } else if (kind == 'log') {
        CoduxLog.info('[codux-flutter-transport] ${event['message'] ?? ''}');
      }
    }
  }
}

ControllerTransportEventHandle? _connectFfiTransport(
  Map<String, dynamic> config,
) {
  final handle = codux_protocol_ffi.ControllerTransportHandle.connect(config);
  return handle == null ? null : _FfiControllerTransportHandle(handle);
}

class _FfiControllerTransportHandle implements ControllerTransportEventHandle {
  _FfiControllerTransportHandle(this._inner);

  final codux_protocol_ffi.ControllerTransportHandle _inner;

  @override
  bool get isClosed => _inner.isClosed;

  @override
  bool send(Map<String, dynamic> envelope) => _inner.send(envelope);

  @override
  bool reportPingTimeout({required String path}) =>
      _inner.reportPingTimeout(path: path);

  @override
  bool probePreferredRoute() => _inner.probePreferredRoute();

  @override
  Map<String, dynamic>? pollEvent() => _inner.pollEvent();

  @override
  void close() => _inner.close();
}

Map<String, dynamic> _controllerTransportConfig(StoredDevice device) {
  final stunUrls = <String>{};
  for (final candidate in device.transports) {
    for (final server in candidate.iceServers) {
      stunUrls.addAll(server.urls);
    }
  }
  return {
    'serverUrl': device.server,
    'hostId': device.hostId,
    'deviceId': device.deviceId,
    'deviceToken': device.token,
    'transports': device.transports.map((item) => item.toJson()).toList(),
    if (stunUrls.isNotEmpty) 'stunUrls': stunUrls.toList(),
  };
}
