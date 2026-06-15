import 'package:codux_flutter/models/remote_models.dart';
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

  test('sends envelopes and increments sequence numbers in order', () async {
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

    final first = RelayEnvelope.fromJson(transport.sent[0]);
    final second = RelayEnvelope.fromJson(transport.sent[1]);
    expect(first.type, 'first');
    expect(first.hostId, 'host-1');
    expect(first.deviceId, 'device-1');
    expect(first.seq, 1);
    expect(second.type, 'second');
    expect(second.hostId, 'host-1');
    expect(second.deviceId, 'device-1');
    expect(second.seq, 2);
  });

  test('attaches active device identity to host info messages', () async {
    final queue = RemoteEnvelopeSendQueue();
    final transport = _FakeTransport();
    final device = await _fakeDevice();

    await queue.send(
      message: const RelayEnvelope(type: 'host.info'),
      transport: transport,
      connected: () => true,
      activeDevice: device,
    );

    final envelope = RelayEnvelope.fromJson(transport.sent.single);
    expect(envelope.type, 'host.info');
    expect(envelope.hostId, 'host-1');
    expect(envelope.deviceId, 'device-1');
    expect(envelope.seq, 1);
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
        RemoteEnvelopeSendResult.droppedWhileDisconnected,
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
  return const StoredDevice(
    server: 'https://relay.example',
    hostId: 'host-1',
    deviceId: 'device-1',
    token: 'token',
    name: 'Mac',
  );
}

class _FakeTransport implements RemoteTransport {
  _FakeTransport({this.sendResult = true});

  final bool sendResult;
  final sent = <Map<String, dynamic>>[];

  @override
  String get kind => RemoteTransportKind.iroh;

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
    return sendResult;
  }
}
