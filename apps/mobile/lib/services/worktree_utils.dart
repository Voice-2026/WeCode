import '../models/remote_models.dart';
import 'remote_protocol.dart';
import 'remote_runtime_payloads.dart';

class RemoteWorktreeState {
  const RemoteWorktreeState({
    required this.worktrees,
    required this.selectedWorktreeId,
    required this.baseBranches,
    required this.defaultBaseBranch,
  });

  final List<RemoteWorktreeInfo> worktrees;
  final String? selectedWorktreeId;
  final List<String> baseBranches;
  final String? defaultBaseBranch;
}

class RemoteWorktreeController {
  const RemoteWorktreeController();

  RelayEnvelope listEnvelope(ProjectInfo project) {
    return RelayEnvelope(
      type: RemoteMessageType.worktreeList,
      payload: {
        'projectId': project.id,
        if (project.path != null) 'projectPath': project.path,
      },
    );
  }

  RelayEnvelope selectEnvelope(
    ProjectInfo project,
    RemoteWorktreeInfo worktree,
  ) {
    return RelayEnvelope(
      type: RemoteMessageType.worktreeSelect,
      payload: {
        'projectId': project.id,
        'worktreeId': worktree.id,
        if (project.path != null) 'projectPath': project.path,
      },
    );
  }

  RelayEnvelope createEnvelope({
    required ProjectInfo project,
    required String baseBranch,
    required String name,
  }) {
    return RelayEnvelope(
      type: RemoteMessageType.worktreeCreate,
      payload: {
        'projectId': project.id,
        'projectPath': project.path,
        'baseBranch': baseBranch,
        'branchName': name,
        'taskTitle': name,
      },
    );
  }

  RelayEnvelope mergeEnvelope(
    ProjectInfo project,
    RemoteWorktreeInfo worktree,
  ) {
    return _operationEnvelope(
      type: RemoteMessageType.worktreeMerge,
      project: project,
      worktree: worktree,
      removeBranch: false,
    );
  }

  RelayEnvelope deleteEnvelope(
    ProjectInfo project,
    RemoteWorktreeInfo worktree,
  ) {
    return _operationEnvelope(
      type: RemoteMessageType.worktreeDelete,
      project: project,
      worktree: worktree,
      removeBranch: true,
    );
  }

  RemoteWorktreeState? stateFromPayload(Object? payload) {
    if (payload is! Map) return null;
    return RemoteWorktreeState(
      worktrees: remoteWorktreesFromPayload(payload),
      selectedWorktreeId: payload['selectedWorktreeId']?.toString(),
      baseBranches: stringListPayload(payload['baseBranches']),
      defaultBaseBranch: payload['defaultBaseBranch']?.toString(),
    );
  }

  RelayEnvelope _operationEnvelope({
    required String type,
    required ProjectInfo project,
    required RemoteWorktreeInfo worktree,
    required bool removeBranch,
  }) {
    return RelayEnvelope(
      type: type,
      payload: {
        'projectId': project.id,
        'projectPath': project.path,
        'worktreePath': worktree.path,
        'worktreeId': worktree.id,
        'removeBranch': removeBranch,
      },
    );
  }
}

List<String> worktreeBranchOptions({
  required String? defaultBaseBranch,
  required List<String> baseBranches,
  required List<RemoteWorktreeInfo> worktrees,
}) {
  final values = <String>[];
  void push(String? value) {
    final branch = value?.trim() ?? '';
    if (branch.isEmpty || values.contains(branch)) return;
    values.add(branch);
  }

  push(defaultBaseBranch);
  for (final branch in baseBranches) {
    push(branch);
  }
  for (final worktree in worktrees) {
    push(worktree.branch);
  }
  return values;
}

String defaultWorktreeBaseBranch({
  required String? preferred,
  required List<String> options,
}) {
  final value = preferred?.trim() ?? '';
  if (value.isNotEmpty && options.contains(value)) return value;
  return options.isNotEmpty ? options.first : '';
}

String worktreeTitle(RemoteWorktreeInfo worktree) {
  final name = worktree.name.trim();
  if (name.isNotEmpty) return name;
  final branch = worktree.branch.trim();
  if (branch.isNotEmpty) return branch;
  return worktree.path;
}

List<String> stringListPayload(Object? value) {
  if (value is! List) return const [];
  return value
      .map((item) => '$item'.trim())
      .where((item) => item.isNotEmpty)
      .toSet()
      .toList(growable: false);
}
