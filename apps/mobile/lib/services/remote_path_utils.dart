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
