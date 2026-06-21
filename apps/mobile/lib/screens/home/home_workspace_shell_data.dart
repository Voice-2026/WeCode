import '../../models/remote_models.dart';

class WorkspaceShellData {
  const WorkspaceShellData({
    required this.terminals,
    required this.worktrees,
    required this.aiStats,
    required this.aiStatsLoading,
    required this.gitStatus,
    required this.currentSessions,
    required this.aiSessions,
    required this.sshProfiles,
    required this.projectFilesPath,
    required this.projectFilesParent,
    required this.projectFileEntries,
    required this.projectFilesLoading,
  });

  final List<TerminalInfo> terminals;
  final List<RemoteWorktreeInfo> worktrees;
  final AIStatsInfo? aiStats;
  final bool aiStatsLoading;
  final RemoteGitStatusInfo? gitStatus;
  final List<AIStatsSessionInfo> currentSessions;
  final List<AISessionRecord> aiSessions;
  final List<RemoteSshProfile> sshProfiles;
  final String projectFilesPath;
  final String? projectFilesParent;
  final List<RemoteFileEntry> projectFileEntries;
  final bool projectFilesLoading;
}
