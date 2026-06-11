import '../models/remote_models.dart';
import 'e2e_crypto.dart';
import 'remote_transport.dart';

typedef RemoteSendErrorHandler = void Function(Object error);
typedef RemoteSendResultHandler =
    void Function(RelayEnvelope message, RemoteEnvelopeSendResult result);

enum RemoteEnvelopeSendResult {
  delivered,
  droppedBeforeEncrypt,
  droppedAfterEncrypt,
  rejected,
}

class RemoteEnvelopeSendQueue {
  int _seq = 0;
  Future<void> _chain = Future<void>.value();

  void reset({int? seed}) {
    _seq = seed ?? 0;
    _chain = Future<void>.value();
  }

  Future<void> send({
    required RelayEnvelope message,
    required RemoteTransport transport,
    required bool Function() connected,
    StoredDevice? activeDevice,
    RemoteSendErrorHandler? onError,
    RemoteSendResultHandler? onResult,
  }) {
    final seq = activeDevice == null ? null : ++_seq;
    final previous = _chain.catchError((_) {});
    final task = previous
        .then((_) async {
          if (!connected()) {
            onResult?.call(
              message,
              RemoteEnvelopeSendResult.droppedBeforeEncrypt,
            );
            return;
          }
          var sent = false;
          if (activeDevice == null) {
            sent = await transport.send(message.toJson());
            onResult?.call(
              message,
              sent
                  ? RemoteEnvelopeSendResult.delivered
                  : RemoteEnvelopeSendResult.rejected,
            );
            return;
          }
          final encrypted = await RemoteE2ECrypto.encryptEnvelope(
            inner: message,
            device: activeDevice,
            seq: seq!,
          );
          if (!connected()) {
            onResult?.call(
              message,
              RemoteEnvelopeSendResult.droppedAfterEncrypt,
            );
            return;
          }
          sent = await transport.send(encrypted.toJson());
          onResult?.call(
            message,
            sent
                ? RemoteEnvelopeSendResult.delivered
                : RemoteEnvelopeSendResult.rejected,
          );
        })
        .catchError((Object error) {
          onError?.call(error);
        });
    _chain = task;
    return task;
  }
}
