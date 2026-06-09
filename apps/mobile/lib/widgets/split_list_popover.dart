import 'package:flutter/material.dart';
import '../i18n.dart';
import '../models/remote_models.dart';
import '../theme/app_theme.dart';
import 'dropdown_overlay.dart';

class SplitListPopover extends StatelessWidget {
  const SplitListPopover({
    super.key,
    required this.top,
    required this.terminals,
    required this.activeTerminalId,
    required this.projectName,
    required this.onClose,
    required this.onSelect,
    required this.onLongPress,
    required this.onCreate,
    required this.onRefresh,
  });

  final double top;
  final List<TerminalInfo> terminals;
  final String? activeTerminalId;
  final String? projectName;
  final VoidCallback onClose;
  final ValueChanged<TerminalInfo> onSelect;
  final ValueChanged<TerminalInfo> onLongPress;
  final VoidCallback onCreate;
  final VoidCallback onRefresh;

  @override
  Widget build(BuildContext context) {
    final accent = Theme.of(context).colorScheme.secondary;
    final prefs = AppPreferences.of(context);
    return DropdownOverlay(
      top: top,
      onClose: onClose,
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          if (terminals.isEmpty)
            Padding(
              padding: const EdgeInsets.symmetric(
                vertical: AppSpacing.l,
                horizontal: AppSpacing.l,
              ),
              child: Text(
                prefs.t('app.splitsEmpty'),
                style: const TextStyle(
                  color: AppColors.textMuted,
                  fontSize: 12,
                ),
              ),
            ),
          for (var i = 0; i < terminals.length; i += 1)
            _TerminalRow(
              index: i + 1,
              terminal: terminals[i],
              active: terminals[i].id == activeTerminalId,
              accent: accent,
              terminalLabel: prefs.t('app.terminal'),
              onTap: () => onSelect(terminals[i]),
              onLongPress: () => onLongPress(terminals[i]),
            ),
          Padding(
            padding: const EdgeInsets.all(AppSpacing.s),
            child: Row(
              children: [
                Expanded(
                  child: _BottomAction(
                    icon: Icons.refresh,
                    label: prefs.t('app.refresh'),
                    color: AppColors.textPrimary,
                    background: AppColors.bgElevated,
                    onTap: onRefresh,
                  ),
                ),
                const SizedBox(width: AppSpacing.s),
                Expanded(
                  child: _BottomAction(
                    icon: Icons.add,
                    label: prefs.t('app.newTerminal'),
                    color: accent,
                    background: accent.withValues(alpha: 0.14),
                    onTap: onCreate,
                  ),
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }
}

class _BottomAction extends StatelessWidget {
  const _BottomAction({
    required this.icon,
    required this.label,
    required this.color,
    required this.background,
    required this.onTap,
  });
  final IconData icon;
  final String label;
  final Color color;
  final Color background;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) => Material(
    color: background,
    borderRadius: BorderRadius.circular(AppRadius.sm),
    child: InkWell(
      borderRadius: BorderRadius.circular(AppRadius.sm),
      onTap: onTap,
      child: SizedBox(
        height: 40,
        child: Row(
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            Icon(icon, size: 17, color: color),
            const SizedBox(width: AppSpacing.xs),
            Text(
              label,
              style: TextStyle(
                color: color,
                fontSize: AppTextSize.small,
                fontWeight: FontWeight.w700,
              ),
            ),
          ],
        ),
      ),
    ),
  );
}

class _TerminalRow extends StatelessWidget {
  const _TerminalRow({
    required this.index,
    required this.terminal,
    required this.active,
    required this.accent,
    required this.terminalLabel,
    required this.onTap,
    required this.onLongPress,
  });
  final int index;
  final TerminalInfo terminal;
  final bool active;
  final Color accent;
  final String terminalLabel;
  final VoidCallback onTap;
  final VoidCallback onLongPress;

  @override
  Widget build(BuildContext context) {
    return InkWell(
      onTap: onTap,
      onLongPress: onLongPress,
      child: Padding(
        padding: const EdgeInsets.symmetric(
          horizontal: AppSpacing.l,
          vertical: AppSpacing.m,
        ),
        child: Row(
          children: [
            SizedBox(
              width: 60,
              child: Text(
                '$terminalLabel $index',
                style: TextStyle(
                  color: active ? accent : AppColors.textMuted,
                  fontSize: 12,
                  fontWeight: FontWeight.w600,
                ),
              ),
            ),
            const SizedBox(width: AppSpacing.m),
            Expanded(
              child: Text(
                terminal.title.isNotEmpty ? terminal.title : 'Terminal',
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
                style: TextStyle(
                  color: active ? accent : AppColors.textPrimary,
                  fontSize: 14,
                  fontWeight: active ? FontWeight.w700 : FontWeight.w500,
                ),
              ),
            ),
            if (active) Icon(Icons.check, color: accent, size: 18),
          ],
        ),
      ),
    );
  }
}
