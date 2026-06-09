import '../models/remote_models.dart';

class RemoteDeviceState {
  const RemoteDeviceState({required this.devices, required this.activeDevice});

  final List<StoredDevice> devices;
  final StoredDevice? activeDevice;
}

class RemoteDeviceUpdateResult {
  const RemoteDeviceUpdateResult({
    required this.state,
    required this.updatedDevice,
  });

  final RemoteDeviceState state;
  final StoredDevice? updatedDevice;
}

class RemoteDeviceRemoveResult {
  const RemoteDeviceRemoveResult({
    required this.state,
    required this.removedActive,
  });

  final RemoteDeviceState state;
  final bool removedActive;
}

class RemoteDeviceController {
  const RemoteDeviceController();

  RemoteDeviceState preserveActive({
    required List<StoredDevice> devices,
    required StoredDevice? activeDevice,
  }) {
    final activeId = activeDevice?.deviceId;
    return RemoteDeviceState(
      devices: List.unmodifiable(devices),
      activeDevice: _deviceById(devices, activeId) ?? _firstOrNull(devices),
    );
  }

  RemoteDeviceState upsertAndActivate({
    required List<StoredDevice> devices,
    required StoredDevice device,
  }) {
    final next = [
      device,
      ...devices.where((item) => item.deviceId != device.deviceId),
    ];
    return RemoteDeviceState(
      devices: List.unmodifiable(next),
      activeDevice: device,
    );
  }

  RemoteDeviceState replace({
    required List<StoredDevice> devices,
    required StoredDevice device,
    required StoredDevice? activeDevice,
  }) {
    final next = devices
        .map((item) => item.deviceId == device.deviceId ? device : item)
        .toList();
    return RemoteDeviceState(
      devices: List.unmodifiable(next),
      activeDevice: activeDevice?.deviceId == device.deviceId
          ? device
          : _deviceById(next, activeDevice?.deviceId) ?? _firstOrNull(next),
    );
  }

  RemoteDeviceUpdateResult updateHostName({
    required List<StoredDevice> devices,
    required StoredDevice? activeDevice,
    required String deviceId,
    required String? hostName,
  }) {
    StoredDevice? updated;
    final next = devices.map((item) {
      if (item.deviceId != deviceId) return item;
      updated = item.copyWith(hostName: hostName);
      return updated!;
    }).toList();
    if (updated == null) {
      return RemoteDeviceUpdateResult(
        state: preserveActive(devices: devices, activeDevice: activeDevice),
        updatedDevice: null,
      );
    }
    return RemoteDeviceUpdateResult(
      state: RemoteDeviceState(
        devices: List.unmodifiable(next),
        activeDevice: activeDevice?.deviceId == deviceId
            ? updated
            : _deviceById(next, activeDevice?.deviceId) ?? _firstOrNull(next),
      ),
      updatedDevice: updated,
    );
  }

  RemoteDeviceRemoveResult remove({
    required List<StoredDevice> devices,
    required StoredDevice? activeDevice,
    required StoredDevice device,
  }) {
    final removedActive = activeDevice?.deviceId == device.deviceId;
    final next = devices
        .where((item) => item.deviceId != device.deviceId)
        .toList();
    return RemoteDeviceRemoveResult(
      state: preserveActive(devices: next, activeDevice: activeDevice),
      removedActive: removedActive,
    );
  }

  StoredDevice? _deviceById(List<StoredDevice> devices, String? deviceId) {
    if (deviceId == null || deviceId.isEmpty) return null;
    for (final device in devices) {
      if (device.deviceId == deviceId) return device;
    }
    return null;
  }

  StoredDevice? _firstOrNull(List<StoredDevice> devices) =>
      devices.isEmpty ? null : devices.first;
}
