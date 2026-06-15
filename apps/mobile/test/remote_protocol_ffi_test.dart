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
    expect(codux_protocol_ffi.transportKind('iroh'), RemoteTransportKind.iroh);
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
      codux_protocol_ffi.transportRelayUrl('https://relay.example'),
      'https://relay.example',
    );
    expect(
      codux_protocol_ffi.transportRelayUrlForPreset(preset: 'china'),
      'https://iroh-service.dux.plus',
    );
    expect(codux_protocol_ffi.transportRelayUrlForPreset(preset: 'global'), '');
    final transports = [
      {
        'kind': RemoteTransportKind.iroh,
        'url': 'https://relay.example',
        'nodeId': 'node-1',
        'relayUrl': 'https://relay.example',
      },
    ];
    expect(
      codux_protocol_ffi.preferredTransportKind(transports, pairing: true),
      RemoteTransportKind.iroh,
    );
    expect(
      codux_protocol_ffi.preferredTransportKind(transports, pairing: false),
      RemoteTransportKind.iroh,
    );
    expect(
      codux_protocol_ffi.preferredTransportKind([
        {'kind': RemoteTransportKind.iroh, 'ticket': 'endpointabc'},
      ], pairing: false),
      RemoteTransportKind.iroh,
    );
  });

  test('Rust FFI summarizes controller transport config', () {
    final summary = codux_protocol_ffi.controllerTransportConfigSummary({
      'relayUrl': 'https://relay.example',
      'hostId': 'host-1',
      'deviceId': 'device-1',
      'deviceToken': 'token-1',
      'transports': [
        {
          'kind': RemoteTransportKind.iroh,
          'url': 'https://relay.example',
          'nodeId': 'node-1',
          'relayUrl': 'https://relay.example',
        },
      ],
    });

    expect(summary['relayUrl'], 'https://relay.example');
    expect(summary['hostId'], 'host-1');
    expect(summary['deviceId'], 'device-1');
    expect(summary['transportKind'], RemoteTransportKind.iroh);
    expect(summary['transportCount'], 1);
    expect(summary.containsKey('stunCount'), isFalse);
  });

  test('Rust FFI terminal input normalizes IME committed text', () {
    expect(codux_protocol_ffi.terminalTextInput('abc'), 'abc');
    expect(codux_protocol_ffi.terminalTextInput('你好かな한글'), '你好かな한글');
    expect(codux_protocol_ffi.terminalTextInput('\u0008'), '\u007f');
    expect(codux_protocol_ffi.terminalTextInput('\n'), '\r');
    expect(codux_protocol_ffi.terminalTextInput('a\u{f700}b'), 'ab');
    expect(codux_protocol_ffi.terminalInsertInput('\u007f'), '\u007f');
    expect(
      codux_protocol_ffi.terminalInsertInput('paste\ntext'),
      '\u001b[200~paste\ntext\u001b[201~',
    );
  });

  test('Rust FFI terminal input maps special keys and app cursor mode', () {
    expect(codux_protocol_ffi.terminalKeyInput(key: 'backspace'), '\u007f');
    expect(codux_protocol_ffi.terminalKeyInput(key: 'enter'), '\r');
    expect(codux_protocol_ffi.terminalKeyInput(key: 'up'), '\u001b[A');
    expect(
      codux_protocol_ffi.terminalKeyInput(key: 'up', applicationCursor: true),
      '\u001bOA',
    );
    expect(
      codux_protocol_ffi.terminalKeyInputBytes(key: 'space', control: true),
      [0],
    );
    expect(
      codux_protocol_ffi.terminalSelectorInput(selector: 'deleteBackward:'),
      '\u007f',
    );
    expect(
      codux_protocol_ffi.terminalSelectorInput(selector: 'moveLeft:'),
      '\u001b[D',
    );
    expect(
      codux_protocol_ffi.terminalSelectorInput(
        selector: 'moveLeft:',
        applicationCursor: true,
      ),
      '\u001bOD',
    );
  });
}
