import 'package:flutter/material.dart';

import '../../../i18n.dart';
import '../../../models/remote_models.dart';
import 'pad_theme.dart';
import 'pad_tool_panels.dart';
import '../../ai_stats_panel.dart';
import '../../project_files_panel.dart';

/// Contextual right column. A borderless, self-rounded surface with a unified
/// header on top (matching the sidebar header height) and a scrollable panel
/// below. Shows the file tree in "files" mode and AI stats in "stats" mode.
class PadRightColumn extends StatelessWidget {
  const PadRightColumn({
    super.key,
    required this.mode,
    required this.aiStats,
    required this.aiStatsLoading,
    required this.onShowStats,
    required this.gitStatus,
    required this.onGitAction,
    required this.sshProfiles,
    required this.reviewSelectedPath,
    required this.onSelectReviewFile,
    required this.projectFilesPath,
    required this.projectFilesParent,
    required this.projectFileEntries,
    required this.projectFilesLoading,
    required this.onRequestProjectFiles,
    required this.onOpenProjectFile,
    required this.onOpenProjectHome,
    required this.onOpenProjectRoot,
    required this.onOpenProjectVolumes,
    required this.onRenameProjectFile,
    required this.onCopyProjectFilePath,
    required this.onDeleteProjectFile,
  });

  final String mode;
  final AIStatsInfo? aiStats;
  final bool aiStatsLoading;
  final VoidCallback onShowStats;
  final RemoteGitStatusInfo? gitStatus;
  final void Function(String op, Map<String, dynamic> args) onGitAction;
  final List<RemoteSshProfile> sshProfiles;
  final String? reviewSelectedPath;
  final ValueChanged<String> onSelectReviewFile;
  final String projectFilesPath;
  final String? projectFilesParent;
  final List<RemoteFileEntry> projectFileEntries;
  final bool projectFilesLoading;
  final ValueChanged<String> onRequestProjectFiles;
  final ValueChanged<RemoteFileEntry> onOpenProjectFile;
  final VoidCallback onOpenProjectHome;
  final VoidCallback onOpenProjectRoot;
  final VoidCallback onOpenProjectVolumes;
  final ValueChanged<RemoteFileEntry> onRenameProjectFile;
  final ValueChanged<RemoteFileEntry> onCopyProjectFilePath;
  final ValueChanged<RemoteFileEntry> onDeleteProjectFile;

  @override
  Widget build(BuildContext context) {
    final prefs = AppPreferences.of(context);
    if (mode == 'stats') {
      return SizedBox(
        width: PadMetrics.rightColumnWidth,
        child: AIStatsPanel(
          stats: aiStats,
          loading: aiStatsLoading,
          onRefresh: onShowStats,
          title: prefs.t('workspace.stats'),
          contentPadding: EdgeInsets.zero,
          cardBordered: true,
          colors: PadColors.statsPanel,
        ),
      );
    }
    if (mode == 'review') {
      final changes = [
        for (final file in gitStatus?.changedFiles ?? const <RemoteGitFileStatus>[])
          _ReviewChangeEntry(
            _reviewStatusCode(file),
            file.path,
            0,
            0,
          ),
      ];
      return PadPanelSurface(
        width: PadMetrics.rightColumnWidth,
        child: Column(
          children: [
            _ColumnHeader(title: prefs.t('workspace.review')),
            Expanded(
              child: _ReviewFileTree(
                changes: changes,
                selectedPath: reviewSelectedPath,
                onSelect: onSelectReviewFile,
              ),
            ),
          ],
        ),
      );
    }
    if (mode == 'ssh') {
      return PadSshToolPanel(profiles: sshProfiles);
    }
    if (mode == 'git') {
      return PadGitToolPanel(gitStatus: gitStatus, onAction: onGitAction);
    }
    return PadPanelSurface(
      width: PadMetrics.rightColumnWidth,
      child: Column(
        children: [
          _ColumnHeader(
            title: prefs.t('workspace.files'),
            trailing: ProjectFilesPanelActions(
              onRefresh: () => onRequestProjectFiles(projectFilesPath),
              onOpenHome: onOpenProjectHome,
              onOpenRoot: onOpenProjectRoot,
              onOpenVolumes: onOpenProjectVolumes,
              dense: true,
              menuColor: PadColors.panel,
              plain: true,
            ),
          ),
          Expanded(child: _files()),
        ],
      ),
    );
  }

  Widget _files() {
    return ProjectFilesPanel(
      path: projectFilesPath,
      parent: projectFilesParent,
      entries: projectFileEntries,
      loading: projectFilesLoading,
      onOpenPath: onRequestProjectFiles,
      onOpenFile: onOpenProjectFile,
      onRefresh: () => onRequestProjectFiles(projectFilesPath),
      onOpenHome: onOpenProjectHome,
      onOpenRoot: onOpenProjectRoot,
      onOpenVolumes: onOpenProjectVolumes,
      onRename: onRenameProjectFile,
      onCopyPath: onCopyProjectFilePath,
      onDelete: onDeleteProjectFile,
      showTopBar: false,
      showFooterPath: true,
      highlightMenuRows: false,
    );
  }
}

class _ReviewFileTree extends StatefulWidget {
  const _ReviewFileTree({
    required this.changes,
    required this.selectedPath,
    required this.onSelect,
  });

  final List<_ReviewChangeEntry> changes;
  final String? selectedPath;
  final ValueChanged<String> onSelect;

  @override
  State<_ReviewFileTree> createState() => _ReviewFileTreeState();
}

class _ReviewFileTreeState extends State<_ReviewFileTree> {
  String _currentPath = '';

  @override
  Widget build(BuildContext context) {
    final accent = Theme.of(context).colorScheme.secondary;
    final prefs = AppPreferences.of(context);
    final snapshot = _ReviewDirectorySnapshot.from(
      _currentPath,
      widget.changes,
    );
    final parentPath = _currentPath.isEmpty
        ? null
        : _parentReviewPath(_currentPath);
    final rows = <Widget>[
      if (parentPath != null)
        _ReviewParentRow(
          label: prefs.t('project.parentDir'),
          path: parentPath,
          accent: accent,
          onTap: () => setState(() => _currentPath = parentPath),
        ),
      for (final folder in snapshot.folders)
        _ReviewFolderRow(
          folder: folder,
          accent: accent,
          onTap: () => setState(() => _currentPath = folder.path),
        ),
      for (final file in snapshot.files)
        _ReviewFileRow(
          file: file,
          accent: accent,
          selected: widget.selectedPath == file.path,
          onTap: () => widget.onSelect(file.path),
        ),
    ];

    return ColoredBox(
      color: PadColors.panel,
      child: Column(
        children: [
          _ReviewPathStrip(path: _currentPath),
          Expanded(
            child: ListView.separated(
              physics: const BouncingScrollPhysics(),
              padding: const EdgeInsets.symmetric(vertical: 6),
              itemCount: rows.length,
              separatorBuilder: (_, _) => const Divider(
                height: 0.5,
                thickness: 0.5,
                color: PadColors.border,
              ),
              itemBuilder: (context, index) => rows[index],
            ),
          ),
        ],
      ),
    );
  }
}

/// Single-letter status from a git file's index/worktree status codes.
String _reviewStatusCode(RemoteGitFileStatus file) {
  final index = file.indexStatus.trim();
  final worktree = file.worktreeStatus.trim();
  if (index == '?' || worktree == '?') return 'A';
  final code = index.isNotEmpty ? index : worktree;
  return code.isEmpty ? 'M' : code;
}

class _ReviewDirectorySnapshot {
  const _ReviewDirectorySnapshot({required this.folders, required this.files});

  final List<_ReviewFolderNode> folders;
  final List<_ReviewChangeEntry> files;

  static _ReviewDirectorySnapshot from(
    String basePath,
    List<_ReviewChangeEntry> changes,
  ) {
    final folders = <String, _ReviewFolderNode>{};
    final files = <_ReviewChangeEntry>[];

    for (final change in changes) {
      final relativePath = _relativeReviewPath(basePath, change.path);
      if (relativePath == null || relativePath.isEmpty) {
        continue;
      }
      final slashIndex = relativePath.indexOf('/');
      if (slashIndex < 0) {
        files.add(change);
        continue;
      }
      final folderName = relativePath.substring(0, slashIndex);
      final folderPath = _joinReviewPath(basePath, folderName);
      folders
          .putIfAbsent(
            folderName,
            () => _ReviewFolderNode(name: folderName, path: folderPath),
          )
          .add(change);
    }

    final sortedFolders = folders.values.toList()
      ..sort((left, right) => left.name.compareTo(right.name));
    files.sort((left, right) => left.name.compareTo(right.name));
    return _ReviewDirectorySnapshot(folders: sortedFolders, files: files);
  }
}

class _ReviewFolderNode {
  _ReviewFolderNode({required this.name, required this.path});

  final String name;
  final String path;
  int count = 0;
  int additions = 0;
  int deletions = 0;

  void add(_ReviewChangeEntry change) {
    count += 1;
    additions += change.additions;
    deletions += change.deletions;
  }
}

class _ReviewChangeEntry {
  const _ReviewChangeEntry(
    this.status,
    this.path,
    this.additions,
    this.deletions,
  );

  final String status;
  final String path;
  final int additions;
  final int deletions;

  String get name {
    final parts = path.split('/');
    return parts.isEmpty ? path : parts.last;
  }

  String get parent {
    final index = path.lastIndexOf('/');
    return index <= 0 ? '' : path.substring(0, index);
  }
}

class _ReviewPathStrip extends StatelessWidget {
  const _ReviewPathStrip({required this.path});

  final String path;

  @override
  Widget build(BuildContext context) {
    final displayPath = path.isEmpty ? 'codux-gpui' : path;
    return Container(
      height: 40,
      decoration: const BoxDecoration(
        color: PadColors.header,
        border: Border(bottom: BorderSide(color: PadColors.border, width: 0.5)),
      ),
      padding: const EdgeInsets.symmetric(horizontal: 12),
      child: Row(
        children: [
          const Icon(
            Icons.account_tree_rounded,
            size: 16,
            color: PadColors.textMuted,
          ),
          const SizedBox(width: 8),
          Expanded(
            child: Text(
              displayPath,
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
              style: const TextStyle(
                color: PadColors.textSecondary,
                fontSize: 12,
                fontWeight: FontWeight.w600,
              ),
            ),
          ),
          const SizedBox(width: 8),
          const Icon(
            Icons.more_horiz_rounded,
            size: 18,
            color: PadColors.textSubtle,
          ),
        ],
      ),
    );
  }
}

class _ReviewParentRow extends StatelessWidget {
  const _ReviewParentRow({
    required this.label,
    required this.path,
    required this.accent,
    required this.onTap,
  });

  final String label;
  final String path;
  final Color accent;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    return _ReviewRowShell(
      onTap: onTap,
      child: Row(
        children: [
          Icon(Icons.arrow_upward_rounded, color: accent, size: 20),
          const SizedBox(width: 10),
          Expanded(
            child: _ReviewTitleBlock(title: label, subtitle: path),
          ),
          const SizedBox(width: 8),
          const Icon(
            Icons.keyboard_return_rounded,
            size: 17,
            color: PadColors.textSubtle,
          ),
        ],
      ),
    );
  }
}

class _ReviewFolderRow extends StatelessWidget {
  const _ReviewFolderRow({
    required this.folder,
    required this.accent,
    required this.onTap,
  });

  final _ReviewFolderNode folder;
  final Color accent;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    return _ReviewRowShell(
      onTap: onTap,
      child: Row(
        children: [
          Icon(Icons.folder_rounded, color: accent, size: 20),
          const SizedBox(width: 10),
          Expanded(
            child: _ReviewTitleBlock(
              title: folder.name,
              subtitle: folder.path,
              trailingLabel: '${folder.count}',
            ),
          ),
          const SizedBox(width: 8),
          _ReviewChangeTotals(
            additions: folder.additions,
            deletions: folder.deletions,
          ),
        ],
      ),
    );
  }
}

class _ReviewFileRow extends StatelessWidget {
  const _ReviewFileRow({
    required this.file,
    required this.accent,
    required this.selected,
    required this.onTap,
  });

  final _ReviewChangeEntry file;
  final Color accent;
  final bool selected;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final statusColor = _reviewStatusColor(file.status, accent);
    return _ReviewRowShell(
      onTap: onTap,
      selected: selected,
      selectedColor: accent.withValues(alpha: 0.14),
      child: Row(
        children: [
          Icon(
            _reviewFileIcon(file.status),
            color: selected ? accent : PadColors.textMuted,
            size: 20,
          ),
          const SizedBox(width: 10),
          Expanded(
            child: _ReviewTitleBlock(title: file.name, subtitle: file.parent),
          ),
          const SizedBox(width: 8),
          _ReviewStatusBadge(label: file.status, color: statusColor),
          const SizedBox(width: 8),
          _ReviewChangeTotals(
            additions: file.additions,
            deletions: file.deletions,
          ),
        ],
      ),
    );
  }
}

class _ReviewRowShell extends StatelessWidget {
  const _ReviewRowShell({
    required this.child,
    required this.onTap,
    this.selected = false,
    this.selectedColor,
  });

  final Widget child;
  final VoidCallback onTap;
  final bool selected;
  final Color? selectedColor;

  @override
  Widget build(BuildContext context) {
    return InkWell(
      onTap: onTap,
      child: AnimatedContainer(
        duration: const Duration(milliseconds: 120),
        curve: Curves.easeOutCubic,
        color: selected ? selectedColor : Colors.transparent,
        padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 10),
        child: child,
      ),
    );
  }
}

class _ReviewTitleBlock extends StatelessWidget {
  const _ReviewTitleBlock({
    required this.title,
    required this.subtitle,
    this.trailingLabel,
  });

  final String title;
  final String subtitle;
  final String? trailingLabel;

  @override
  Widget build(BuildContext context) {
    return Column(
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
            if (trailingLabel != null) ...[
              const SizedBox(width: 6),
              Container(
                height: 18,
                constraints: const BoxConstraints(minWidth: 22),
                alignment: Alignment.center,
                padding: const EdgeInsets.symmetric(horizontal: 6),
                decoration: BoxDecoration(
                  color: PadColors.cardActive,
                  borderRadius: BorderRadius.circular(6),
                ),
                child: Text(
                  trailingLabel!,
                  style: const TextStyle(
                    color: PadColors.textMuted,
                    fontSize: 10.5,
                    fontWeight: FontWeight.w700,
                  ),
                ),
              ),
            ],
          ],
        ),
        const SizedBox(height: 3),
        Text(
          subtitle.isEmpty ? '.' : subtitle,
          textDirection: TextDirection.rtl,
          textAlign: TextAlign.right,
          maxLines: 1,
          overflow: TextOverflow.ellipsis,
          style: const TextStyle(color: PadColors.textSubtle, fontSize: 11),
        ),
      ],
    );
  }
}

class _ReviewStatusBadge extends StatelessWidget {
  const _ReviewStatusBadge({required this.label, required this.color});

  final String label;
  final Color color;

  @override
  Widget build(BuildContext context) {
    return Container(
      width: 24,
      height: 24,
      alignment: Alignment.center,
      decoration: BoxDecoration(
        color: color.withValues(alpha: 0.14),
        borderRadius: BorderRadius.circular(6),
      ),
      child: Text(
        label,
        style: TextStyle(
          color: color,
          fontSize: 11,
          fontWeight: FontWeight.w800,
        ),
      ),
    );
  }
}

class _ReviewChangeTotals extends StatelessWidget {
  const _ReviewChangeTotals({required this.additions, required this.deletions});

  final int additions;
  final int deletions;

  @override
  Widget build(BuildContext context) {
    return SizedBox(
      width: 46,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.end,
        children: [
          Text(
            '+$additions',
            style: const TextStyle(
              color: PadColors.success,
              fontSize: 11,
              fontWeight: FontWeight.w700,
            ),
          ),
          const SizedBox(height: 3),
          Text(
            '-$deletions',
            style: const TextStyle(
              color: PadColors.danger,
              fontSize: 11,
              fontWeight: FontWeight.w700,
            ),
          ),
        ],
      ),
    );
  }
}

String? _parentReviewPath(String path) {
  if (path.isEmpty) {
    return null;
  }
  final index = path.lastIndexOf('/');
  return index < 0 ? '' : path.substring(0, index);
}

String? _relativeReviewPath(String basePath, String path) {
  if (basePath.isEmpty) {
    return path;
  }
  final prefix = '$basePath/';
  if (!path.startsWith(prefix)) {
    return null;
  }
  return path.substring(prefix.length);
}

String _joinReviewPath(String basePath, String child) {
  return basePath.isEmpty ? child : '$basePath/$child';
}

Color _reviewStatusColor(String status, Color accent) {
  return switch (status) {
    'A' => PadColors.success,
    'D' => PadColors.danger,
    'R' => PadColors.warning,
    _ => accent,
  };
}

IconData _reviewFileIcon(String status) {
  return switch (status) {
    'A' => Icons.note_add_rounded,
    'D' => Icons.note_alt_outlined,
    'R' => Icons.drive_file_rename_outline_rounded,
    _ => Icons.description_outlined,
  };
}

/// Unified column header — same height as the sidebar header bars.
class _ColumnHeader extends StatelessWidget {
  const _ColumnHeader({required this.title, this.trailing});

  final String title;
  final Widget? trailing;

  @override
  Widget build(BuildContext context) {
    return Container(
      height: 48,
      color: PadColors.header,
      padding: const EdgeInsets.symmetric(horizontal: 14),
      alignment: Alignment.centerLeft,
      child: Row(
        children: [
          Expanded(
            child: Text(
              title,
              style: const TextStyle(
                color: PadColors.textPrimary,
                fontSize: 15,
                fontWeight: FontWeight.w700,
              ),
            ),
          ),
          ?trailing,
        ],
      ),
    );
  }
}
