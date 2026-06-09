import 'package:codux_flutter/models/remote_models.dart';
import 'package:codux_flutter/services/e2e_crypto.dart';
import 'package:codux_flutter/services/remote_envelope_send_queue.dart';
import 'package:codux_flutter/services/remote_transport.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('sends plain envelopes when no active device is available', () async {
    final queue = RemoteEnvelopeSendQueue();
    final transport = _FakeTransport();

    await queue.send(
      message: const RelayEnvelope(type: 'host.info'),
      transport: transport,
      connected: () => true,
    );

    expect(transport.sent.single['type'], 'host.info');
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
      var connected = true;

      await queue.send(
        message: const RelayEnvelope(type: 'first'),
        transport: transport,
        connected: () => connected,
      );
      connected = false;
      await queue.send(
        message: const RelayEnvelope(type: 'second'),
        transport: transport,
        connected: () => connected,
      );

      expect(transport.sent.map((item) => item['type']), ['first']);
    },
  );
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
  Future<bool> send(Map<String, dynamic> envelope) async {
    sent.add(envelope);
    return true;
  }
}
