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

class RemoteLatencyState {
  const RemoteLatencyState({
    this.pendingPingId,
    this.pingSentAt,
    this.missCount = 0,
    this.latencyMs,
  });

  final String? pendingPingId;
  final DateTime? pingSentAt;
  final int missCount;
  final int? latencyMs;

  bool get hasPendingPing => pendingPingId != null;

  RemoteLatencyState copyWith({
    Object? pendingPingId = _unset,
    Object? pingSentAt = _unset,
    int? missCount,
    Object? latencyMs = _unset,
  }) {
    return RemoteLatencyState(
      pendingPingId: pendingPingId == _unset
          ? this.pendingPingId
          : pendingPingId as String?,
      pingSentAt: pingSentAt == _unset
          ? this.pingSentAt
          : pingSentAt as DateTime?,
      missCount: missCount ?? this.missCount,
      latencyMs: latencyMs == _unset ? this.latencyMs : latencyMs as int?,
    );
  }
}

class RemoteTransportPing {
  const RemoteTransportPing({required this.id, required this.sentAt});

  final String id;
  final DateTime sentAt;
}

class RemoteTransportPongResult {
  const RemoteTransportPongResult({
    required this.accepted,
    required this.state,
    this.latencyMs,
  });

  final bool accepted;
  final RemoteLatencyState state;
  final int? latencyMs;
}

class RemoteTransportStateController {
  RemoteTransportStateController({DateTime Function()? now})
    : _now = now ?? DateTime.now;

  final DateTime Function() _now;
  RemoteLatencyState latency = const RemoteLatencyState();
  int _pingSeq = 0;

  RemoteTransportPing? beginPing({
    required bool transportReady,
    required bool transportConnected,
    required bool hasDevice,
  }) {
    if (!transportReady || !transportConnected || !hasDevice) return null;
    if (latency.hasPendingPing) return null;
    final sentAt = _now();
    final ping = RemoteTransportPing(
      id: '${sentAt.microsecondsSinceEpoch}-${++_pingSeq}',
      sentAt: sentAt,
    );
    latency = latency.copyWith(pendingPingId: ping.id, pingSentAt: sentAt);
    return ping;
  }

  void cancelPendingPing() {
    latency = latency.copyWith(pendingPingId: null, pingSentAt: null);
  }

  void clearLatency() {
    latency = const RemoteLatencyState();
  }

  void pauseLatency() {
    latency = latency.copyWith(pendingPingId: null, pingSentAt: null);
  }

  RemoteTransportPongResult recordPong(Object? payload) {
    final sentAt = latency.pingSentAt;
    if (sentAt == null) {
      return RemoteTransportPongResult(accepted: false, state: latency);
    }
    if (payload is Map && latency.pendingPingId != null) {
      final id = payload['id']?.toString();
      if (id != null && id != latency.pendingPingId) {
        return RemoteTransportPongResult(accepted: false, state: latency);
      }
    }
    final nextLatency = _now().difference(sentAt).inMilliseconds;
    final next = latency.copyWith(
      pendingPingId: null,
      pingSentAt: null,
      missCount: 0,
      latencyMs: nextLatency > 0 && nextLatency <= 60000
          ? nextLatency
          : latency.latencyMs,
    );
    latency = next;
    return RemoteTransportPongResult(
      accepted: true,
      state: latency,
      latencyMs: next.latencyMs,
    );
  }

  int recordPingTimeoutMiss() {
    latency = latency.copyWith(
      pendingPingId: null,
      pingSentAt: null,
      missCount: latency.missCount + 1,
    );
    return latency.missCount;
  }

  void markResponsive() {
    latency = latency.copyWith(missCount: 0);
  }
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

const Object _unset = Object();
