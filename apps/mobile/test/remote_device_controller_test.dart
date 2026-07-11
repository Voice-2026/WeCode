import 'package:wecode_flutter/models/remote_models.dart';
import 'package:wecode_flutter/services/remote_device_controller.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  const controller = RemoteDeviceController();
  const mac = StoredDevice(
    server: 'https://relay.example',
    hostId: 'host-mac',
    deviceId: 'device-mac',
    token: 'token-mac',
    name: 'Mac',
  );
  const win = StoredDevice(
    server: 'https://relay.example',
    hostId: 'host-win',
    deviceId: 'device-win',
    token: 'token-win',
    name: 'Win',
  );

  test('preserves active device when saving the device list', () {
    final state = controller.preserveActive(
      devices: const [mac, win],
      activeDevice: win,
    );

    expect(state.devices, const [mac, win]);
    expect(state.activeDevice, win);
  });

  test('falls back to the first device when active is missing', () {
    final state = controller.preserveActive(
      devices: const [mac, win],
      activeDevice: const StoredDevice(
        server: '',
        hostId: 'missing',
        deviceId: 'missing',
        token: '',
        name: 'Missing',
      ),
    );

    expect(state.activeDevice, mac);
  });

  test('upserts paired device at the front and activates it', () {
    final state = controller.upsertAndActivate(
      devices: const [mac, win],
      device: win.copyWith(hostName: 'Windows Host'),
    );

    expect(state.devices.map((item) => item.deviceId), [
      'device-win',
      'device-mac',
    ]);
    expect(state.activeDevice?.hostName, 'Windows Host');
  });

  test('replaces edited device while keeping active selection', () {
    final edited = win.copyWith(hostName: 'Studio Win');
    final state = controller.replace(
      devices: const [mac, win],
      device: edited,
      activeDevice: win,
    );

    expect(state.devices.last.hostName, 'Studio Win');
    expect(state.activeDevice, edited);
  });

  test('updates host name without changing unrelated devices', () {
    final result = controller.updateHostName(
      devices: const [mac, win],
      activeDevice: mac,
      deviceId: 'device-mac',
      hostName: 'Studio Mac',
    );

    expect(result.updatedDevice?.hostName, 'Studio Mac');
    expect(result.state.activeDevice?.hostName, 'Studio Mac');
    expect(result.state.devices.last, win);
  });

  test('removes active device and selects the first remaining device', () {
    final result = controller.remove(
      devices: const [mac, win],
      activeDevice: mac,
      device: mac,
    );

    expect(result.removedActive, isTrue);
    expect(result.state.devices, const [win]);
    expect(result.state.activeDevice, win);
  });
}
