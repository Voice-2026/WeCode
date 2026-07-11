import 'package:wecode_flutter/services/remote_protocol_service.dart';
import 'package:wecode_flutter/models/remote_models.dart';
import 'package:wecode_protocol_ffi/wecode_protocol_ffi.dart'
    as wecode_protocol_ffi;
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('Rust FFI protocol names match Dart compile-time constants', () {
    expect(wecode_protocol_ffi.protocolVersion(), remoteProtocolVersion);
    expect(
      wecode_protocol_ffi.messageType('resourceSubscribe'),
      RemoteMessageType.resourceSubscribe,
    );
    expect(
      wecode_protocol_ffi.messageType('resourceUnsubscribe'),
      RemoteMessageType.resourceUnsubscribe,
    );
    expect(
      wecode_protocol_ffi.resourceType('terminals'),
      RemoteResourceType.terminals,
    );
    expect(wecode_protocol_ffi.transportKind('iroh'), RemoteTransportKind.iroh);
    expect(
      wecode_protocol_ffi.messageType('terminalBuffer'),
      RemoteMessageType.terminalBuffer,
    );
    expect(
      wecode_protocol_ffi.messageType('gitStatus'),
      RemoteMessageType.gitStatus,
    );
  });

  test('Rust FFI builds terminal resource subscribe envelope', () {
    final envelope = wecode_protocol_ffi.resourceSubscribeEnvelope(
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
    final relayPresets = wecode_protocol_ffi.transportRelayPresets();
    final tencentRelayUrl = relayPresets.firstWhere(
      (preset) => preset['key'] == 'china-tencent',
    )['url'];
    final aliyunRelayUrl = relayPresets.firstWhere(
      (preset) => preset['key'] == 'china-aliyun',
    )['url'];

    expect(
      wecode_protocol_ffi.transportRelayUrl('https://relay.example'),
      'https://relay.example',
    );
    expect(
      wecode_protocol_ffi.transportRelayUrlForPreset(preset: 'china'),
      tencentRelayUrl,
    );
    expect(
      wecode_protocol_ffi.transportRelayUrlForPreset(preset: 'china-aliyun'),
      aliyunRelayUrl,
    );
    expect(wecode_protocol_ffi.transportRelayUrlForPreset(preset: 'global'), '');
    expect(
      relayPresets.map((preset) => preset['key']).toList(),
      containsAll(['global', 'china-tencent', 'china-aliyun', 'custom']),
    );
    final transports = [
      {
        'kind': RemoteTransportKind.iroh,
        'url': 'https://relay.example',
        'nodeId': 'node-1',
        'relayUrl': 'https://relay.example',
      },
    ];
    expect(
      wecode_protocol_ffi.preferredTransportKind(transports, pairing: true),
      RemoteTransportKind.iroh,
    );
    expect(
      wecode_protocol_ffi.preferredTransportKind(transports, pairing: false),
      RemoteTransportKind.iroh,
    );
    expect(
      wecode_protocol_ffi.preferredTransportKind([
        {'kind': RemoteTransportKind.iroh, 'ticket': 'endpointabc'},
      ], pairing: false),
      RemoteTransportKind.iroh,
    );
  });

  test('Rust FFI summarizes controller transport config', () {
    final summary = wecode_protocol_ffi.controllerTransportConfigSummary({
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
    expect(wecode_protocol_ffi.terminalTextInput('abc'), 'abc');
    expect(wecode_protocol_ffi.terminalTextInput('你好かな한글'), '你好かな한글');
    expect(wecode_protocol_ffi.terminalTextInput('\u0008'), '\u007f');
    expect(wecode_protocol_ffi.terminalTextInput('\n'), '\r');
    expect(wecode_protocol_ffi.terminalTextInput('a\u{f700}b'), 'ab');
    expect(wecode_protocol_ffi.terminalInsertInput('\u007f'), '\u007f');
    expect(
      wecode_protocol_ffi.terminalInsertInput('paste\ntext'),
      '\u001b[200~paste\ntext\u001b[201~',
    );
  });

  test('Rust FFI terminal input maps special keys and app cursor mode', () {
    expect(wecode_protocol_ffi.terminalKeyInput(key: 'backspace'), '\u007f');
    expect(wecode_protocol_ffi.terminalKeyInput(key: 'enter'), '\r');
    expect(wecode_protocol_ffi.terminalKeyInput(key: 'up'), '\u001b[A');
    expect(
      wecode_protocol_ffi.terminalKeyInput(key: 'up', applicationCursor: true),
      '\u001bOA',
    );
  });
}
