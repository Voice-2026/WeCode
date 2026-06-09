import 'package:codux_flutter/models/remote_models.dart';
import 'package:codux_flutter/services/remote_transport.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('factory creates websocket relay transport for stored relay device', () {
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

    expect(transport.kind, RemoteTransportKind.websocketRelay);
  });

  test('factory creates webrtc composite driver when relay and p2p exist', () {
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
          RemoteTransportCandidate(
            kind: RemoteTransportKind.webRtc,
            url: 'https://codux-service.dux.plus/v3',
          ),
        ],
      ),
    );

    expect(transport.kind, RemoteTransportKind.webRtc);
  });

  test('factory falls back to relay if p2p has no relay for signaling', () {
    final transport = createRemoteTransport(
      const StoredDevice(
        server: 'https://codux-service.dux.plus/v3',
        hostId: 'host-1',
        deviceId: 'device-1',
        token: 'token-1',
        name: 'Phone',
        transports: [
          RemoteTransportCandidate(
            kind: RemoteTransportKind.webRtc,
            url: 'https://codux-service.dux.plus/v3',
          ),
        ],
      ),
    );

    expect(transport.kind, RemoteTransportKind.websocketRelay);
  });
}
