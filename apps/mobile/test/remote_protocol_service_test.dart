import 'dart:convert';
import 'package:codux_flutter/models/remote_models.dart';
import 'package:codux_flutter/services/remote_protocol_service.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('stored device reads iroh transport candidates', () {
    final device = StoredDevice.fromJson({
      'server': 'https://relay.example',
      'hostId': 'host-1',
      'deviceId': 'device-1',
      'token': '',
      'name': 'Phone',
      'transports': [irohCandidate(url: 'https://relay.example')],
    });

    expect(
      remotePreferredTransportKind(device.transports, pairing: false),
      RemoteTransportKind.iroh,
    );
    expect(
      device.transportByKind(RemoteTransportKind.iroh)?.url,
      'https://relay.example',
    );
    expect(device.transportByKind(RemoteTransportKind.iroh)?.nodeId, 'node-1');
  });

  test(
    'embedded iroh ticket pairing payload parses without relay ticket fetch',
    () async {
      final qr = embeddedPairingQr({
        'protocolVersion': remoteProtocolVersion,
        'code': '205503D6',
        'secret': 'pairing-secret',
        'pairingId': 'pair-1',
        'hostId': 'host-1',
        'hostName': 'Mac',
        'transports': [
          irohCandidate(
            url: 'https://relay.example',
            ticket: 'endpointabc',
            relayAuthentication: 'relay-token',
          ),
        ],
      });

      final payload = await parsePairingPayload(qr);
      final transport = payload.transportByKind(RemoteTransportKind.iroh);
      expect(payload.server, 'https://relay.example');
      expect(transport?.ticket, 'endpointabc');
      expect(transport?.relayAuthentication, 'relay-token');
    },
  );

  test(
    'embedded iroh token payload and confirmation use transport candidates',
    () async {
      final qr = embeddedPairingQr({
        'protocolVersion': remoteProtocolVersion,
        'code': '205503D6',
        'secret': 'pairing-secret',
        'pairingId': 'pair-1',
        'hostId': 'host-1',
        'hostName': 'Mac',
        'transports': [irohCandidate(url: 'https://relay.example')],
      });

      final payload = await parsePairingPayload(qr);
      expect(
        remotePreferredTransportKind(payload.transports, pairing: true),
        RemoteTransportKind.iroh,
      );
      expect(payload.hostId, 'host-1');
      expect(payload.pairingId, 'pair-1');
      expect(
        payload.transportByKind(RemoteTransportKind.iroh)?.url,
        'https://relay.example',
      );
      expect(
        payload.transportByKind(RemoteTransportKind.iroh)?.nodeId,
        'node-1',
      );

      final request = pairingRequestEnvelope(payload, 'Phone');
      expect(request.type, 'pairing.request');
      expect((request.payload as Map)['pairingId'], 'pair-1');
      expect((request.payload as Map)['deviceName'], 'Phone');
      expect((request.payload as Map)['deviceId'], payload.deviceId);

      final confirmed = confirmedDevice(
        payload: payload,
        name: 'Phone',
        confirmed: const RelayEnvelope(
          type: 'pairing.confirmed',
          payload: {
            'hostId': 'host-1',
            'deviceId': 'device-1',
            'token': '',
            'hostName': 'Mac',
          },
        ),
      );
      expect(
        remotePreferredTransportKind(confirmed.transports, pairing: false),
        RemoteTransportKind.iroh,
      );
      expect(confirmed.server, 'https://relay.example');
      expect(confirmed.deviceId, 'device-1');
      expect(confirmed.toJson()['transports'], [
        irohCandidate(url: 'https://relay.example'),
      ]);
    },
  );

  test('pasted bare token parses like a scanned qr payload', () async {
    final manual = embeddedPairingToken({
      'protocolVersion': remoteProtocolVersion,
      'code': '123456',
      'secret': 'pairing-secret',
      'pairingId': 'pair-1',
      'hostId': 'host-1',
      'hostName': 'Mac',
      'transports': [irohCandidate(url: 'https://relay.example')],
    });

    final payload = await parsePairingPayload(manual);
    expect(payload.server, 'https://relay.example');
    expect(payload.code, '123456');
    expect(payload.hostId, 'host-1');
    expect(payload.pairingId, 'pair-1');
    expect(
      payload.transportByKind(RemoteTransportKind.iroh)?.url,
      'https://relay.example',
    );
  });

  test('pairing payload rejects missing iroh transport', () async {
    final qr = embeddedPairingQr({
      'protocolVersion': remoteProtocolVersion,
      'code': '205503D6',
      'secret': 'pairing-secret',
      'pairingId': 'pair-1',
      'hostId': 'host-1',
      'transports': const [],
    });

    expect(() => parsePairingPayload(qr), throwsException);
  });

  test('pairing payload reports missing required fields', () async {
    final qr = embeddedPairingQr({
      'protocolVersion': remoteProtocolVersion,
      'code': '205503D6',
    });

    expect(
      () => parsePairingPayload(qr),
      throwsA(
        isA<Exception>()
            .having((error) => error.toString(), 'message', contains('secret'))
            .having(
              (error) => error.toString(),
              'message',
              contains('pairingId'),
            )
            .having((error) => error.toString(), 'message', contains('hostId'))
            .having(
              (error) => error.toString(),
              'message',
              contains('transports.iroh.ticket'),
            ),
      ),
    );
  });

  test('confirmation rejects incomplete device credentials', () async {
    final payload = await parsePairingPayload(
      embeddedPairingQr({
        'protocolVersion': remoteProtocolVersion,
        'code': '205503D6',
        'secret': 'pairing-secret',
        'pairingId': 'pair-1',
        'hostId': 'host-1',
        'transports': [irohCandidate(url: 'https://relay.example')],
      }),
    );
    expect(
      () => confirmedDevice(
        payload: payload,
        name: 'Phone',
        confirmed: const RelayEnvelope(
          type: 'pairing.confirmed',
          payload: {'hostId': 'host-1'},
        ),
      ),
      throwsA(
        isA<Exception>().having(
          (error) => error.toString(),
          'message',
          contains('Pairing confirmed without device credentials'),
        ),
      ),
    );
  });

  test('pairing payload requires host id for iroh ticket pairing', () async {
    final qr = embeddedPairingQr({
      'protocolVersion': remoteProtocolVersion,
      'code': '205503D6',
      'secret': 'pairing-secret',
      'pairingId': 'pair-1',
      'hostName': 'Mac',
      'transports': [irohCandidate(url: 'https://relay.example')],
    });

    expect(
      () => parsePairingPayload(qr),
      throwsA(
        isA<Exception>().having(
          (error) => error.toString(),
          'message',
          contains('hostId'),
        ),
      ),
    );
  });

  test(
    'embedded payload keeps its iroh candidate url as connection authority',
    () async {
      final qr = embeddedPairingQr({
        'protocolVersion': remoteProtocolVersion,
        'code': '205503D6',
        'secret': 'pairing-secret',
        'pairingId': 'pair-1',
        'hostId': 'host-1',
        'hostName': 'Mac',
        'transports': [irohCandidate(url: 'https://stale-relay.example')],
      });

      final payload = await parsePairingPayload(qr);

      expect(payload.server, 'https://stale-relay.example');
      expect(
        payload.transportByKind(RemoteTransportKind.iroh)?.url,
        'https://stale-relay.example',
      );
      expect(
        payload.transportByKind(RemoteTransportKind.iroh)?.relayUrl,
        'https://relay.example',
      );

      final confirmed = confirmedDevice(
        payload: payload,
        name: 'Phone',
        confirmed: const RelayEnvelope(
          type: 'pairing.confirmed',
          payload: {
            'hostId': 'host-1',
            'deviceId': 'device-1',
            'token': '',
            'hostName': 'Mac',
          },
        ),
      );

      expect(confirmed.server, 'https://stale-relay.example');
      expect(
        confirmed.transportByKind(RemoteTransportKind.iroh)?.url,
        'https://stale-relay.example',
      );
    },
  );
}

Map<String, dynamic> irohCandidate({
  required String url,
  String ticket = '',
  String relayAuthentication = '',
}) =>
    {
      'kind': RemoteTransportKind.iroh,
      'role': 'host',
      'url': url,
      'nodeId': 'node-1',
      'relayUrl': 'https://relay.example',
      if (ticket.isNotEmpty) 'ticket': ticket,
      if (relayAuthentication.isNotEmpty)
        'relayAuthentication': relayAuthentication,
    };

String embeddedPairingQr(Map<String, dynamic> payload) {
  final encoded = base64Url.encode(utf8.encode(jsonEncode(payload)));
  return Uri(
    scheme: 'codux',
    host: 'pair',
    queryParameters: {'payload': encoded},
  ).toString();
}

String embeddedPairingToken(Map<String, dynamic> payload) {
  return base64Url.encode(utf8.encode(jsonEncode(payload)));
}
