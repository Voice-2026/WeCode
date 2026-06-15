import 'package:codux_flutter/models/remote_models.dart';
import 'package:codux_flutter/services/device_selection_service.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  const service = DeviceSelectionService();

  StoredDevice device(String id) => StoredDevice(
    server: 'https://relay.example',
    hostId: 'host-$id',
    deviceId: id,
    token: 'token-$id',
    name: 'Device $id',
  );

  test('selects last responsive device for startup auto connect', () {
    final first = device('old');
    final second = device('current');

    final selection = service.selectStartupDevice([first, second], 'current');

    expect(selection.displayedDevice?.deviceId, 'current');
    expect(selection.autoConnectDevice?.deviceId, 'current');
  });

  test(
    'does not auto connect arbitrary first device without last responsive id',
    () {
      final first = device('old');
      final second = device('current');

      final selection = service.selectStartupDevice([first, second], null);

      expect(selection.displayedDevice?.deviceId, 'old');
      expect(selection.autoConnectDevice, isNull);
    },
  );
}
