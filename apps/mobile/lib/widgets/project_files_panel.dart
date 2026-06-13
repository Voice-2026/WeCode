import 'package:flutter/material.dart';
import '../i18n.dart';
import '../models/remote_models.dart';
import '../theme/app_theme.dart';

class ProjectFilesPanel extends StatelessWidget {
  const ProjectFilesPanel({
    super.key,
    required this.path,
    required this.parent,
    required this.entries,
    required this.loading,
    required this.onOpenPath,
    required this.onOpenFile,
    required this.onRefresh,
    required this.onOpenHome,
    required this.onOpenRoot,
    required this.onOpenVolumes,
    required this.onRename,
    required this.onCopyPath,
    required this.onDelete,
  });

  final String path;
  final String? parent;
  final List<RemoteFileEntry> entries;
  final bool loading;
  final ValueChanged<String> onOpenPath;
  final ValueChanged<RemoteFileEntry> onOpenFile;
  final VoidCallback onRefresh;
  final VoidCallback onOpenHome;
  final VoidCallback onOpenRoot;
  final VoidCallback onOpenVolumes;
  final ValueChanged<RemoteFileEntry> onRename;
  final ValueChanged<RemoteFileEntry> onCopyPath;
  final ValueChanged<RemoteFileEntry> onDelete;

  @override
  Widget build(BuildContext context) {
    final accent = Theme.of(context).colorScheme.secondary;
    final prefs = AppPreferences.of(context);
    return ColoredBox(
      color: AppColors.bgBase,
      child: Column(
        children: [
          Container(
            width: double.infinity,
            padding: const EdgeInsets.fromLTRB(
              AppSpacing.l,
              AppSpacing.m,
              AppSpacing.l,
              AppSpacing.s,
            ),
            color: AppColors.bgSurface,
            child: Row(
              children: [
                Expanded(
                  child: Text(
                    path.isEmpty ? prefs.t('project.currentDir') : path,
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    style: const TextStyle(
                      color: AppColors.textMuted,
                      fontSize: AppTextSize.small,
                    ),
                  ),
                ),
                PopupMenuButton<String>(
                  icon: Icon(Icons.storage_rounded, color: accent, size: 19),
                  color: AppColors.bgSurface,
                  onSelected: (value) {
                    if (value == 'home') onOpenHome();
                    if (value == 'root') onOpenRoot();
                    if (value == 'volumes') onOpenVolumes();
                  },
                  itemBuilder: (_) => [
                    const PopupMenuItem(value: 'home', child: Text('Home')),
                    PopupMenuItem(
                      value: 'volumes',
                      child: Text(prefs.t('storage.volumes')),
                    ),
                    PopupMenuItem(
                      value: 'root',
                      child: Text(prefs.t('storage.root')),
                    ),
                  ],
                ),
                SizedBox(
                  width: 34,
                  height: 34,
                  child: IconButton(
                    padding: EdgeInsets.zero,
                    onPressed: onRefresh,
                    icon: Icon(Icons.refresh, color: accent, size: 18),
                  ),
                ),
              ],
            ),
          ),
          Expanded(
            child: loading
                ? Center(child: CircularProgressIndicator(color: accent))
                : ListView.separated(
                    physics: const BouncingScrollPhysics(),
                    padding: const EdgeInsets.only(bottom: AppSpacing.xxl),
                    itemCount: entries.length + (parent == null ? 0 : 1),
                    separatorBuilder: (_, _) => const Divider(
                      height: 0.5,
                      thickness: 0.5,
                      color: AppColors.border,
                    ),
                    itemBuilder: (context, index) {
                      if (parent != null && index == 0) {
                        return _ProjectFileRow(
                          icon: Icons.arrow_upward_rounded,
                          name: prefs.t('project.parentDir'),
                          path: parent!,
                          accent: accent,
                          onTap: () => onOpenPath(parent!),
                        );
                      }
                      final offset = parent == null ? index : index - 1;
                      final item = entries[offset];
                      return _ProjectFileRow(
                        icon: item.isDirectory
                            ? Icons.folder_rounded
                            : Icons.description_outlined,
                        name: item.name,
                        path: item.path,
                        accent: accent,
                        onTap: item.isDirectory
                            ? () => onOpenPath(item.path)
                            : () => onOpenFile(item),
                        onRename: () => onRename(item),
                        onCopyPath: () => onCopyPath(item),
                        onDelete: () => onDelete(item),
                      );
                    },
                  ),
          ),
        ],
      ),
    );
  }
}

class _ProjectFileRow extends StatefulWidget {
  const _ProjectFileRow({
    required this.icon,
    required this.name,
    required this.path,
    required this.accent,
    required this.onTap,
    this.onRename,
    this.onCopyPath,
    this.onDelete,
  });

  final IconData icon;
  final String name;
  final String path;
  final Color accent;
  final VoidCallback onTap;
  final VoidCallback? onRename;
  final VoidCallback? onCopyPath;
  final VoidCallback? onDelete;

  @override
  State<_ProjectFileRow> createState() => _ProjectFileRowState();
}

class _ProjectFileRowState extends State<_ProjectFileRow> {
  bool _menuOpen = false;

  @override
  Widget build(BuildContext context) => InkWell(
    onTap: widget.onTap,
    onLongPress:
        widget.onRename == null &&
            widget.onCopyPath == null &&
            widget.onDelete == null
        ? null
        : () => _showFileMenu(context),
    child: AnimatedContainer(
      duration: const Duration(milliseconds: 120),
      curve: Curves.easeOutCubic,
      color: _menuOpen
          ? widget.accent.withValues(alpha: 0.14)
          : Colors.transparent,
      child: Padding(
        padding: const EdgeInsets.symmetric(
          horizontal: AppSpacing.l,
          vertical: AppSpacing.m,
        ),
        child: Row(
          children: [
            Icon(widget.icon, color: widget.accent, size: 22),
            const SizedBox(width: AppSpacing.m),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    widget.name,
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    style: const TextStyle(
                      color: AppColors.textPrimary,
                      fontSize: AppTextSize.body,
                      fontWeight: FontWeight.w600,
                    ),
                  ),
                  const SizedBox(height: 2),
                  Text(
                    widget.path,
                    textAlign: TextAlign.right,
                    textDirection: TextDirection.rtl,
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    style: const TextStyle(
                      color: AppColors.textSubtle,
                      fontSize: AppTextSize.small,
                    ),
                  ),
                ],
              ),
            ),
            const Icon(
              Icons.chevron_right,
              color: AppColors.textSubtle,
              size: 18,
            ),
          ],
        ),
      ),
    ),
  );

  Future<void> _showFileMenu(BuildContext context) async {
    final prefs = AppPreferences.of(context);
    setState(() => _menuOpen = true);
    await showModalBottomSheet<void>(
      context: context,
      backgroundColor: Colors.transparent,
      builder: (context) {
        final accent = Theme.of(context).colorScheme.secondary;
        return SafeArea(
          top: false,
          child: Padding(
            padding: const EdgeInsets.fromLTRB(
              AppSpacing.m,
              0,
              AppSpacing.m,
              AppSpacing.m,
            ),
            child: Container(
              decoration: BoxDecoration(
                color: AppColors.bgSurface,
                borderRadius: BorderRadius.circular(AppRadius.lg),
                border: Border.all(color: AppColors.border, width: 0.5),
              ),
              clipBehavior: Clip.antiAlias,
              child: Column(
                mainAxisSize: MainAxisSize.min,
                children: [
                  Padding(
                    padding: const EdgeInsets.fromLTRB(
                      AppSpacing.l,
                      AppSpacing.m,
                      AppSpacing.l,
                      AppSpacing.s,
                    ),
                    child: Row(
                      children: [
                        Icon(widget.icon, color: accent, size: 22),
                        const SizedBox(width: AppSpacing.m),
                        Expanded(
                          child: Text(
                            widget.name,
                            maxLines: 1,
                            overflow: TextOverflow.ellipsis,
                            style: const TextStyle(
                              color: AppColors.textPrimary,
                              fontSize: AppTextSize.body,
                              fontWeight: FontWeight.w700,
                            ),
                          ),
                        ),
                      ],
                    ),
                  ),
                  const Divider(height: 0.5, color: AppColors.border),
                  _FileMenuItem(
                    icon: Icons.drive_file_rename_outline_rounded,
                    label: prefs.t('file.menuRename'),
                    onTap: widget.onRename,
                  ),
                  _FileMenuItem(
                    icon: Icons.content_copy_rounded,
                    label: prefs.t('file.menuCopyPath'),
                    onTap: widget.onCopyPath,
                  ),
                  _FileMenuItem(
                    icon: Icons.delete_outline_rounded,
                    label: prefs.t('file.menuDelete'),
                    danger: true,
                    onTap: widget.onDelete,
                  ),
                ],
              ),
            ),
          ),
        );
      },
    );
    if (mounted) {
      setState(() => _menuOpen = false);
    }
  }
}

class _FileMenuItem extends StatelessWidget {
  const _FileMenuItem({
    required this.icon,
    required this.label,
    required this.onTap,
    this.danger = false,
  });

  final IconData icon;
  final String label;
  final VoidCallback? onTap;
  final bool danger;

  @override
  Widget build(BuildContext context) {
    final color = danger ? AppColors.danger : AppColors.textPrimary;
    return InkWell(
      onTap: onTap == null
          ? null
          : () {
              Navigator.of(context).pop();
              onTap?.call();
            },
      child: Padding(
        padding: const EdgeInsets.symmetric(
          horizontal: AppSpacing.l,
          vertical: AppSpacing.m,
        ),
        child: Row(
          children: [
            Icon(icon, color: color, size: 20),
            const SizedBox(width: AppSpacing.m),
            Text(
              label,
              style: TextStyle(
                color: color,
                fontSize: AppTextSize.body,
                fontWeight: FontWeight.w600,
              ),
            ),
          ],
        ),
      ),
    );
  }
}

class CodeEditingController extends TextEditingController {
  CodeEditingController({super.text});
  bool highlightEnabled = true;

  static final _pattern = RegExp(
    r'''(//.*?$|#.*?$|/\*[\s\S]*?\*/|"(?:\\.|[^"\\])*"|'(?:\\.|[^'\\])*'|\b(?:class|func|function|const|let|var|final|return|if|else|for|while|switch|case|import|from|export|async|await|try|catch|throw|struct|enum|extension|public|private|static|new|true|false|null|nil)\b|\b\d+(?:\.\d+)?\b)''',
    multiLine: true,
  );

  @override
  TextSpan buildTextSpan({
    required BuildContext context,
    TextStyle? style,
    required bool withComposing,
  }) {
    final source = text;
    if (!highlightEnabled || source.length > 80000) {
      return TextSpan(style: style, text: source);
    }
    final spans = <TextSpan>[];
    var cursor = 0;
    for (final match in _pattern.allMatches(source)) {
      if (match.start > cursor) {
        spans.add(TextSpan(text: source.substring(cursor, match.start)));
      }
      final token = match.group(0) ?? '';
      spans.add(TextSpan(text: token, style: _styleFor(token)));
      cursor = match.end;
    }
    if (cursor < source.length) {
      spans.add(TextSpan(text: source.substring(cursor)));
    }
    return TextSpan(style: style, children: spans);
  }

  TextStyle _styleFor(String token) {
    if (token.startsWith('//') ||
        token.startsWith('#') ||
        token.startsWith('/*')) {
      return const TextStyle(color: Color(0xFF7D8590));
    }
    if (token.startsWith('"') || token.startsWith("'")) {
      return const TextStyle(color: Color(0xFFA5D6FF));
    }
    if (RegExp(r'^\d').hasMatch(token)) {
      return const TextStyle(color: Color(0xFFFFC777));
    }
    return const TextStyle(
      color: Color(0xFFFF7B72),
      fontWeight: FontWeight.w700,
    );
  }
}

class FileEditorOverlay extends StatelessWidget {
  const FileEditorOverlay({
    super.key,
    required this.path,
    required this.controller,
    required this.loading,
    required this.saving,
    required this.editing,
    required this.editable,
    required this.onClose,
    required this.onEdit,
    required this.onSave,
  });

  final String path;
  final TextEditingController controller;
  final bool loading;
  final bool saving;
  final bool editing;
  final bool editable;
  final VoidCallback onClose;
  final VoidCallback onEdit;
  final VoidCallback onSave;

  @override
  Widget build(BuildContext context) {
    final accent = Theme.of(context).colorScheme.secondary;
    final prefs = AppPreferences.of(context);
    final pathParts = path.split('/').where((item) => item.isNotEmpty).toList();
    final fileName = pathParts.isEmpty ? path : pathParts.last;
    return Positioned.fill(
      child: Material(
        color: AppColors.backdrop,
        child: GestureDetector(
          behavior: HitTestBehavior.opaque,
          onTap: onClose,
          child: Align(
            alignment: Alignment.bottomCenter,
            child: GestureDetector(
              onTap: () {},
              child: FractionallySizedBox(
                heightFactor: 0.88,
                widthFactor: 1,
                child: Container(
                  decoration: const BoxDecoration(
                    color: AppColors.bgBase,
                    borderRadius: BorderRadius.vertical(
                      top: Radius.circular(AppRadius.lg),
                    ),
                    border: Border(
                      top: BorderSide(color: AppColors.border, width: 0.5),
                    ),
                  ),
                  clipBehavior: Clip.antiAlias,
                  child: Column(
                    children: [
                      Container(
                        height: 54,
                        decoration: const BoxDecoration(
                          color: AppColors.bgSurface,
                          border: Border(
                            bottom: BorderSide(
                              color: AppColors.border,
                              width: 0.5,
                            ),
                          ),
                        ),
                        child: Row(
                          children: [
                            IconButton(
                              onPressed: onClose,
                              icon: const Icon(
                                Icons.keyboard_arrow_down,
                                size: 26,
                              ),
                              color: AppColors.textPrimary,
                            ),
                            Expanded(
                              child: Text(
                                fileName.isNotEmpty ? fileName : path,
                                maxLines: 1,
                                overflow: TextOverflow.ellipsis,
                                style: const TextStyle(
                                  color: AppColors.textPrimary,
                                  fontSize: AppTextSize.body,
                                  fontWeight: FontWeight.w700,
                                ),
                              ),
                            ),
                            if (!editing)
                              TextButton(
                                onPressed: loading || !editable ? null : onEdit,
                                child: Text(
                                  editable
                                      ? prefs.t('file.edit')
                                      : prefs.t('file.readOnlyLarge'),
                                ),
                              )
                            else
                              TextButton(
                                onPressed: loading || saving ? null : onSave,
                                child: saving
                                    ? SizedBox(
                                        width: 16,
                                        height: 16,
                                        child: CircularProgressIndicator(
                                          strokeWidth: 2,
                                          color: accent,
                                        ),
                                      )
                                    : Text(prefs.t('file.save')),
                              ),
                            const SizedBox(width: AppSpacing.s),
                          ],
                        ),
                      ),
                      Expanded(
                        child: loading
                            ? Center(
                                child: CircularProgressIndicator(color: accent),
                              )
                            : Container(
                                color: const Color(0xFF070A0F),
                                child: TextField(
                                  controller: controller,
                                  expands: true,
                                  maxLines: null,
                                  minLines: null,
                                  readOnly: !editing,
                                  showCursor: editing,
                                  keyboardType: TextInputType.multiline,
                                  textAlignVertical: TextAlignVertical.top,
                                  style: const TextStyle(
                                    color: AppColors.textPrimary,
                                    fontSize: AppTextSize.small,
                                    height: 1.42,
                                    fontFamily: 'monospace',
                                  ),
                                  cursorColor: accent,
                                  decoration: const InputDecoration(
                                    border: InputBorder.none,
                                    contentPadding: EdgeInsets.all(
                                      AppSpacing.m,
                                    ),
                                  ),
                                ),
                              ),
                      ),
                    ],
                  ),
                ),
              ),
            ),
          ),
        ),
      ),
    );
  }
}
