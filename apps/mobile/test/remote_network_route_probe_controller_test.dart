import 'package:connectivity_plus/connectivity_plus.dart';
import 'package:codux_flutter/services/remote_network_route_probe_controller.dart';
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

  test('network changes debounce preferred route probes', () async {
    final probes = <String>[];
    var pauses = 0;
    final controller = RemoteNetworkRouteProbeController(
      onPauseLatency: () => pauses += 1,
      onProbeRoute: probes.add,
      debounce: const Duration(milliseconds: 1),
    );

    controller.handleChanged([ConnectivityResult.wifi]);
    controller.handleChanged([ConnectivityResult.wifi]);
    await Future<void>.delayed(const Duration(milliseconds: 5));

    expect(pauses, 0);
    expect(probes, ['network-change']);
    controller.dispose();
  });

  test('none connectivity pauses latency without probing route', () async {
    final probes = <String>[];
    var pauses = 0;
    final controller = RemoteNetworkRouteProbeController(
      onPauseLatency: () => pauses += 1,
      onProbeRoute: probes.add,
      debounce: const Duration(milliseconds: 1),
    );

    controller.handleChanged([ConnectivityResult.none]);
    await Future<void>.delayed(const Duration(milliseconds: 5));

    expect(pauses, 1);
    expect(probes, isEmpty);
    controller.dispose();
  });
}
