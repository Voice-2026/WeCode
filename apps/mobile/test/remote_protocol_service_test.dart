import 'dart:convert';
import 'dart:io';

import 'package:codux_flutter/models/remote_models.dart';
import 'package:codux_flutter/services/e2e_crypto.dart';
import 'package:codux_flutter/services/remote_protocol_service.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('match code uses the shared host and mobile formula', () {
    expect(
      RemoteE2ECrypto.matchCode(
        hostPublicKey: 'host-public-key',
        devicePublicKey: 'device-public-key',
        pairingCode: '205503D6',
        pairingSecret: 'pairing-secret',
      ),
      '8EC-D5F',
    );
  });

  test('stored device reads v3 transport candidates', () {
    final device = StoredDevice.fromJson({
      'server': 'https://codux-service.dux.plus/v3',
      'hostId': 'host-1',
      'deviceId': 'device-1',
      'token': '',
      'name': 'Phone',
      'transports': [
        {
          'kind': 'websocketRelay',
          'role': 'host',
          'url': 'https://codux-service.dux.plus/v3',
        },
      ],
    });

    expect(device.transport, RemoteTransportKind.websocketRelay);
    expect(device.preferredTransport.url, 'https://codux-service.dux.plus/v3');
  });

  test(
    'v3 pairing ticket fetches payload and confirmation uses transport candidates',
    () async {
      final qr = await pairingTicketQr({
        'protocolVersion': remoteProtocolVersion,
        'code': '205503D6',
        'secret': 'pairing-secret',
        'pairingId': 'pair-1',
        'hostId': 'host-1',
        'hostPublicKey': 'host-public-key',
        'cryptoVersion': 1,
        'hostName': 'Mac',
        'transports': [
          {
            'kind': 'websocketRelay',
            'role': 'host',
            'url': 'https://codux-service.dux.plus/v3',
          },
          {
            'kind': 'webRtc',
            'role': 'host',
            'url': 'https://codux-service.dux.plus/v3',
          },
        ],
      });

      final payload = await parsePairingPayload(qr);
      expect(payload.transport.kind, RemoteTransportKind.websocketRelay);
      expect(payload.hostId, 'host-1');
      expect(payload.pairingId, 'pair-1');
      expect(payload.transport.url, 'https://codux-service.dux.plus/v3');

      final request = pairingRequestEnvelope(payload, 'Phone');
      expect(request.type, 'pairing.request');
      expect((request.payload as Map)['pairingId'], 'pair-1');
      expect((request.payload as Map)['deviceName'], 'Phone');
      expect(
        (request.payload as Map)['devicePublicKey'],
        payload.devicePublicKey,
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
      expect(confirmed.transport, RemoteTransportKind.webRtc);
      expect(confirmed.server, 'https://codux-service.dux.plus/v3');
      expect(confirmed.devicePublicKey, payload.devicePublicKey);
      expect(confirmed.toJson()['transports'], [
        {
          'kind': 'websocketRelay',
          'role': 'host',
          'url': 'https://codux-service.dux.plus/v3',
        },
        {
          'kind': 'webRtc',
          'role': 'host',
          'url': 'https://codux-service.dux.plus/v3',
        },
      ]);
    },
  );

  test('pairing payload rejects missing supported transport', () async {
    final qr = await pairingTicketQr({
      'protocolVersion': remoteProtocolVersion,
      'code': '205503D6',
      'secret': 'pairing-secret',
      'pairingId': 'pair-1',
      'hostId': 'host-1',
      'hostPublicKey': 'host-public-key',
      'cryptoVersion': 1,
      'transports': [
        {'kind': 'webRtc'},
      ],
    });

    expect(() => parsePairingPayload(qr), throwsException);
  });

  test('pairing payload reports missing encrypted fields', () async {
    final qr = await pairingTicketQr({
      'protocolVersion': remoteProtocolVersion,
      'code': '205503D6',
      'cryptoVersion': 1,
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
              contains('hostPublicKey'),
            )
            .having(
              (error) => error.toString(),
              'message',
              contains('transports.websocketRelay.url'),
            ),
      ),
    );
  });

  test('confirmation rejects incomplete device credentials', () async {
    final payload = await parsePairingPayload(
      await pairingTicketQr({
        'protocolVersion': remoteProtocolVersion,
        'code': '205503D6',
        'secret': 'pairing-secret',
        'pairingId': 'pair-1',
        'hostId': 'host-1',
        'hostPublicKey': 'host-public-key',
        'cryptoVersion': 1,
        'transports': [
          {
            'kind': 'websocketRelay',
            'url': 'https://codux-service.dux.plus/v3',
          },
        ],
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

  test('pairing payload requires host id for stateless v3 relay', () async {
    final qr = await pairingTicketQr({
      'protocolVersion': remoteProtocolVersion,
      'code': '205503D6',
      'secret': 'pairing-secret',
      'pairingId': 'pair-1',
      'hostPublicKey': 'host-public-key',
      'cryptoVersion': 1,
      'hostName': 'Mac',
      'transports': [
        {
          'kind': 'websocketRelay',
          'role': 'host',
          'url': 'https://codux-service.dux.plus',
        },
      ],
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
}

Future<String> pairingTicketQr(Map<String, dynamic> payload) async {
  final server = await HttpServer.bind(InternetAddress.loopbackIPv4, 0);
  final ticket = 'ticket-1';
  final uri = Uri.parse('http://${server.address.host}:${server.port}/v3');
  server.listen((request) {
    if (request.method == 'GET' &&
        request.uri.path == '/v3/api/tickets/$ticket') {
      request.response.headers.contentType = ContentType.json;
      request.response.write(jsonEncode(payload));
      request.response.close();
      return;
    }
    request.response.statusCode = HttpStatus.notFound;
    request.response.close();
  });
  addTearDown(server.close);
  return Uri(
    scheme: 'codux',
    host: 'pair',
    queryParameters: {'server': uri.toString(), 'ticket': ticket},
  ).toString();
}
