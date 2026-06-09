import '../models/remote_models.dart';

class RemoteTerminalScope {
  const RemoteTerminalScope({required this.projectId, this.projectPath});

  final String projectId;
  final String? projectPath;

  Map<String, Object> toPayload() => {
    'projectId': projectId,
    if (projectPath != null && projectPath!.trim().isNotEmpty)
      'projectPath': projectPath!,
  };
}

RemoteTerminalScope? remoteTerminalScopeForProject({
  required String projectId,
  required List<ProjectInfo> projects,
}) {
  final scopedProjectId = projectId.trim();
  if (scopedProjectId.isEmpty) return null;
  return RemoteTerminalScope(
    projectId: scopedProjectId,
    projectPath: _projectPath(scopedProjectId, projects),
  );
}

RemoteTerminalScope? remoteTerminalScopeForSession({
  required String sessionId,
  required List<ProjectInfo> projects,
  required List<TerminalInfo> terminals,
  required String? selectedProjectId,
  TerminalInfo? terminal,
}) {
  final terminalProjectId =
      (terminal?.id == sessionId ? terminal?.projectId : null) ??
      _terminalProjectId(sessionId, terminals);
  final projectId = _firstNonEmpty([terminalProjectId, selectedProjectId]);
  if (projectId == null) return null;
  return RemoteTerminalScope(
    projectId: projectId,
    projectPath: _projectPath(projectId, projects),
  );
}

Map<String, Object?> scopedTerminalPayload(
  Map<String, Object?> payload,
  RemoteTerminalScope scope,
) {
  return {...payload, ...scope.toPayload()};
}

RelayEnvelope scopedTerminalEnvelope(
  RelayEnvelope envelope,
  RemoteTerminalScope scope,
) {
  final payload = envelope.payload is Map
      ? Map<String, Object?>.from(envelope.payload as Map)
      : <String, Object?>{};
  return envelope.copyWith(payload: scopedTerminalPayload(payload, scope));
}

String? _terminalProjectId(String sessionId, List<TerminalInfo> terminals) {
  for (final terminal in terminals) {
    if (terminal.id == sessionId && terminal.projectId.trim().isNotEmpty) {
      return terminal.projectId;
    }
  }
  return null;
}

String? _projectPath(String projectId, List<ProjectInfo> projects) {
  for (final project in projects) {
    if (project.id == projectId && (project.path ?? '').trim().isNotEmpty) {
      return project.path;
    }
  }
  return null;
}

String? _firstNonEmpty(Iterable<String?> values) {
  for (final value in values) {
    final trimmed = value?.trim();
    if (trimmed != null && trimmed.isNotEmpty) return trimmed;
  }
  return null;
}
