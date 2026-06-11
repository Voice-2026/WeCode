import 'package:codux_flutter/models/remote_models.dart';
import 'package:codux_flutter/services/e2e_crypto.dart';
import 'package:codux_flutter/services/remote_envelope_send_queue.dart';
import 'package:codux_flutter/services/remote_transport.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('sends plain envelopes when no active device is available', () async {
    final queue = RemoteEnvelopeSendQueue();
    final transport = _FakeTransport();
    final results = <RemoteEnvelopeSendResult>[];

    await queue.send(
      message: const RelayEnvelope(type: 'host.info'),
      transport: transport,
      connected: () => true,
      onResult: (_, result) => results.add(result),
    );

    expect(transport.sent.single['type'], 'host.info');
    expect(results, [RemoteEnvelopeSendResult.delivered]);
  });

  test('encrypts envelopes and increments sequence numbers in order', () async {
    final queue = RemoteEnvelopeSendQueue();
    final transport = _FakeTransport();
    final device = await _fakeDevice();

    await queue.send(
      message: const RelayEnvelope(type: 'first'),
      transport: transport,
      connected: () => true,
      activeDevice: device,
    );
    await queue.send(
      message: const RelayEnvelope(type: 'second'),
      transport: transport,
      connected: () => true,
      activeDevice: device,
    );

    final first = await RemoteE2ECrypto.decryptEnvelope(
      outer: RelayEnvelope.fromJson(transport.sent[0]),
      device: device,
    );
    final second = await RemoteE2ECrypto.decryptEnvelope(
      outer: RelayEnvelope.fromJson(transport.sent[1]),
      device: device,
    );
    expect(first.type, 'first');
    expect(first.seq, 1);
    expect(second.type, 'second');
    expect(second.seq, 2);
  });

  test(
    'does not send when connection drops before queued message runs',
    () async {
      final queue = RemoteEnvelopeSendQueue();
      final transport = _FakeTransport();
      final results = <RemoteEnvelopeSendResult>[];
      var connected = true;

      await queue.send(
        message: const RelayEnvelope(type: 'first'),
        transport: transport,
        connected: () => connected,
        onResult: (_, result) => results.add(result),
      );
      connected = false;
      await queue.send(
        message: const RelayEnvelope(type: 'second'),
        transport: transport,
        connected: () => connected,
        onResult: (_, result) => results.add(result),
      );

      expect(transport.sent.map((item) => item['type']), ['first']);
      expect(results, [
        RemoteEnvelopeSendResult.delivered,
        RemoteEnvelopeSendResult.droppedBeforeEncrypt,
      ]);
    },
  );

  test('reports rejected sends from the transport layer', () async {
    final queue = RemoteEnvelopeSendQueue();
    final transport = _FakeTransport(sendResult: false);
    final results = <RemoteEnvelopeSendResult>[];

    await queue.send(
      message: const RelayEnvelope(type: 'project.select'),
      transport: transport,
      connected: () => true,
      onResult: (_, result) => results.add(result),
    );

    expect(transport.sent.single['type'], 'project.select');
    expect(results, [RemoteEnvelopeSendResult.rejected]);
  });
}

Future<StoredDevice> _fakeDevice() async {
  final host = await RemoteE2ECrypto.newDeviceKeyPair();
  final mobile = await RemoteE2ECrypto.newDeviceKeyPair();
  return StoredDevice(
    server: 'https://relay.example/v3',
    hostId: 'host-1',
    deviceId: 'device-1',
    token: 'token',
    name: 'Mac',
    hostPublicKey: host.publicKey,
    devicePrivateKey: mobile.privateKey,
    devicePublicKey: mobile.publicKey,
    cryptoVersion: 1,
  );
}

class _FakeTransport implements RemoteTransport {
  _FakeTransport({this.sendResult = true});

  final bool sendResult;
  final sent = <Map<String, dynamic>>[];

  @override
  String get kind => RemoteTransportKind.websocketRelay;

  @override
  set onEnvelope(RemoteTransportEnvelopeHandler? handler) {}

  @override
  set onState(RemoteTransportStateHandler? handler) {}

  @override
  Future<void> close() async {}

  @override
  Future<void> connect(StoredDevice device) async {}

  @override
  Future<bool> probePreferredRoute(StoredDevice device) async => false;

  @override
  Future<bool> reportPingTimeout({required String path}) async => false;

  @override
  Future<bool> send(Map<String, dynamic> envelope) async {
    sent.add(envelope);
    return sendResult;
  }
}
