import 'package:connectivity_plus/connectivity_plus.dart';
import 'package:wecode_flutter/services/remote_network_route_refresh_controller.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('normalizes connectivity signatures', () {
    expect(
      connectivitySignature([
        ConnectivityResult.mobile,
        ConnectivityResult.wifi,
      ]),
      'mobile+wifi',
    );
    expect(
      connectivitySignature([ConnectivityResult.none, ConnectivityResult.wifi]),
      'wifi',
    );
    expect(connectivitySignature([ConnectivityResult.none]), 'none');
  });

  test('network changes debounce route refreshes', () async {
    final refreshes = <String>[];
    var pauses = 0;
    final controller = RemoteNetworkRouteRefreshController(
      onPauseLatency: () => pauses += 1,
      onRefreshRoute: refreshes.add,
      debounce: const Duration(milliseconds: 1),
    );

    controller.handleChanged([ConnectivityResult.wifi]);
    controller.handleChanged([ConnectivityResult.wifi]);
    await Future<void>.delayed(const Duration(milliseconds: 5));

    expect(pauses, 0);
    expect(refreshes, ['network-change']);
    controller.dispose();
  });

  test('none connectivity pauses latency without refreshing route', () async {
    final refreshes = <String>[];
    var pauses = 0;
    final controller = RemoteNetworkRouteRefreshController(
      onPauseLatency: () => pauses += 1,
      onRefreshRoute: refreshes.add,
      debounce: const Duration(milliseconds: 1),
    );

    controller.handleChanged([ConnectivityResult.none]);
    await Future<void>.delayed(const Duration(milliseconds: 5));

    expect(pauses, 1);
    expect(refreshes, isEmpty);
    controller.dispose();
  });
}
