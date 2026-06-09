import 'dart:async';
import 'dart:convert';
import 'dart:io';
import '../i18n.dart';
import '../models/remote_models.dart';
import 'e2e_crypto.dart';

const String remoteProtocolVersion = 'v3.1';

Future<PairingPayload> parsePairingPayload(String input) async {
  final parsed = await _fetchPairingTicketPayload(input);
  final code = parsed['code']?.toString();
  final secret = parsed['secret']?.toString();
  final hostId = parsed['hostId']?.toString();
  final hostPublicKey = parsed['hostPublicKey']?.toString() ?? '';
  final cryptoVersion = parsed['cryptoVersion'] is num
      ? (parsed['cryptoVersion'] as num).toInt()
      : int.tryParse('${parsed['cryptoVersion'] ?? ''}') ?? 0;
  final transports = _normalizedPairingTransports(parsed);
  final hasSupportedTransport = transports.any(
    (item) =>
        item.kind == RemoteTransportKind.websocketRelay &&
        item.url.trim().isNotEmpty,
  );
  final missingFields = <String>[
    if (code == null || code.isEmpty) 'code',
    if (secret == null || secret.isEmpty) 'secret',
    if (hostId == null || hostId.isEmpty) 'hostId',
    if (parsed['pairingId']?.toString().trim().isEmpty != false) 'pairingId',
    if (hostPublicKey.isEmpty) 'hostPublicKey',
    if (cryptoVersion < 1) 'cryptoVersion',
    if (!hasSupportedTransport) 'transports.websocketRelay.url',
  ];
  if (missingFields.isNotEmpty) {
    throw Exception(
      '${tr('remote.qrMissingFields', LocaleChoices.system.id)} (${missingFields.join(', ')})',
    );
  }
  final pairingCode = code!;
  final pairingSecret = secret!;
  final deviceKeyPair = await RemoteE2ECrypto.newDeviceKeyPair();
  return PairingPayload(
    code: pairingCode,
    secret: pairingSecret,
    hostPublicKey: hostPublicKey,
    devicePrivateKey: deviceKeyPair.privateKey,
    devicePublicKey: deviceKeyPair.publicKey,
    matchCode: RemoteE2ECrypto.matchCode(
      hostPublicKey: hostPublicKey,
      devicePublicKey: deviceKeyPair.publicKey,
      pairingCode: pairingCode,
      pairingSecret: pairingSecret,
    ),
    cryptoVersion: cryptoVersion,
    hostName: parsed['hostName']?.toString(),
    hostId: hostId,
    transports: transports,
    pairingId: parsed['pairingId']?.toString(),
  );
}

List<RemoteTransportCandidate> _normalizedPairingTransports(
  Map<String, dynamic> parsed,
) {
  return remoteTransportCandidatesFromJson(parsed['transports']);
}

Future<Map<String, dynamic>> _fetchPairingTicketPayload(String input) async {
  final value = input.trim();
  if (value.isEmpty) {
    throw Exception(tr('remote.qrEmpty', LocaleChoices.system.id));
  }
  final uri = Uri.tryParse(value);
  if (uri == null ||
      uri.scheme != 'codux' ||
      uri.host != 'pair' ||
      uri.queryParameters['server']?.trim().isEmpty != false ||
      uri.queryParameters['ticket']?.trim().isEmpty != false) {
    throw Exception(tr('remote.qrInvalid', LocaleChoices.system.id));
  }
  final server = uri.queryParameters['server']!.trim();
  final ticket = uri.queryParameters['ticket']!.trim();
  final response = await HttpClient()
      .getUrl(_pairingTicketUri(server, ticket))
      .then((request) => request.close())
      .timeout(const Duration(seconds: 12));
  final text = await response.transform(utf8.decoder).join();
  if (response.statusCode < 200 || response.statusCode >= 300) {
    throw Exception(tr('remote.qrInvalid', LocaleChoices.system.id));
  }
  final decoded = jsonDecode(text);
  if (decoded is Map<String, dynamic>) return decoded;
  if (decoded is Map) return Map<String, dynamic>.from(decoded);
  throw Exception(tr('remote.qrInvalid', LocaleChoices.system.id));
}

Uri _pairingTicketUri(String base, String ticket) {
  final baseUri = Uri.parse(base.trim());
  return baseUri.replace(
    path: _joinRemotePath(baseUri.path, '/api/tickets/$ticket'),
    query: '',
  );
}

RelayEnvelope pairingRequestEnvelope(PairingPayload payload, String name) {
  final pairingId = payload.pairingId?.trim();
  if (pairingId == null || pairingId.isEmpty) {
    throw Exception(tr('remote.qrMissingFields', LocaleChoices.system.id));
  }
  return RelayEnvelope(
    type: 'pairing.request',
    deviceId: payload.devicePublicKey,
    payload: {
      'pairingId': pairingId,
      'code': payload.code,
      'secret': payload.secret,
      'deviceName': name,
      'devicePublicKey': payload.devicePublicKey,
    },
  );
}

Future<StoredDevice> claimPairingOverRelay({
  required PairingPayload payload,
  required String name,
  Duration timeout = const Duration(seconds: 90),
}) async {
  RemoteTransportCandidate? transport;
  for (final candidate in payload.transports) {
    if (candidate.kind == RemoteTransportKind.websocketRelay &&
        candidate.url.trim().isNotEmpty) {
      transport = candidate;
      break;
    }
  }
  if (transport == null) {
    throw Exception(tr('remote.qrMissingFields', LocaleChoices.system.id));
  }
  final socket = await WebSocket.connect(
    _pairingWebSocketUri(transport.url, payload).toString(),
  ).timeout(const Duration(seconds: 12));
  try {
    socket.add(jsonEncode(pairingRequestEnvelope(payload, name).toJson()));
    final message = await socket
        .where((raw) => raw is String)
        .map((raw) {
          try {
            final decoded = jsonDecode(raw as String);
            if (decoded is Map) {
              return RelayEnvelope.fromJson(Map<String, dynamic>.from(decoded));
            }
          } catch (_) {}
          return const RelayEnvelope(type: '');
        })
        .where(
          (message) =>
              message.type == 'pairing.confirmed' ||
              message.type == 'pairing.rejected',
        )
        .first
        .timeout(timeout);
    if (message.type == 'pairing.rejected') {
      throw const PairingRejectedException();
    }
    return confirmedDevice(payload: payload, name: name, confirmed: message);
  } on TimeoutException {
    throw Exception(tr('remote.waitTimeout', LocaleChoices.system.id));
  } finally {
    await socket.close();
  }
}

Uri _pairingWebSocketUri(String base, PairingPayload payload) {
  final baseUri = Uri.parse(base.trim());
  final scheme = switch (baseUri.scheme) {
    'https' => 'wss',
    'http' => 'ws',
    final other => other,
  };
  return baseUri.replace(
    scheme: scheme,
    path: _joinRemotePath(baseUri.path, '/ws/client'),
    queryParameters: {
      'hostId': payload.hostId ?? '',
      'deviceId': payload.devicePublicKey,
    },
  );
}

String _joinRemotePath(String basePath, String path) {
  var base = basePath.trim().replaceAll(RegExp(r'/+$'), '');
  if (base.isEmpty) base = '/v3';
  final suffix = path.trim().replaceFirst(RegExp(r'^/+'), '');
  if (suffix.isEmpty) return base;
  return '$base/$suffix';
}

StoredDevice confirmedDevice({
  required PairingPayload payload,
  required String name,
  required RelayEnvelope confirmed,
}) {
  final data = confirmed.payload;
  if (data is! Map ||
      data['hostId'] == null ||
      data['deviceId'] == null ||
      data['token'] == null) {
    throw Exception('Pairing confirmed without device credentials');
  }
  RemoteTransportCandidate? relay;
  for (final candidate in payload.transports) {
    if (candidate.kind == RemoteTransportKind.websocketRelay &&
        candidate.url.trim().isNotEmpty) {
      relay = candidate;
      break;
    }
  }
  final server = relay?.url ?? '';
  return StoredDevice(
    server: server,
    hostId: '${data['hostId']}',
    deviceId: '${data['deviceId']}',
    token: '${data['token']}',
    name: name,
    hostPublicKey: payload.hostPublicKey,
    devicePrivateKey: payload.devicePrivateKey,
    devicePublicKey: payload.devicePublicKey,
    cryptoVersion: payload.cryptoVersion,
    hostName: data['hostName']?.toString() ?? payload.hostName,
    transports: payload.transports,
  );
}

class PairingCancelledException implements Exception {
  const PairingCancelledException();
  @override
  String toString() => tr('pair.cancelled', LocaleChoices.system.id);
}

class PairingRejectedException implements Exception {
  const PairingRejectedException();
  @override
  String toString() => tr('pair.rejected', LocaleChoices.system.id);
}
