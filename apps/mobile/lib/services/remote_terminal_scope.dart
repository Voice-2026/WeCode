import '../models/remote_models.dart';

class RemoteTerminalScope {
  const RemoteTerminalScope({
    required this.projectId,
    this.projectPath,
    this.worktreeId,
  });

  final String projectId;
  final String? projectPath;
  final String? worktreeId;

  factory RemoteTerminalScope.fromJson(Map<String, dynamic> json) {
    return RemoteTerminalScope(
      projectId: '${json['projectId'] ?? ''}',
      projectPath: _nullableString(json['projectPath']),
      worktreeId: _nullableString(json['worktreeId']),
    );
  }

  Map<String, Object> toPayload() => {
    'projectId': projectId,
    if (worktreeId != null && worktreeId!.trim().isNotEmpty)
      'worktreeId': worktreeId!,
    if (projectPath != null && projectPath!.trim().isNotEmpty)
      'projectPath': projectPath!,
  };
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

String? _nullableString(Object? value) {
  final trimmed = value?.toString().trim();
  return trimmed == null || trimmed.isEmpty ? null : trimmed;
}
