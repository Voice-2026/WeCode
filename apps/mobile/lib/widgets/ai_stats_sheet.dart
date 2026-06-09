import 'package:flutter/material.dart';
import '../i18n.dart';
import '../models/remote_models.dart';
import '../theme/app_theme.dart';

class AIStatsSheet extends StatelessWidget {
  const AIStatsSheet({
    super.key,
    required this.bottomInset,
    required this.stats,
    required this.onClose,
  });

  final double bottomInset;
  final AIStatsInfo stats;
  final VoidCallback onClose;

  @override
  Widget build(BuildContext context) {
    final accent = Theme.of(context).colorScheme.secondary;
    final prefs = AppPreferences.of(context);
    return Positioned.fill(
      child: GestureDetector(
        behavior: HitTestBehavior.opaque,
        onTap: onClose,
        child: Container(
          color: AppColors.backdrop,
          alignment: Alignment.bottomCenter,
          child: GestureDetector(
            onTap: () {},
            child: Container(
              width: double.infinity,
              margin: const EdgeInsets.all(AppSpacing.s),
              padding: EdgeInsets.fromLTRB(
                AppSpacing.l,
                AppSpacing.l,
                AppSpacing.l,
                bottomInset + AppSpacing.l,
              ),
              decoration: BoxDecoration(
                color: AppColors.bgSurface,
                borderRadius: BorderRadius.circular(AppRadius.lg),
                border: Border.all(color: AppColors.border, width: 0.5),
              ),
              child: Column(
                mainAxisSize: MainAxisSize.min,
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Row(
                    children: [
                      Container(
                        width: 36,
                        height: 36,
                        decoration: BoxDecoration(
                          color: accent.withValues(alpha: 0.16),
                          borderRadius: BorderRadius.circular(999),
                        ),
                        child: Icon(
                          Icons.bar_chart_rounded,
                          color: accent,
                          size: 20,
                        ),
                      ),
                      const SizedBox(width: AppSpacing.m),
                      Expanded(
                        child: Column(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: [
                            Text(
                              stats.projectName,
                              maxLines: 1,
                              overflow: TextOverflow.ellipsis,
                              style: const TextStyle(
                                color: AppColors.textPrimary,
                                fontSize: AppTextSize.title,
                                fontWeight: FontWeight.w700,
                              ),
                            ),
                            Text(
                              prefs.t('stats.aiTitle'),
                              style: const TextStyle(
                                color: AppColors.textMuted,
                                fontSize: AppTextSize.small,
                              ),
                            ),
                          ],
                        ),
                      ),
                      IconButton(
                        onPressed: onClose,
                        icon: const Icon(Icons.close, size: 20),
                        color: AppColors.textMuted,
                      ),
                    ],
                  ),
                  const SizedBox(height: AppSpacing.l),
                  Wrap(
                    spacing: AppSpacing.s,
                    runSpacing: AppSpacing.s,
                    children: [
                      _Metric(
                        label: prefs.t('stats.todayToken'),
                        value: _formatInt(stats.todayTokens),
                        accent: accent,
                      ),
                      _Metric(
                        label: prefs.t('stats.totalToken'),
                        value: _formatInt(stats.totalTokens),
                        accent: accent,
                      ),
                      _Metric(
                        label: prefs.t('stats.currentSession'),
                        value: _formatInt(stats.currentSessionTokens),
                        accent: accent,
                      ),
                      _Metric(
                        label: prefs.t('stats.requests'),
                        value: _formatInt(stats.requestCount),
                        accent: accent,
                      ),
                    ],
                  ),
                  const SizedBox(height: AppSpacing.m),
                  _MetaLine(
                    label: prefs.t('stats.tool'),
                    value: stats.currentTool ?? '-',
                  ),
                  _MetaLine(
                    label: prefs.t('stats.model'),
                    value: stats.currentModel ?? '-',
                  ),
                  _MetaLine(
                    label: prefs.t('stats.context'),
                    value: stats.contextUsagePercent == null
                        ? '-'
                        : '${stats.contextUsagePercent!.toStringAsFixed(1)}%',
                  ),
                ],
              ),
            ),
          ),
        ),
      ),
    );
  }

  static String _formatInt(int value) {
    if (value >= 1000000) return '${(value / 1000000).toStringAsFixed(1)}M';
    if (value >= 1000) return '${(value / 1000).toStringAsFixed(1)}K';
    return '$value';
  }
}

class _Metric extends StatelessWidget {
  const _Metric({
    required this.label,
    required this.value,
    required this.accent,
  });
  final String label;
  final String value;
  final Color accent;

  @override
  Widget build(BuildContext context) => Container(
    width:
        (MediaQuery.sizeOf(context).width -
            AppSpacing.l * 2 -
            AppSpacing.s * 3) /
        2,
    padding: const EdgeInsets.all(AppSpacing.m),
    decoration: BoxDecoration(
      color: AppColors.bgBase,
      borderRadius: BorderRadius.circular(AppRadius.md),
    ),
    child: Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(
          label,
          style: const TextStyle(
            color: AppColors.textMuted,
            fontSize: AppTextSize.small,
          ),
        ),
        const SizedBox(height: AppSpacing.xs),
        Text(
          value,
          style: TextStyle(
            color: accent,
            fontSize: AppTextSize.title,
            fontWeight: FontWeight.w800,
          ),
        ),
      ],
    ),
  );
}

class _MetaLine extends StatelessWidget {
  const _MetaLine({required this.label, required this.value});
  final String label;
  final String value;

  @override
  Widget build(BuildContext context) => Padding(
    padding: const EdgeInsets.only(top: AppSpacing.xs),
    child: Row(
      children: [
        SizedBox(
          width: 56,
          child: Text(
            label,
            style: const TextStyle(
              color: AppColors.textMuted,
              fontSize: AppTextSize.small,
            ),
          ),
        ),
        Expanded(
          child: Text(
            value,
            maxLines: 1,
            overflow: TextOverflow.ellipsis,
            style: const TextStyle(
              color: AppColors.textPrimary,
              fontSize: AppTextSize.body,
            ),
          ),
        ),
      ],
    ),
  );
}
