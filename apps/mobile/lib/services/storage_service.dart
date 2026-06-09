import 'dart:convert';
import 'package:shared_preferences/shared_preferences.dart';
import '../models/remote_models.dart';

class StorageService {
  static const devicesKey = 'codux.mobile.devices';
  static const legacyDeviceKey = 'codux.mobile.device';
  static const lastDeviceIdKey = 'codux.mobile.last_device_id';
  static const settingsKey = 'codux.mobile.settings';
  static const projectCachePrefix = 'codux.mobile.projects';

  Future<List<StoredDevice>> loadDevices() async {
    final prefs = await SharedPreferences.getInstance();
    final value = prefs.getString(devicesKey);
    if (value != null && value.isNotEmpty) {
      final list = jsonDecode(value) as List<dynamic>;
      return list
          .map((item) => StoredDevice.fromJson(item as Map<String, dynamic>))
          .toList();
    }
    final legacy = prefs.getString(legacyDeviceKey);
    if (legacy != null && legacy.isNotEmpty) {
      final migrated = [
        StoredDevice.fromJson(jsonDecode(legacy) as Map<String, dynamic>),
      ];
      await saveDevices(migrated);
      return migrated;
    }
    return [];
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
