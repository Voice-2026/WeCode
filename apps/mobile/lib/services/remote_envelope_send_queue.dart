import '../models/remote_models.dart';
import 'e2e_crypto.dart';
import 'remote_transport.dart';

typedef RemoteSendErrorHandler = void Function(Object error);

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
  }) {
    final seq = activeDevice == null ? null : ++_seq;
    final previous = _chain.catchError((_) {});
    final task = previous
        .then((_) async {
          if (!connected()) return;
          if (activeDevice == null) {
            await transport.send(message.toJson());
            return;
          }
          final encrypted = await RemoteE2ECrypto.encryptEnvelope(
            inner: message,
            device: activeDevice,
            seq: seq!,
          );
          if (!connected()) return;
          await transport.send(encrypted.toJson());
        })
        .catchError((Object error) {
          onError?.call(error);
        });
    _chain = task;
    return task;
  }
}
