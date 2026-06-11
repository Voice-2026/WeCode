import 'package:codux_flutter/models/remote_models.dart';
import 'package:codux_flutter/services/remote_transport.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('factory creates unified Rust controller transport', () {
    final transport = createRemoteTransport(
      const StoredDevice(
        server: 'https://codux-service.dux.plus/v3',
        hostId: 'host-1',
        deviceId: 'device-1',
        token: 'token-1',
        name: 'Phone',
        transports: [
          RemoteTransportCandidate(
            kind: RemoteTransportKind.websocketRelay,
            url: 'https://codux-service.dux.plus/v3',
          ),
        ],
      ),
    );

    expect(transport, isA<RustControllerTransport>());
    expect(transport.kind, RemoteTransportKind.websocketRelay);
  });

  test(
    'Rust controller transport keeps webRtc as protocol when relay is only signaling',
    () async {
      final transport = RustControllerTransport(
        handleFactory: (_) => _FakeControllerHandle([]),
      );

      await transport.connect(
        const StoredDevice(
          server: 'https://codux-service.dux.plus/v3',
          hostId: 'host-1',
          deviceId: 'device-1',
          token: 'token-1',
          name: 'Phone',
          transports: [
            RemoteTransportCandidate(
              kind: RemoteTransportKind.websocketRelay,
              url: 'https://codux-service.dux.plus/v3',
            ),
            RemoteTransportCandidate(
              kind: RemoteTransportKind.webRtc,
              url: 'https://codux-service.dux.plus/v3',
            ),
          ],
        ),
      );

      expect(transport.kind, RemoteTransportKind.webRtc);
      await transport.close();
    },
  );

  test('drain stops when state callback closes the active handle', () async {
    late _FakeControllerHandle handle;
    final transport = RustControllerTransport(
      handleFactory: (_) {
        handle = _FakeControllerHandle([
          {'kind': 'state', 'state': 'closed'},
          {'kind': 'state', 'state': 'connected:path=direct'},
        ]);
        return handle;
      },
    );
    final states = <String>[];
    transport.onState = (state) {
      states.add(state);
      if (state == 'closed') {
        transport.close();
      }
    };

    await transport.connect(
      const StoredDevice(
        server: 'https://codux-service.dux.plus/v3',
        hostId: 'host-1',
        deviceId: 'device-1',
        token: 'token-1',
        name: 'Phone',
        transports: [
          RemoteTransportCandidate(
            kind: RemoteTransportKind.websocketRelay,
            url: 'https://codux-service.dux.plus/v3',
          ),
        ],
      ),
    );

    expect(states, ['connecting', 'closed']);
    expect(handle.pollCount, 1);
  });
}

final class _FakeControllerHandle implements ControllerTransportEventHandle {
  _FakeControllerHandle(this._events);

  final List<Map<String, dynamic>> _events;
  var _closed = false;
  var pollCount = 0;

  @override
  bool get isClosed => _closed;

  @override
  void close() {
    _closed = true;
  }

  @override
  Map<String, dynamic>? pollEvent() {
    if (_closed) {
      throw StateError('Controller transport has been closed');
    }
    pollCount += 1;
    if (_events.isEmpty) return null;
    return _events.removeAt(0);
  }

  @override
  bool probePreferredRoute() => false;

  @override
  bool reportPingTimeout({required String path}) => false;

  @override
  bool send(Map<String, dynamic> envelope) => true;
}
