import '../models/remote_models.dart';

class StartupDeviceSelection {
  const StartupDeviceSelection({
    required this.displayedDevice,
    required this.autoConnectDevice,
  });

  final StoredDevice? displayedDevice;
  final StoredDevice? autoConnectDevice;
}

class DeviceSelectionService {
  const DeviceSelectionService();

  StartupDeviceSelection selectStartupDevice(
    List<StoredDevice> devices,
    String? lastResponsiveDeviceId,
  ) {
    if (devices.isEmpty) {
      return const StartupDeviceSelection(
        displayedDevice: null,
        autoConnectDevice: null,
      );
    }
    final last = lastResponsiveDeviceId?.trim();
    StoredDevice? autoConnect;
    if (last != null && last.isNotEmpty) {
      for (final device in devices) {
        if (device.deviceId == last) {
          autoConnect = device;
          break;
        }
      }
    }
    return StartupDeviceSelection(
      displayedDevice: autoConnect ?? devices.first,
      autoConnectDevice: autoConnect,
    );
  }
}
