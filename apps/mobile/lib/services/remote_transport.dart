import 'dart:async';
import 'dart:convert';
import 'dart:io';

import 'package:codux_protocol_ffi/codux_protocol_ffi.dart'
    as codux_protocol_ffi;
import 'package:flutter_webrtc/flutter_webrtc.dart';

import '../models/remote_models.dart';
import 'remote_protocol.dart';

typedef RemoteTransportStateHandler = void Function(String state);
typedef RemoteTransportEnvelopeHandler =
    void Function(Map<String, dynamic> envelope);
typedef RemoteTransportFactory = RemoteTransport Function(StoredDevice device);

abstract interface class RemoteTransport {
  String get kind;
  set onState(RemoteTransportStateHandler? handler);
  set onEnvelope(RemoteTransportEnvelopeHandler? handler);
  Future<void> connect(StoredDevice device);
  Future<bool> send(Map<String, dynamic> envelope);
  Future<void> close();
}

RemoteTransport createRemoteTransport(StoredDevice device) {
  return RustControllerTransport();
}

class RustControllerTransport implements RemoteTransport {
  codux_protocol_ffi.ControllerTransportHandle? _handle;
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
    final handle = codux_protocol_ffi.ControllerTransportHandle.connect(config);
    if (handle == null) {
      _onState?.call('failed:transport-connect');
      throw StateError('Failed to connect remote transport');
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
      final event = handle.pollEvent();
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
      }
    }
  }
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

class WebSocketRelayTransport implements RemoteTransport {
  WebSocket? _socket;
  RemoteTransportStateHandler? _onState;
  RemoteTransportEnvelopeHandler? _onEnvelope;

  @override
  String get kind => RemoteTransportKind.websocketRelay;

  @override
  set onState(RemoteTransportStateHandler? handler) => _onState = handler;

  @override
  set onEnvelope(RemoteTransportEnvelopeHandler? handler) =>
      _onEnvelope = handler;

  @override
  Future<void> connect(StoredDevice device) async {
    await close();
    final url = _clientWebSocketUri(device);
    _onState?.call('connecting');
    final socket = await WebSocket.connect(url.toString());
    _socket = socket;
    _onState?.call('connected:path=relay');
    unawaited(_readLoop(socket));
  }

  @override
  Future<bool> send(Map<String, dynamic> envelope) async {
    final socket = _socket;
    if (socket == null) return false;
    socket.add(jsonEncode(envelope));
    return true;
  }

  @override
  Future<void> close() async {
    final socket = _socket;
    _socket = null;
    await socket?.close();
  }

  Future<void> _readLoop(WebSocket socket) async {
    try {
      await for (final message in socket) {
        if (!identical(socket, _socket)) return;
        if (message is! String) continue;
        final decoded = jsonDecode(message);
        if (decoded is Map<String, dynamic>) {
          _onEnvelope?.call(decoded);
        } else if (decoded is Map) {
          _onEnvelope?.call(Map<String, dynamic>.from(decoded));
        }
      }
      if (identical(socket, _socket)) _onState?.call('closed');
    } catch (error) {
      if (identical(socket, _socket)) _onState?.call('failed:$error');
    }
  }

  Uri _clientWebSocketUri(StoredDevice device) {
    final candidate =
        device.transportByKind(RemoteTransportKind.websocketRelay) ??
        device.transportByKind(
          remotePreferredTransportKind(device.transports, pairing: false),
        );
    if (candidate == null) {
      throw StateError('Missing relay transport candidate');
    }
    return Uri.parse(
      remoteTransportClientWebSocketUrl(
        base: candidate.url,
        hostId: device.hostId,
        deviceId: device.deviceId,
        token: device.token,
      ),
    );
  }
}

class WebRtcTransport implements RemoteTransport {
  final WebSocketRelayTransport _relay = WebSocketRelayTransport();
  RemoteTransportStateHandler? _onState;
  RemoteTransportEnvelopeHandler? _onEnvelope;
  RTCPeerConnection? _peerConnection;
  RTCDataChannel? _dataChannel;
  bool _closed = false;
  bool _directReady = false;
  bool _relayReady = false;

  @override
  String get kind => RemoteTransportKind.webRtc;

  @override
  set onState(RemoteTransportStateHandler? handler) => _onState = handler;

  @override
  set onEnvelope(RemoteTransportEnvelopeHandler? handler) =>
      _onEnvelope = handler;

  @override
  Future<void> connect(StoredDevice device) async {
    await close();
    _closed = false;
    _directReady = false;
    _relayReady = false;
    _relay
      ..onState = _handleRelayState
      ..onEnvelope = _handleRelayEnvelope;
    await _relay.connect(device);
    unawaited(_startPeer(device));
  }

  @override
  Future<bool> send(Map<String, dynamic> envelope) async {
    final dataChannel = _dataChannel;
    if (_directReady &&
        dataChannel != null &&
        dataChannel.state == RTCDataChannelState.RTCDataChannelOpen) {
      await dataChannel.send(RTCDataChannelMessage(jsonEncode(envelope)));
      return true;
    }
    return _relay.send(envelope);
  }

  @override
  Future<void> close() async {
    _closed = true;
    _directReady = false;
    _relayReady = false;
    final channel = _dataChannel;
    final peerConnection = _peerConnection;
    _dataChannel = null;
    _peerConnection = null;
    await channel?.close();
    await peerConnection?.close();
    await peerConnection?.dispose();
    await _relay.close();
  }

  void _handleRelayState(String state) {
    if (state == 'connected:path=relay') {
      _relayReady = true;
      if (!_directReady) _onState?.call(state);
      return;
    }
    if (state == 'closed' || state.startsWith('failed:')) {
      _relayReady = false;
      if (!_directReady) _onState?.call(state);
      return;
    }
    if (!_directReady) _onState?.call(state);
  }

  Future<void> _startPeer(StoredDevice device) async {
    try {
      final peerConnection = await createPeerConnection({
        'iceServers': _iceServers(device),
        'sdpSemantics': 'unified-plan',
      });
      if (_closed) {
        await peerConnection.close();
        await peerConnection.dispose();
        return;
      }
      _peerConnection = peerConnection;
      peerConnection.onIceCandidate = (candidate) {
        if (_closed || candidate.candidate == null) return;
        unawaited(
          _relay.send({
            'type': 'webrtc.ice',
            'deviceId': device.deviceId,
            'payload': {'candidate': candidate.toMap()},
          }),
        );
      };
      peerConnection.onConnectionState = (state) {
        if (state == RTCPeerConnectionState.RTCPeerConnectionStateConnected) {
          _markDirectReady();
        } else if (state ==
                RTCPeerConnectionState.RTCPeerConnectionStateFailed ||
            state == RTCPeerConnectionState.RTCPeerConnectionStateClosed) {
          _markRelayFallback();
        }
      };
      final dataChannel = await peerConnection.createDataChannel(
        'codux',
        RTCDataChannelInit()..ordered = true,
      );
      _installDataChannel(dataChannel);
      final offer = await peerConnection.createOffer({});
      await peerConnection.setLocalDescription(offer);
      await _relay.send({
        'type': 'webrtc.offer',
        'deviceId': device.deviceId,
        'payload': {'description': offer.toMap()},
      });
    } catch (error) {
      _markRelayFallback();
    }
  }

  void _installDataChannel(RTCDataChannel channel) {
    _dataChannel = channel;
    channel.onDataChannelState = (state) {
      if (state == RTCDataChannelState.RTCDataChannelOpen) {
        _markDirectReady();
      } else if (state == RTCDataChannelState.RTCDataChannelClosed ||
          state == RTCDataChannelState.RTCDataChannelClosing) {
        _markRelayFallback();
      }
    };
    channel.onMessage = (message) {
      if (message.isBinary) return;
      final decoded = jsonDecode(message.text);
      if (decoded is Map<String, dynamic>) {
        _onEnvelope?.call(decoded);
      } else if (decoded is Map) {
        _onEnvelope?.call(Map<String, dynamic>.from(decoded));
      }
    };
  }

  void _handleRelayEnvelope(Map<String, dynamic> envelope) {
    final type = '${envelope['type'] ?? ''}';
    if (type == 'webrtc.answer') {
      unawaited(_handleAnswer(envelope['payload']));
      return;
    }
    if (type == 'webrtc.ice') {
      unawaited(_handleIce(envelope['payload']));
      return;
    }
    _onEnvelope?.call(envelope);
  }

  Future<void> _handleAnswer(Object? payload) async {
    final peerConnection = _peerConnection;
    final description = _descriptionFromPayload(payload);
    if (peerConnection == null || description == null) return;
    try {
      await peerConnection.setRemoteDescription(description);
    } catch (_) {
      _markRelayFallback();
    }
  }

  Future<void> _handleIce(Object? payload) async {
    final peerConnection = _peerConnection;
    final candidate = _candidateFromPayload(payload);
    if (peerConnection == null || candidate == null) return;
    try {
      await peerConnection.addCandidate(candidate);
    } catch (_) {}
  }

  void _markDirectReady() {
    if (_closed || _directReady) return;
    _directReady = true;
    _onState?.call('connected:path=direct');
  }

  void _markRelayFallback() {
    if (_closed) return;
    if (_directReady) {
      _directReady = false;
      _onState?.call(_relayReady ? 'connected:path=relay' : 'closed');
    }
  }
}

RTCSessionDescription? _descriptionFromPayload(Object? payload) {
  if (payload is! Map) return null;
  final description = payload['description'];
  if (description is! Map) return null;
  final sdp = description['sdp']?.toString();
  final type = description['type']?.toString();
  if (sdp == null || type == null) return null;
  return RTCSessionDescription(sdp, type);
}

RTCIceCandidate? _candidateFromPayload(Object? payload) {
  if (payload is! Map) return null;
  final candidate = payload['candidate'];
  if (candidate is! Map) return null;
  final text = candidate['candidate']?.toString();
  if (text == null || text.isEmpty) return null;
  final sdpMid = candidate['sdpMid']?.toString();
  final sdpMLineIndex = candidate['sdpMLineIndex'] is num
      ? (candidate['sdpMLineIndex'] as num).toInt()
      : int.tryParse('${candidate['sdpMLineIndex'] ?? ''}');
  return RTCIceCandidate(text, sdpMid, sdpMLineIndex);
}

List<Map<String, dynamic>> _iceServers(StoredDevice device) {
  final webRtc = device.transportByKind(RemoteTransportKind.webRtc);
  final configured = webRtc?.iceServers
      .map((server) => {'urls': server.urls})
      .where((server) => (server['urls'] as List).isNotEmpty)
      .toList();
  if (configured != null && configured.isNotEmpty) return configured;
  return remoteTransportDefaultIceServers();
}
