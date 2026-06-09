import '../models/remote_models.dart';

class MobileSettingsController {
  const MobileSettingsController();

  static const fallbackDeviceName = 'Codux Mobile';

  String detectedNameFromDeviceInfo(Map<String, Object?> data) {
    for (final key in const ['name', 'model', 'product', 'localizedModel']) {
      final value = data[key]?.toString().trim();
      if (value != null && value.isNotEmpty) return value;
    }
    return fallbackDeviceName;
  }

  MobileSettings startupSettings({
    required MobileSettings? stored,
    required String detectedDeviceName,
  }) {
    return stored ?? MobileSettings(localName: detectedDeviceName);
  }

  MobileSettings saveSettings({
    required MobileSettings current,
    required String inputLocalName,
    required String detectedDeviceName,
  }) {
    final name = inputLocalName.trim();
    return current.copyWith(
      localName: name.isEmpty ? detectedDeviceName : name,
    );
  }
}
