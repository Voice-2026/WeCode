import '../models/remote_models.dart';

List<ProjectInfo> remoteProjectsFromPayload(Object? payload) {
  if (payload is! Map) return const [];
  final list = payload['projects'] as List<dynamic>? ?? const [];
  return list
      .whereType<Map>()
      .map((item) => ProjectInfo.fromJson(Map<String, dynamic>.from(item)))
      .toList();
}

String? remoteSelectedProjectIdFromPayload(Object? payload) {
  if (payload is! Map) return null;
  final value = payload['selectedProjectId']?.toString().trim();
  return value == null || value.isEmpty ? null : value;
}

String? remoteSelectedWorktreeIdFromPayload(Object? payload) {
  if (payload is! Map) return null;
  final value = payload['selectedWorktreeId']?.toString().trim();
  return value == null || value.isEmpty ? null : value;
}

List<TerminalInfo> remoteTerminalsFromPayload(Object? payload) {
  if (payload is! Map) return const [];
  final list = payload['terminals'] as List<dynamic>? ?? const [];
  return list
      .whereType<Map>()
      .map((item) => TerminalInfo.fromJson(Map<String, dynamic>.from(item)))
      .toList();
}

TerminalInfo? remoteTerminalFromPayload(Object? payload) {
  if (payload is! Map) return null;
  return TerminalInfo.fromJson(Map<String, dynamic>.from(payload));
}

List<RemoteFileEntry> remoteFileEntriesFromPayload(Object? payload) {
  if (payload is! Map) return const [];
  final list = payload['entries'] as List<dynamic>? ?? const [];
  return list
      .whereType<Map>()
      .map((item) => RemoteFileEntry.fromJson(Map<String, dynamic>.from(item)))
      .toList();
}

List<RemoteWorktreeInfo> remoteWorktreesFromPayload(Object? payload) {
  if (payload is! Map) return const [];
  final baseBranchByWorktreeId = <String, String>{};
  final tasks = payload['tasks'];
  if (tasks is List) {
    for (final task in tasks.whereType<Map>()) {
      final worktreeId = task['worktreeId']?.toString().trim() ?? '';
      final baseBranch = task['baseBranch']?.toString().trim() ?? '';
      if (worktreeId.isEmpty || baseBranch.isEmpty) continue;
      baseBranchByWorktreeId[worktreeId] = baseBranch;
    }
  }
  final list = payload['worktrees'] as List<dynamic>? ?? const [];
  return list.whereType<Map>().map((item) {
    final worktree = RemoteWorktreeInfo.fromJson(
      Map<String, dynamic>.from(item),
    );
    return worktree.copyWith(baseBranch: baseBranchByWorktreeId[worktree.id]);
  }).toList();
}

RemoteGitStatusInfo? remoteGitStatusFromPayload(Object? payload) {
  if (payload is! Map) return null;
  return RemoteGitStatusInfo.fromJson(Map<String, dynamic>.from(payload));
}
