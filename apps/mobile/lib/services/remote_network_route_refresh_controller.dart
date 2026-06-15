import 'dart:async';

import 'package:connectivity_plus/connectivity_plus.dart';

class RemoteNetworkRouteRefreshController {
  RemoteNetworkRouteRefreshController({
    required this.onPauseLatency,
    required this.onRefreshRoute,
    this.onInitialSignature,
    this.onSignatureChanged,
    this.onInitialCheckFailed,
    this.onListenError,
    this.debounce = const Duration(milliseconds: 700),
  });

  final RemoteNetworkPauseHandler onPauseLatency;
  final RemoteNetworkRefreshHandler onRefreshRoute;
  final RemoteNetworkInitialSignatureHandler? onInitialSignature;
  final RemoteNetworkChangedHandler? onSignatureChanged;
  final RemoteNetworkErrorHandler? onInitialCheckFailed;
  final RemoteNetworkErrorHandler? onListenError;
  final Duration debounce;

  Timer? _debounceTimer;
  StreamSubscription<List<ConnectivityResult>>? _subscription;
  String _signature = '';

  String get signature => _signature;

  void start([Connectivity? connectivity]) {
    final source = connectivity ?? Connectivity();
    unawaited(
      source
          .checkConnectivity()
          .then((results) {
            _signature = connectivitySignature(results);
            onInitialSignature?.call(_signature);
          })
          .catchError((Object error) {
            onInitialCheckFailed?.call(error);
          }),
    );
    _subscription = source.onConnectivityChanged.listen(
      handleChanged,
      onError: (Object error) => onListenError?.call(error),
    );
  }

  void handleChanged(List<ConnectivityResult> results) {
    final next = connectivitySignature(results);
    if (next == _signature) return;
    final previous = _signature;
    _signature = next;
    onSignatureChanged?.call(previous, next);
    if (next == ConnectivityResult.none.name) {
      _debounceTimer?.cancel();
      _debounceTimer = null;
      onPauseLatency();
      return;
    }
    _debounceTimer?.cancel();
    _debounceTimer = Timer(debounce, () => onRefreshRoute('network-change'));
  }

  void dispose() {
    _debounceTimer?.cancel();
    _debounceTimer = null;
    unawaited(_subscription?.cancel());
    _subscription = null;
  }
}

String connectivitySignature(List<ConnectivityResult> results) {
  final names =
      results
          .map((result) => result.name)
          .where((name) => name != ConnectivityResult.none.name)
          .toList()
        ..sort();
  return names.isEmpty ? ConnectivityResult.none.name : names.join('+');
}

typedef RemoteNetworkPauseHandler = void Function();
typedef RemoteNetworkRefreshHandler = void Function(String reason);
typedef RemoteNetworkInitialSignatureHandler = void Function(String signature);
typedef RemoteNetworkChangedHandler =
    void Function(String previous, String next);
typedef RemoteNetworkErrorHandler = void Function(Object error);
