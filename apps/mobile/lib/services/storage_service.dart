import 'dart:convert';
import 'package:shared_preferences/shared_preferences.dart';
import '../models/remote_models.dart';

class StorageService {
  static const devicesKey = 'wecode.mobile.devices';
  static const singleDeviceKey = 'wecode.mobile.device';
  static const lastDeviceIdKey = 'wecode.mobile.last_device_id';
  static const settingsKey = 'wecode.mobile.settings';
  static const projectCachePrefix = 'wecode.mobile.projects';

  Future<List<StoredDevice>> loadDevices() async {
    final prefs = await SharedPreferences.getInstance();
    final value = prefs.getString(devicesKey);
    if (value != null && value.isNotEmpty) {
      final list = jsonDecode(value) as List<dynamic>;
      final devices = list
          .map((item) => StoredDevice.fromJson(item as Map<String, dynamic>))
          .toList();
      return _normalizeAndPersistDevices(prefs, devices);
    }
    final singleDevice = prefs.getString(singleDeviceKey);
    if (singleDevice != null && singleDevice.isNotEmpty) {
      final migrated = [
        StoredDevice.fromJson(jsonDecode(singleDevice) as Map<String, dynamic>),
      ];
      await saveDevices(migrated);
      return migrated;
    }
    return [];
  }

  Future<List<StoredDevice>> _normalizeAndPersistDevices(
    SharedPreferences prefs,
    List<StoredDevice> devices,
  ) async {
    final normalized = devices.map(_normalizeDeviceTransport).toList();
    if (jsonEncode(normalized.map((item) => item.toJson()).toList()) !=
        jsonEncode(devices.map((item) => item.toJson()).toList())) {
      await prefs.setString(
        devicesKey,
        jsonEncode(normalized.map((item) => item.toJson()).toList()),
      );
    }
    return normalized;
  }

  StoredDevice _normalizeDeviceTransport(StoredDevice device) {
    final transports = device.transports
        .where((candidate) => candidate.kind == RemoteTransportKind.iroh)
        .map(
          (candidate) => RemoteTransportCandidate(
            kind: candidate.kind,
            role: candidate.role,
            url: candidate.url,
            nodeId: candidate.nodeId,
            relayUrl: candidate.relayUrl,
            relayAuthentication: candidate.relayAuthentication,
          ),
        )
        .toList();
    return device.copyWith(transports: transports);
  }

  Future<void> saveDevices(List<StoredDevice> devices) async {
    final prefs = await SharedPreferences.getInstance();
    await prefs.setString(
      devicesKey,
      jsonEncode(devices.map((item) => item.toJson()).toList()),
    );
  }

  Future<String?> loadLastDeviceId() async {
    final prefs = await SharedPreferences.getInstance();
    final value = prefs.getString(lastDeviceIdKey);
    return value == null || value.isEmpty ? null : value;
  }

  Future<void> saveLastDeviceId(String deviceId) async {
    if (deviceId.isEmpty) return;
    final prefs = await SharedPreferences.getInstance();
    await prefs.setString(lastDeviceIdKey, deviceId);
  }

  Future<MobileSettings?> loadSettings() async {
    final prefs = await SharedPreferences.getInstance();
    final value = prefs.getString(settingsKey);
    if (value == null || value.isEmpty) return null;
    return MobileSettings.fromJson(jsonDecode(value) as Map<String, dynamic>);
  }

  Future<void> saveSettings(MobileSettings settings) async {
    final prefs = await SharedPreferences.getInstance();
    await prefs.setString(settingsKey, jsonEncode(settings.toJson()));
  }

  Future<List<ProjectInfo>> loadCachedProjects(StoredDevice device) async {
    final prefs = await SharedPreferences.getInstance();
    final value = prefs.getString(_projectCacheKey(device));
    if (value == null || value.isEmpty) return [];
    final list = jsonDecode(value) as List<dynamic>;
    return list
        .map((item) => ProjectInfo.fromJson(item as Map<String, dynamic>))
        .toList();
  }

  Future<void> saveCachedProjects(
    StoredDevice device,
    List<ProjectInfo> projects,
  ) async {
    final prefs = await SharedPreferences.getInstance();
    await prefs.setString(
      _projectCacheKey(device),
      jsonEncode(projects.map((item) => item.toJson()).toList()),
    );
  }

  String _projectCacheKey(StoredDevice device) =>
      '$projectCachePrefix.${device.hostId}';
}
