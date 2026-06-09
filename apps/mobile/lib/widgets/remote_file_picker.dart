import 'package:flutter/material.dart';
import '../i18n.dart';
import '../models/remote_models.dart';
import '../theme/app_theme.dart';

class RemoteFilePicker extends StatelessWidget {
  const RemoteFilePicker({
    super.key,
    required this.topInset,
    required this.bottomInset,
    required this.title,
    required this.path,
    required this.parent,
    required this.entries,
    required this.loading,
    required this.onClose,
    required this.onOpenPath,
    required this.onSelect,
    required this.onOpenHome,
    required this.onOpenRoot,
    required this.onOpenVolumes,
  });

  final double topInset;
  final double bottomInset;
  final String title;
  final String path;
  final String? parent;
  final List<RemoteFileEntry> entries;
  final bool loading;
  final VoidCallback onClose;
  final ValueChanged<String> onOpenPath;
  final ValueChanged<RemoteFileEntry> onSelect;
  final VoidCallback onOpenHome;
  final VoidCallback onOpenRoot;
  final VoidCallback onOpenVolumes;

  @override
  Widget build(BuildContext context) {
    final accent = Theme.of(context).colorScheme.secondary;
    final prefs = AppPreferences.of(context);
    return Positioned.fill(
      child: Material(
        color: AppColors.bgBase,
        child: Column(
          children: [
            Container(
              height: AppLayout.topBarHeight + topInset,
              padding: EdgeInsets.only(top: topInset),
              decoration: const BoxDecoration(
                color: AppColors.bgBase,
                border: Border(
                  bottom: BorderSide(color: AppColors.border, width: 0.5),
                ),
              ),
              child: Row(
                children: [
                  const SizedBox(width: AppSpacing.s),
                  SizedBox(
                    width: 44,
                    height: 44,
                    child: IconButton(
                      onPressed: onClose,
                      icon: const Icon(Icons.close, size: 22),
                      color: AppColors.textPrimary,
                    ),
                  ),
                  Expanded(
                    child: Text(
                      title,
                      textAlign: TextAlign.center,
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      style: const TextStyle(
                        color: AppColors.textPrimary,
                        fontSize: AppTextSize.title,
                        fontWeight: FontWeight.w700,
                      ),
                    ),
                  ),
                  PopupMenuButton<String>(
                    icon: const Icon(
                      Icons.storage_rounded,
                      color: AppColors.textPrimary,
                    ),
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
                ],
              ),
            ),
            Container(
              width: double.infinity,
              padding: const EdgeInsets.fromLTRB(
                AppSpacing.l,
                AppSpacing.m,
                AppSpacing.l,
                AppSpacing.s,
              ),
              color: AppColors.bgSurface,
              child: Text(
                path.isEmpty ? '~' : path,
                maxLines: 2,
                overflow: TextOverflow.ellipsis,
                style: const TextStyle(
                  color: AppColors.textMuted,
                  fontSize: AppTextSize.small,
                  height: 1.25,
                ),
              ),
            ),
            Expanded(
              child: loading
                  ? Center(child: CircularProgressIndicator(color: accent))
                  : ListView.separated(
                      physics: const BouncingScrollPhysics(),
                      padding: EdgeInsets.only(
                        bottom: bottomInset + AppSpacing.l,
                      ),
                      itemCount: entries.length + (parent == null ? 0 : 1),
                      separatorBuilder: (_, _) => const Divider(
                        height: 0.5,
                        thickness: 0.5,
                        color: AppColors.border,
                      ),
                      itemBuilder: (context, index) {
                        if (parent != null && index == 0) {
                          return _FileRow(
                            icon: Icons.drive_folder_upload_outlined,
                            name: '..',
                            path: parent!,
                            accent: accent,
                            onTap: () => onOpenPath(parent!),
                            onTrailingTap: null,
                          );
                        }
                        final offset = parent == null ? index : index - 1;
                        final item = entries[offset];
                        return _FileRow(
                          icon: item.isDirectory
                              ? Icons.folder_rounded
                              : Icons.insert_drive_file_outlined,
                          name: item.name,
                          path: item.path,
                          accent: accent,
                          onTap: item.isDirectory
                              ? () => onOpenPath(item.path)
                              : null,
                          onTrailingTap: item.isDirectory
                              ? () => onSelect(item)
                              : null,
                        );
                      },
                    ),
            ),
          ],
        ),
      ),
    );
  }
}

class _FileRow extends StatelessWidget {
  const _FileRow({
    required this.icon,
    required this.name,
    required this.path,
    required this.accent,
    required this.onTap,
    required this.onTrailingTap,
  });

  final IconData icon;
  final String name;
  final String path;
  final Color accent;
  final VoidCallback? onTap;
  final VoidCallback? onTrailingTap;

  @override
  Widget build(BuildContext context) {
    final prefs = AppPreferences.of(context);
    return InkWell(
      onTap: onTap,
      child: Padding(
        padding: const EdgeInsets.symmetric(
          horizontal: AppSpacing.l,
          vertical: AppSpacing.m,
        ),
        child: Row(
          children: [
            Icon(
              icon,
              color: onTap == null ? AppColors.textSubtle : accent,
              size: 22,
            ),
            const SizedBox(width: AppSpacing.m),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    name,
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
                    path,
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
            if (onTrailingTap != null)
              TextButton(
                onPressed: onTrailingTap,
                style: TextButton.styleFrom(foregroundColor: accent),
                child: Text(
                  prefs.t('common.select'),
                  style: const TextStyle(fontSize: AppTextSize.body),
                ),
              ),
          ],
        ),
      ),
    );
  }
}
