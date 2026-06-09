import 'package:codux_flutter/services/remote_transport_state_controller.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('parses transport path updates', () {
    final event = RemoteTransportStateEvent.parse('path:path=direct');

    expect(event.state, 'path');
    expect(event.path, 'direct');
    expect(event.isPathUpdate, isTrue);
    expect(parseTransportPath('foo=bar; path=relay'), 'relay');
    expect(parseTransportPath('path=invalid'), isNull);
  });

  test('records ping and matching pong latency', () {
    var now = DateTime.fromMicrosecondsSinceEpoch(1000000);
    final controller = RemoteTransportStateController(now: () => now);

    final ping = controller.beginPing(
      transportReady: true,
      transportConnected: true,
      hasDevice: true,
    );
    expect(ping, isNotNull);
    expect(
      controller.beginPing(
        transportReady: true,
        transportConnected: true,
        hasDevice: true,
      ),
      isNull,
    );

    now = now.add(const Duration(milliseconds: 120));
    final result = controller.recordPong({'id': ping!.id});

    expect(result.accepted, isTrue);
    expect(result.latencyMs, 120);
    expect(controller.latency.pendingPingId, isNull);
    expect(controller.latency.missCount, 0);
  });

  test('ignores stale pong id', () {
    var now = DateTime.fromMicrosecondsSinceEpoch(1000000);
    final controller = RemoteTransportStateController(now: () => now);
    controller.beginPing(
      transportReady: true,
      transportConnected: true,
      hasDevice: true,
    );
    now = now.add(const Duration(milliseconds: 50));

    final result = controller.recordPong({'id': 'stale'});

    expect(result.accepted, isFalse);
    expect(controller.latency.pendingPingId, isNotNull);
  });

  test('records timeout misses and clears pending ping', () {
    final controller = RemoteTransportStateController();
    controller.beginPing(
      transportReady: true,
      transportConnected: true,
      hasDevice: true,
    );

    expect(controller.recordPingTimeoutMiss(), 1);
    expect(controller.latency.pendingPingId, isNull);
    expect(controller.latency.missCount, 1);
  });
}
