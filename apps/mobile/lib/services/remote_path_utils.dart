import 'remote_protocol.dart';

String remoteLastPathComponent(String path) {
  final normalized = path.replaceAll('\\', '/');
  final parts = normalized
      .split('/')
      .where((part) => part.trim().isNotEmpty)
      .toList();
  return parts.isEmpty ? 'Project' : parts.last;
}

String remoteParentPathOf(String path) {
  final value = path.replaceAll('\\', '/');
  final normalized = value.endsWith('/') && value.length > 1
      ? value.substring(0, value.length - 1)
      : value;
  final index = normalized.lastIndexOf('/');
  if (index <= 0) return '/';
  return normalized.substring(0, index);
}

String cleanRemoteTransportEndpoint(String value) {
  var endpoint = value.trim();
  if (endpoint.startsWith('relay:')) {
    endpoint = endpoint.substring('relay:'.length).trim();
  } else if (endpoint.startsWith('ip:')) {
    endpoint = endpoint.substring('ip:'.length).trim();
  } else if (endpoint.startsWith('custom:')) {
    endpoint = endpoint.substring('custom:'.length).trim();
  }
  return endpoint;
}

String remoteRelayDisplayName(String value) {
  final endpoint = cleanRemoteTransportEndpoint(value);
  if (endpoint.isEmpty) return '';
  final normalized = endpoint.replaceFirst(RegExp(r'/+$'), '');
  final presets = remoteTransportRelayPresets();
  for (final preset in presets) {
    final url = '${preset['url'] ?? ''}'.trim().replaceFirst(
      RegExp(r'/+$'),
      '',
    );
    if (url.isNotEmpty && url == normalized) {
      final name = '${preset['name'] ?? ''}'.trim();
      if (name.isNotEmpty) return name;
    }
  }
  return endpoint;
}
