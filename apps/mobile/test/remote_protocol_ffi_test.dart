import 'package:codux_flutter/services/remote_protocol_service.dart';
import 'package:codux_flutter/models/remote_models.dart';
import 'package:codux_protocol_ffi/codux_protocol_ffi.dart'
    as codux_protocol_ffi;
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('Rust FFI protocol names match Dart compile-time constants', () {
    expect(codux_protocol_ffi.protocolVersion(), remoteProtocolVersion);
    expect(
      codux_protocol_ffi.messageType('resourceSubscribe'),
      RemoteMessageType.resourceSubscribe,
    );
    expect(
      codux_protocol_ffi.messageType('resourceUnsubscribe'),
      RemoteMessageType.resourceUnsubscribe,
    );
    expect(
      codux_protocol_ffi.resourceType('terminals'),
      RemoteResourceType.terminals,
    );
    expect(
      codux_protocol_ffi.transportKind('websocketRelay'),
      RemoteTransportKind.websocketRelay,
    );
    expect(
      codux_protocol_ffi.transportKind('webRtc'),
      RemoteTransportKind.webRtc,
    );
    expect(
      codux_protocol_ffi.messageType('terminalBuffer'),
      RemoteMessageType.terminalBuffer,
    );
    expect(
      codux_protocol_ffi.messageType('gitStatus'),
      RemoteMessageType.gitStatus,
    );
  });

  test('Rust FFI builds terminal resource subscribe envelope', () {
    final envelope = codux_protocol_ffi.resourceSubscribeEnvelope(
      resource: RemoteResourceType.terminals,
      projectId: 'project-1',
      baseline: true,
      maxChars: 65536,
      chunkChars: 16384,
    );

    expect(envelope['type'], RemoteMessageType.resourceSubscribe);
    expect(envelope['sessionId'], isNull);
    final payload = envelope['payload'] as Map;
    expect(payload['resource'], RemoteResourceType.terminals);
    expect(payload['projectId'], 'project-1');
    expect(payload['baseline'], isTrue);
    expect(payload['maxChars'], 65536);
    expect(payload['chunkChars'], 16384);
  });

  test('Rust FFI owns controller transport URL and selection rules', () {
    expect(
      codux_protocol_ffi.transportServerUrl('https://relay.example'),
      'https://relay.example/v3',
    );
    expect(
      codux_protocol_ffi.transportPairingTicketUrl(
        base: 'https://relay.example',
        ticket: 'ticket-1',
      ),
      'https://relay.example/v3/api/tickets/ticket-1',
    );
    expect(
      codux_protocol_ffi.transportPairingWebSocketUrl(
        base: 'https://relay.example',
        hostId: 'host-1',
        devicePublicKey: 'device-key',
      ),
      'wss://relay.example/v3/ws/client?hostId=host-1&deviceId=device-key',
    );
    expect(
      codux_protocol_ffi.transportClientWebSocketUrl(
        base: 'https://relay.example',
        hostId: 'host-1',
        deviceId: 'device-1',
        token: 'token-1',
      ),
      'wss://relay.example/v3/ws/client?hostId=host-1&deviceId=device-1&token=token-1',
    );

    final transports = [
      {
        'kind': RemoteTransportKind.websocketRelay,
        'url': 'https://relay.example/v3',
      },
      {'kind': RemoteTransportKind.webRtc, 'url': 'https://relay.example/v3'},
    ];
    expect(
      codux_protocol_ffi.preferredTransportKind(transports, pairing: true),
      RemoteTransportKind.websocketRelay,
    );
    expect(
      codux_protocol_ffi.preferredTransportKind(transports, pairing: false),
      RemoteTransportKind.webRtc,
    );
    expect(
      codux_protocol_ffi.preferredTransportKind([
        {'kind': RemoteTransportKind.webRtc, 'url': 'https://relay.example/v3'},
      ], pairing: false),
      '',
    );
    expect(
      codux_protocol_ffi.transportDefaultIceServers().first['urls'],
      contains('stun:stun.miwifi.com:3478'),
    );
  });

  test('Rust FFI terminal core owns remote pty baseline state', () {
    final session = codux_protocol_ffi.TerminalCoreSession(
      sessionId: 'session-1',
      maxCachedChars: 4,
    );
    try {
      session.requireBaseline();
      final first = session.acceptBaselinePage(
        data: 'ab',
        offset: 0,
        bufferLength: 4,
        truncated: true,
      );
      expect(first.ready, isFalse);
      expect(session.bufferLength, 2);

      final second = session.acceptBaselinePage(
        data: 'cd',
        offset: 2,
        bufferLength: 4,
        truncated: false,
      );
      expect(second.ready, isTrue);
      session.replaceFromBaseline(
        content: second.data,
        bufferLength: 4,
        sequence: 7,
      );
      session.appendLive(data: '你好', bufferLength: 6, sequence: 8);

      expect(session.content, 'cd你好');
      expect(session.bufferLength, 6);
      expect(session.sequence, 8);
    } finally {
      session.dispose();
    }
  });

  test('Rust FFI terminal core returns replay tokens after baseline', () {
    final session = codux_protocol_ffi.TerminalCoreSession(
      sessionId: 'session-1',
      maxCachedChars: 100,
    );
    try {
      session.requireBaseline();
      expect(session.holdLiveToken(sequence: 11, token: 1), isTrue);
      expect(session.holdLiveToken(sequence: 9, token: 2), isTrue);
      expect(session.holdLiveToken(sequence: null, token: 3), isTrue);

      final tokens = session.replaceFromBaseline(
        content: 'baseline',
        bufferLength: 8,
        sequence: 10,
      );

      expect(tokens, [1, 3]);
    } finally {
      session.dispose();
    }
  });
}
