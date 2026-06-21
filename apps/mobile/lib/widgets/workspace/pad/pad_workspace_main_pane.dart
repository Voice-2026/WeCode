import 'package:flutter/material.dart';

import '../../../i18n.dart';
import '../../../models/remote_models.dart';
import '../../project_files_panel.dart';
import 'pad_theme.dart';

/// Center workspace: the terminal tab strip and the terminal body. View
/// switching and the contextual panels (files / AI stats) live in the top bar
/// and right column respectively.
class PadWorkspaceMainPane extends StatelessWidget {
  const PadWorkspaceMainPane({
    super.key,
    required this.terminals,
    required this.activeTerminalId,
    required this.workspaceMode,
    required this.terminalBody,
    required this.gitDiff,
    required this.reviewSelectedPath,
    required this.editingFilePath,
    required this.fileEditorController,
    required this.fileEditorLoading,
    required this.fileEditorSaving,
    required this.fileEditorEditing,
    required this.fileEditorEditable,
    required this.onEditFile,
    required this.onSaveFile,
    required this.onCloseFileEditor,
    required this.onSelectTerminal,
    required this.onCreateTerminal,
    required this.onCloseTerminal,
  });

  final List<TerminalInfo> terminals;
  final String? activeTerminalId;
  final String workspaceMode;
  final Widget terminalBody;
  final RemoteGitDiff? gitDiff;
  final String? reviewSelectedPath;
  final String? editingFilePath;
  final TextEditingController fileEditorController;
  final bool fileEditorLoading;
  final bool fileEditorSaving;
  final bool fileEditorEditing;
  final bool fileEditorEditable;
  final VoidCallback onEditFile;
  final VoidCallback onSaveFile;
  final VoidCallback onCloseFileEditor;
  final ValueChanged<TerminalInfo> onSelectTerminal;
  final VoidCallback onCreateTerminal;
  final ValueChanged<TerminalInfo> onCloseTerminal;

  @override
  Widget build(BuildContext context) {
    if (workspaceMode == 'files') {
      if (editingFilePath != null) {
        return FileEditorView(
          path: editingFilePath!,
          controller: fileEditorController,
          loading: fileEditorLoading,
          saving: fileEditorSaving,
          editing: fileEditorEditing,
          editable: fileEditorEditable,
          onClose: onCloseFileEditor,
          onEdit: onEditFile,
          onSave: onSaveFile,
        );
      }
      return _PadEditorEmpty(
        text: AppPreferences.of(context).t('file.selectToOpen'),
      );
    }
    if (workspaceMode == 'review') {
      return PadDiffView(diff: gitDiff, path: reviewSelectedPath);
    }
    return Column(
      children: [
        _PadTerminalTabs(
          terminals: terminals,
          activeTerminalId: activeTerminalId,
          onSelectTerminal: onSelectTerminal,
          onCreateTerminal: onCreateTerminal,
          onCloseTerminal: onCloseTerminal,
        ),
        Expanded(child: terminalBody),
      ],
    );
  }
}

/// Renders the unified diff for the selected review file (from `git.read diff`).
class PadDiffView extends StatelessWidget {
  const PadDiffView({super.key, required this.diff, required this.path});

  final RemoteGitDiff? diff;
  final String? path;

  @override
  Widget build(BuildContext context) {
    final accent = Theme.of(context).colorScheme.secondary;
    if (path == null) {
      return const _DiffEmpty(text: 'Select a file to view its diff');
    }
    return Column(
      children: [
        Container(
          height: 44,
          color: PadColors.panel,
          padding: const EdgeInsets.symmetric(horizontal: 14),
          alignment: Alignment.centerLeft,
          child: Row(
            children: [
              const Icon(Icons.difference_rounded, size: 15, color: PadColors.textMuted),
              const SizedBox(width: 8),
              Expanded(
                child: Text(
                  path!,
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: const TextStyle(
                    color: PadColors.textSecondary,
                    fontSize: 12.5,
                    fontWeight: FontWeight.w600,
                  ),
                ),
              ),
            ],
          ),
        ),
        Expanded(
          child: diff == null
              ? Center(child: CircularProgressIndicator(color: accent))
              : (diff!.diff.trim().isEmpty
                    ? const _DiffEmpty(text: 'No changes')
                    : _DiffBody(diff: diff!.diff, accent: accent)),
        ),
      ],
    );
  }
}

class _DiffEmpty extends StatelessWidget {
  const _DiffEmpty({required this.text});

  final String text;

  @override
  Widget build(BuildContext context) {
    return ColoredBox(
      color: PadColors.bg,
      child: Center(
        child: Text(
          text,
          style: const TextStyle(color: PadColors.textSubtle, fontSize: 13),
        ),
      ),
    );
  }
}

/// Empty state shown in the center pane when files mode is active but no file
/// is open yet (files are opened one at a time from the right-column tree, so
/// there is no editor tab strip — the open file shows its title in its header).
class _PadEditorEmpty extends StatelessWidget {
  const _PadEditorEmpty({required this.text});

  final String text;

  @override
  Widget build(BuildContext context) {
    return ColoredBox(
      color: PadColors.bg,
      child: Center(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            const Icon(
              Icons.description_outlined,
              size: 34,
              color: PadColors.textSubtle,
            ),
            const SizedBox(height: 12),
            Text(
              text,
              style: const TextStyle(color: PadColors.textSubtle, fontSize: 13),
            ),
          ],
        ),
      ),
    );
  }
}

class _DiffBody extends StatelessWidget {
  const _DiffBody({required this.diff, required this.accent});

  final String diff;
  final Color accent;

  @override
  Widget build(BuildContext context) {
    final lines = diff.split('\n');
    return ColoredBox(
      color: PadColors.bg,
      child: SingleChildScrollView(
        scrollDirection: Axis.vertical,
        child: SingleChildScrollView(
          scrollDirection: Axis.horizontal,
          child: Padding(
            padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                for (final line in lines)
                  Text(
                    line.isEmpty ? ' ' : line,
                    style: TextStyle(
                      fontFamily: 'MapleMonoNFCN',
                      fontSize: 12,
                      height: 1.4,
                      color: _diffLineColor(line, accent),
                    ),
                  ),
              ],
            ),
          ),
        ),
      ),
    );
  }

  Color _diffLineColor(String line, Color accent) {
    if (line.startsWith('+') && !line.startsWith('+++')) return PadColors.success;
    if (line.startsWith('-') && !line.startsWith('---')) return PadColors.danger;
    if (line.startsWith('@@')) return accent;
    if (line.startsWith('diff ') || line.startsWith('index ')) {
      return PadColors.textMuted;
    }
    return PadColors.textSecondary;
  }
}

class _PadTerminalTabs extends StatelessWidget {
  const _PadTerminalTabs({
    required this.terminals,
    required this.activeTerminalId,
    required this.onSelectTerminal,
    required this.onCreateTerminal,
    required this.onCloseTerminal,
  });

  final List<TerminalInfo> terminals;
  final String? activeTerminalId;
  final ValueChanged<TerminalInfo> onSelectTerminal;
  final VoidCallback onCreateTerminal;
  final ValueChanged<TerminalInfo> onCloseTerminal;

  @override
  Widget build(BuildContext context) {
    return Container(
      height: 48,
      color: PadColors.header,
      child: Row(
        children: [
          Expanded(
            child: ListView.builder(
              scrollDirection: Axis.horizontal,
              itemCount: terminals.length,
              itemBuilder: (context, index) {
                final terminal = terminals[index];
                final active = terminal.id == activeTerminalId;
                final title = terminal.title.trim().isNotEmpty
                    ? terminal.title.trim()
                    : terminal.id;
                return _TerminalTab(
                  title: title,
                  active: active,
                  onTap: () => onSelectTerminal(terminal),
                  onClose: () => onCloseTerminal(terminal),
                );
              },
            ),
          ),
          _TabBarAction(icon: Icons.add_rounded, onTap: onCreateTerminal),
          const _TabBarAction(icon: Icons.more_horiz_rounded),
          const SizedBox(width: 6),
        ],
      ),
    );
  }
}

class _TerminalTab extends StatelessWidget {
  const _TerminalTab({
    required this.title,
    required this.active,
    required this.onTap,
    required this.onClose,
  });

  final String title;
  final bool active;
  final VoidCallback onTap;
  final VoidCallback onClose;

  @override
  Widget build(BuildContext context) {
    final accent = Theme.of(context).colorScheme.secondary;
    return InkWell(
      onTap: onTap,
      borderRadius: const BorderRadius.vertical(top: Radius.circular(10)),
      child: Container(
        constraints: const BoxConstraints(minWidth: 160, maxWidth: 240),
        padding: const EdgeInsets.symmetric(horizontal: 14),
        decoration: BoxDecoration(
          // Selected tab uses the theme accent background.
          color: active ? accent.withValues(alpha: 0.18) : Colors.transparent,
          borderRadius: const BorderRadius.vertical(top: Radius.circular(10)),
        ),
        child: Row(
          children: [
            Icon(
              Icons.terminal_rounded,
              size: 14,
              color: active ? accent : PadColors.textMuted,
            ),
            const SizedBox(width: 8),
            Expanded(
              child: Text(
                title,
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
                style: TextStyle(
                  color: active
                      ? PadColors.textPrimary
                      : PadColors.textSecondary,
                  fontSize: 13.5,
                  fontWeight: FontWeight.w600,
                ),
              ),
            ),
            const SizedBox(width: 10),
            GestureDetector(
              onTap: onClose,
              child: Icon(
                Icons.close_rounded,
                size: 15,
                color: active ? PadColors.textSecondary : PadColors.textMuted,
              ),
            ),
          ],
        ),
      ),
    );
  }
}

class _TabBarAction extends StatelessWidget {
  const _TabBarAction({required this.icon, this.onTap});

  final IconData icon;
  final VoidCallback? onTap;

  @override
  Widget build(BuildContext context) {
    return InkWell(
      onTap: onTap,
      borderRadius: BorderRadius.circular(8),
      child: SizedBox(
        width: 40,
        height: 48,
        child: Center(child: Icon(icon, size: 18, color: PadColors.textMuted)),
      ),
    );
  }
}
