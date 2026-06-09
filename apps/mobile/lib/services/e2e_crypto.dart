import 'dart:convert';
import 'dart:math';
import 'dart:typed_data';

import 'package:cryptography/cryptography.dart';
import 'package:cryptography/dart.dart';

import '../models/remote_models.dart';

class DeviceKeyPair {
  const DeviceKeyPair({required this.privateKey, required this.publicKey});

  final String privateKey;
  final String publicKey;
}

class RemoteE2ECrypto {
  static final X25519 _x25519 = X25519();
  static final Hkdf _hkdf = Hkdf(hmac: Hmac.sha256(), outputLength: 32);
  static final AesGcm _cipher = AesGcm.with256bits();
  static final Random _random = Random.secure();
  static final Map<String, Future<SecretKey>> _keyCache = {};

  static Future<DeviceKeyPair> newDeviceKeyPair() async {
    final keyPair = await _x25519.newKeyPair();
    final privateBytes = await keyPair.extractPrivateKeyBytes();
    final publicKey = await keyPair.extractPublicKey();
    return DeviceKeyPair(
      privateKey: base64UrlEncodeNoPadding(privateBytes),
      publicKey: base64UrlEncodeNoPadding(publicKey.bytes),
    );
  }

  static String matchCode({
    required String hostPublicKey,
    required String devicePublicKey,
    required String pairingCode,
    required String pairingSecret,
  }) {
    final material =
        'codux-e2e-match-v1|$hostPublicKey|$devicePublicKey|$pairingCode|$pairingSecret';
    final bytes = utf8.encode(material);
    final digest = const DartSha256().hashSync(bytes).bytes;
    final prefix = digest.take(3).map((byte) {
      return byte.toRadixString(16).padLeft(2, '0').toUpperCase();
    }).join();
    return '${prefix.substring(0, 3)}-${prefix.substring(3)}';
  }

  static Future<RelayEnvelope> encryptEnvelope({
    required RelayEnvelope inner,
    required StoredDevice device,
    required int seq,
  }) async {
    final securedInner = inner.copyWith(seq: seq);
    final plaintext = utf8.encode(jsonEncode(securedInner.toJson()));
    final key = await symmetricKey(device);
    final nonce = List<int>.generate(12, (_) => _random.nextInt(256));
    final box = await _cipher.encrypt(
      plaintext,
      secretKey: key,
      nonce: nonce,
      aad: _aad(device),
    );
    return RelayEnvelope(
      type: 'secure.message',
      deviceId: device.deviceId,
      sessionId: inner.sessionId,
      payload: {
        'v': 1,
        'alg': 'X25519-HKDF-SHA256-AES-256-GCM',
        'nonce': base64UrlEncodeNoPadding(box.nonce),
        'ciphertext': base64UrlEncodeNoPadding(box.cipherText),
        'tag': base64UrlEncodeNoPadding(box.mac.bytes),
      },
    );
  }

  static Future<RelayEnvelope> decryptEnvelope({
    required RelayEnvelope outer,
    required StoredDevice device,
  }) async {
    final payload = outer.payload;
    if (payload is! Map) {
      throw const FormatException('Missing encrypted payload');
    }
    final nonce = base64UrlDecodeValue('${payload['nonce'] ?? ''}');
    final ciphertext = base64UrlDecodeValue('${payload['ciphertext'] ?? ''}');
    final tag = base64UrlDecodeValue('${payload['tag'] ?? ''}');
    final key = await symmetricKey(device);
    final plaintext = await _cipher.decrypt(
      SecretBox(ciphertext, nonce: nonce, mac: Mac(tag)),
      secretKey: key,
      aad: _aad(device),
    );
    final decoded = jsonDecode(utf8.decode(plaintext));
    if (decoded is! Map<String, dynamic>) {
      throw const FormatException('Invalid decrypted envelope');
    }
    return RelayEnvelope.fromJson(decoded).copyWith(deviceId: device.deviceId);
  }

  static String base64UrlEncodeNoPadding(List<int> bytes) {
    return base64Url.encode(bytes).replaceAll('=', '');
  }

  static Uint8List base64UrlDecodeValue(String value) {
    return base64Url.decode(base64Url.normalize(value));
  }

  static Future<SecretKey> symmetricKey(StoredDevice device) {
    final key = cacheKey(device);
    return _keyCache.putIfAbsent(key, () => _deriveSymmetricKey(device));
  }

  static void clearCache() {
    _keyCache.clear();
  }

  static String cacheKey(StoredDevice device) {
    final material =
        'codux-e2e-cache-v1|${device.devicePrivateKey}|${device.hostPublicKey}|${device.hostId}|${device.deviceId}';
    final digest = const DartSha256().hashSync(utf8.encode(material)).bytes;
    return base64UrlEncodeNoPadding(digest);
  }

  static Future<SecretKey> _deriveSymmetricKey(StoredDevice device) async {
    if (device.devicePrivateKey.isEmpty || device.hostPublicKey.isEmpty) {
      throw StateError('This device must be paired again for E2E encryption');
    }
    final keyPair = await _x25519.newKeyPairFromSeed(
      base64UrlDecodeValue(device.devicePrivateKey),
    );
    final shared = await _x25519.sharedSecretKey(
      keyPair: keyPair,
      remotePublicKey: SimplePublicKey(
        base64UrlDecodeValue(device.hostPublicKey),
        type: KeyPairType.x25519,
      ),
    );
    return _hkdf.deriveKey(
      secretKey: shared,
      nonce: utf8.encode('codux-e2e-v1|${device.hostId}|${device.deviceId}'),
      info: utf8.encode('codux-remote-payload-v1'),
    );
  }

  static List<int> _aad(StoredDevice device) {
    return utf8.encode('codux-e2e-aad-v1|${device.hostId}|${device.deviceId}');
  }
}
