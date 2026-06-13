import 'dart:convert';
import 'dart:typed_data';

import 'package:codux_protocol_ffi/codux_protocol_ffi.dart' as ffi;

import '../models/remote_models.dart';
import 'remote_protocol.dart';

class DeviceKeyPair {
  const DeviceKeyPair({required this.privateKey, required this.publicKey});

  final String privateKey;
  final String publicKey;
}

/// End-to-end crypto for the remote channel. The actual X25519/HKDF/AES-GCM
/// runs in the shared `codux-remote-crypto` Rust crate via FFI — the same code
/// the desktop host uses — so the two ends stay byte-compatible without a
/// hand-maintained second implementation.
class RemoteE2ECrypto {
  static Future<DeviceKeyPair> newDeviceKeyPair() async {
    final pair = ffi.e2eNewDeviceKeypair();
    return DeviceKeyPair(
      privateKey: '${pair['privateKey']}',
      publicKey: '${pair['publicKey']}',
    );
  }

  static Future<RelayEnvelope> encryptEnvelope({
    required RelayEnvelope inner,
    required StoredDevice device,
    required int seq,
  }) async {
    final securedInner = inner.copyWith(seq: seq);
    final plaintext = utf8.encode(jsonEncode(securedInner.toJson()));
    final payload = ffi.e2eEncrypt(
      devicePrivateKey: device.devicePrivateKey,
      hostPublicKey: device.hostPublicKey,
      hostId: device.hostId,
      deviceId: device.deviceId,
      plaintextBase64: base64UrlEncodeNoPadding(plaintext),
    );
    return RelayEnvelope(
      type: RemoteMessageType.secureMessage,
      deviceId: device.deviceId,
      sessionId: inner.sessionId,
      payload: payload,
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
    final plaintextBase64 = ffi.e2eDecrypt(
      devicePrivateKey: device.devicePrivateKey,
      hostPublicKey: device.hostPublicKey,
      hostId: device.hostId,
      deviceId: device.deviceId,
      payloadJson: jsonEncode(payload),
    );
    final plaintext = base64UrlDecodeValue(plaintextBase64);
    final decoded = jsonDecode(utf8.decode(plaintext));
    if (decoded is! Map<String, dynamic>) {
      throw const FormatException('Invalid decrypted envelope');
    }
    return RelayEnvelope.fromJson(decoded).copyWith(deviceId: device.deviceId);
  }

  /// Clear the cached derived symmetric keys inside the Rust core (called on
  /// reconnect / re-pair).
  static void clearCache() {
    ffi.e2eClearKeyCache();
  }

  static String base64UrlEncodeNoPadding(List<int> bytes) {
    return base64Url.encode(bytes).replaceAll('=', '');
  }

  static Uint8List base64UrlDecodeValue(String value) {
    return base64Url.decode(base64Url.normalize(value));
  }
}
