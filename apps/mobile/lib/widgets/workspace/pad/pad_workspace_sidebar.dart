import 'package:flutter/material.dart';

import '../../../i18n.dart';
import '../../../models/remote_models.dart';
import '../../../theme/app_theme.dart';
import 'pad_project_picker_modal.dart';
import 'pad_theme.dart';
import 'pad_workspace_shared.dart';

class PadWorkspaceSidebar extends StatelessWidget {
  const PadWorkspaceSidebar({
    super.key,
    required this.project,
    required this.projects,
    required this.selectedProjectId,
    required this.worktrees,
    required this.selectedWorktreeId,
    required this.terminals,
    required this.activeTerminalId,
    required this.aiSessions,
    required this.onSelectProject,
    required this.onEditProject,
    required this.onAddProject,
    required this.onRemoveProject,
    required this.onSelectWorktree,
    required this.onCreateWorktree,
    required this.onSelectTerminal,
    required this.onCreateTerminal,
    required this.onCloseTerminal,
    required this.onRenameSession,
  });

  final ProjectInfo? project;
  final List<ProjectInfo> projects;
  final String? selectedProjectId;
  final List<RemoteWorktreeInfo> worktrees;
  final String? selectedWorktreeId;
  final List<TerminalInfo> terminals;
  final String? activeTerminalId;
  final List<AISessionRecord> aiSessions;
  final ValueChanged<ProjectInfo> onSelectProject;
  final VoidCallback onEditProject;
  final VoidCallback onAddProject;
  final VoidCallback onRemoveProject;
  final ValueChanged<RemoteWorktreeInfo> onSelectWorktree;
  final VoidCallback onCreateWorktree;
  final ValueChanged<TerminalInfo> onSelectTerminal;
  final VoidCallback onCreateTerminal;
  final ValueChanged<TerminalInfo> onCloseTerminal;
  final ValueChanged<TerminalInfo> onRenameSession;

  @override
  Widget build(BuildContext context) {
    final prefs = AppPreferences.of(context);
    final accent = Theme.of(context).colorScheme.secondary;
    return Container(
      color: PadColors.panel,
      child: Column(
        children: [
          _HeaderBar(
            title: project?.name ?? prefs.t('app.noProjects'),
            onTap: () => showPadProjectPicker(
              context,
              projects: projects,
              selectedProjectId: selectedProjectId,
              onSelectProject: onSelectProject,
              onAddProject: onAddProject,
            ),
            trailing: const Icon(
              Icons.expand_more_rounded,
              size: 20,
              color: PadColors.textMuted,
            ),
          ),
          Expanded(
            flex: 5,
            child: worktrees.isEmpty
                ? _EmptyHint(text: prefs.t('worktree.empty'))
                : ListView.separated(
                    padding: const EdgeInsets.fromLTRB(8, 8, 8, 12),
                    itemCount: worktrees.length,
                    separatorBuilder: (_, _) => const SizedBox(height: 6),
                    itemBuilder: (context, index) {
                      final item = worktrees[index];
                      return _WorktreeRow(
                        info: item,
                        active: item.id == selectedWorktreeId,
                        accent: accent,
                        onTap: () => onSelectWorktree(item),
                      );
                    },
                  ),
          ),
          _HeaderBar(
            title: prefs.t('workspace.sessions'),
            onTap: null,
            trailing: const SizedBox.shrink(),
          ),
          Expanded(
            flex: 6,
            child: aiSessions.isNotEmpty
                ? ListView.separated(
                    padding: const EdgeInsets.fromLTRB(8, 8, 8, 12),
                    itemCount: aiSessions.length,
                    separatorBuilder: (_, _) => const SizedBox(height: 4),
                    itemBuilder: (context, index) =>
                        _HistorySessionRow(session: aiSessions[index]),
                  )
                : _EmptyHint(text: prefs.t('workspace.sessionsEmpty')),
          ),
        ],
      ),
    );
  }
}

/// Darker section bar (project switcher / sessions header) — a touch darker than
/// the panel so it reads as a header strip.
class _HeaderBar extends StatelessWidget {
  const _HeaderBar({
    required this.title,
    required this.onTap,
    required this.trailing,
  });

  final String title;
  final VoidCallback? onTap;
  final Widget trailing;

  @override
  Widget build(BuildContext context) {
    return InkWell(
      onTap: onTap,
      child: Container(
        height: 48,
        color: PadColors.header,
        padding: const EdgeInsets.only(left: 14, right: 8),
        child: Row(
          children: [
            Expanded(
              child: Text(
                title,
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
                style: const TextStyle(
                  color: PadColors.textPrimary,
                  fontSize: 15,
                  fontWeight: FontWeight.w700,
                ),
              ),
            ),
            trailing,
          ],
        ),
      ),
    );
  }
}

class _EmptyHint extends StatelessWidget {
  const _EmptyHint({required this.text});

  final String text;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 14),
      child: Text(
        text,
        style: const TextStyle(color: PadColors.textSubtle, fontSize: 12),
      ),
    );
  }
}

class _WorktreeRow extends StatelessWidget {
  const _WorktreeRow({
    required this.info,
    required this.active,
    required this.accent,
    required this.onTap,
  });

  final RemoteWorktreeInfo info;
  final bool active;
  final Color accent;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    return Material(
      color: active ? accent.withValues(alpha: 0.16) : Colors.transparent,
      borderRadius: BorderRadius.circular(12),
      child: InkWell(
        borderRadius: BorderRadius.circular(12),
        onTap: onTap,
        child: Padding(
          padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 9),
          child: Row(
            children: [
              _Avatar(label: info.name, active: active, accent: accent),
              const SizedBox(width: AppSpacing.s),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(
                      info.name,
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      style: TextStyle(
                        color: active
                            ? PadColors.textPrimary
                            : PadColors.textSecondary,
                        fontSize: 13,
                        fontWeight: FontWeight.w700,
                      ),
                    ),
                    const SizedBox(height: 3),
                    Text(
                      info.path,
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      style: const TextStyle(
                        color: PadColors.textMuted,
                        fontSize: 11,
                      ),
                    ),
                  ],
                ),
              ),
              if (info.branch.trim().isNotEmpty) ...[
                const SizedBox(width: 8),
                _BranchBadge(
                  branch: info.branch.trim(),
                  accent: accent,
                  active: active,
                ),
              ],
            ],
          ),
        ),
      ),
    );
  }
}

class _Avatar extends StatelessWidget {
  const _Avatar({
    required this.label,
    required this.active,
    required this.accent,
  });

  final String label;
  final bool active;
  final Color accent;

  @override
  Widget build(BuildContext context) {
    return Container(
      width: 32,
      height: 32,
      decoration: BoxDecoration(
        color: active ? accent : PadColors.cardActive,
        borderRadius: BorderRadius.circular(AppRadius.sm),
      ),
      alignment: Alignment.center,
      child: Text(
        projectInitials(label),
        style: TextStyle(
          color: active ? Colors.white : PadColors.textSecondary,
          fontSize: 12,
          fontWeight: FontWeight.w800,
        ),
      ),
    );
  }
}

class _BranchBadge extends StatelessWidget {
  const _BranchBadge({
    required this.branch,
    required this.accent,
    required this.active,
  });

  final String branch;
  final Color accent;
  final bool active;

  @override
  Widget build(BuildContext context) {
    return Container(
      constraints: const BoxConstraints(maxWidth: 92),
      padding: const EdgeInsets.symmetric(horizontal: 7, vertical: 3),
      decoration: BoxDecoration(
        color: active ? accent.withValues(alpha: 0.16) : PadColors.cardActive,
        borderRadius: BorderRadius.circular(6),
      ),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          Icon(
            Icons.alt_route_rounded,
            size: 11,
            color: active ? accent : PadColors.textMuted,
          ),
          const SizedBox(width: 4),
          Flexible(
            child: Text(
              branch,
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
              style: TextStyle(
                color: active ? accent : PadColors.textMuted,
                fontSize: 10,
                fontWeight: FontWeight.w700,
              ),
            ),
          ),
        ],
      ),
    );
  }
}

/// Session-history item from `ai.session` (mirrors the desktop "会话记录" list).
class _HistorySessionRow extends StatelessWidget {
  const _HistorySessionRow({required this.session});

  final AISessionRecord session;

  @override
  Widget build(BuildContext context) {
    final title = session.title.trim().isNotEmpty ? session.title.trim() : session.id;
    final time = formatEpochSeconds(session.time);
    final tool = session.tool.trim();
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 8),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            children: [
              Expanded(
                child: Text(
                  title,
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: const TextStyle(
                    color: PadColors.textPrimary,
                    fontSize: 13,
                    fontWeight: FontWeight.w700,
                  ),
                ),
              ),
              if (session.size > 0) ...[
                const SizedBox(width: 8),
                Text(
                  formatTokenSize(session.size),
                  style: const TextStyle(
                    color: PadColors.textSubtle,
                    fontSize: 11,
                    fontWeight: FontWeight.w800,
                  ),
                ),
              ],
            ],
          ),
          if (tool.isNotEmpty || time.isNotEmpty) ...[
            const SizedBox(height: 3),
            // Second line: tool on the left, time on the right (two-ends aligned).
            Row(
              children: [
                Expanded(
                  child: Text(
                    tool,
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    style: const TextStyle(
                      color: PadColors.textMuted,
                      fontSize: 11,
                    ),
                  ),
                ),
                if (time.isNotEmpty) ...[
                  const SizedBox(width: 8),
                  Text(
                    time,
                    style: const TextStyle(
                      color: PadColors.textSubtle,
                      fontSize: 11,
                    ),
                  ),
                ],
              ],
            ),
          ],
        ],
      ),
    );
  }
}
