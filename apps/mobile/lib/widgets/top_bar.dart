import 'package:flutter/material.dart';
import '../i18n.dart';
import '../theme/app_theme.dart';

class TopBar extends StatelessWidget {
  const TopBar({
    super.key,
    required this.title,
    required this.status,
    required this.connected,
    required this.topInset,
    required this.onTapDevice,
    required this.onTapMore,
    this.deviceMenuOpen = false,
  });

  final String title;
  final String status;
  final bool connected;
  final double topInset;
  final bool deviceMenuOpen;
  final VoidCallback onTapDevice;
  final VoidCallback onTapMore;

  @override
  Widget build(BuildContext context) {
    final prefs = AppPreferences.of(context);
    final dotColor = connected ? AppColors.success : AppColors.danger;
    final subtitle = connected && status.isEmpty
        ? prefs.t('app.connected')
        : status;
    return Container(
      padding: EdgeInsets.only(top: topInset),
      height: AppLayout.topBarHeight + topInset,
      decoration: const BoxDecoration(
        color: AppColors.bgBase,
        border: Border(bottom: BorderSide(color: AppColors.border, width: 0.5)),
      ),
      child: Row(
        children: [
          const SizedBox(width: AppSpacing.l),
          Expanded(
            child: InkWell(
              borderRadius: BorderRadius.circular(AppRadius.sm),
              onTap: onTapDevice,
              child: Padding(
                padding: const EdgeInsets.symmetric(
                  horizontal: AppSpacing.s,
                  vertical: 6,
                ),
                child: Row(
                  children: [
                    const Icon(
                      Icons.desktop_mac_outlined,
                      size: 19,
                      color: AppColors.textPrimary,
                    ),
                    const SizedBox(width: AppSpacing.s),
                    Expanded(
                      child: Column(
                        mainAxisAlignment: MainAxisAlignment.center,
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Row(
                            mainAxisSize: MainAxisSize.max,
                            children: [
                              Flexible(
                                child: Text(
                                  title,
                                  maxLines: 1,
                                  overflow: TextOverflow.ellipsis,
                                  style: const TextStyle(
                                    color: AppColors.textPrimary,
                                    fontSize: AppTextSize.body,
                                    fontWeight: FontWeight.w700,
                                    height: 1.05,
                                  ),
                                ),
                              ),
                              const SizedBox(width: 2),
                              SizedBox(
                                width: 30,
                                height: 24,
                                child: Icon(
                                  deviceMenuOpen
                                      ? Icons.keyboard_arrow_up
                                      : Icons.keyboard_arrow_down,
                                  size: 26,
                                  color: AppColors.textMuted,
                                ),
                              ),
                            ],
                          ),
                          const SizedBox(height: 4),
                          Row(
                            mainAxisSize: MainAxisSize.min,
                            children: [
                              Container(
                                width: 6,
                                height: 6,
                                decoration: BoxDecoration(
                                  color: dotColor,
                                  shape: BoxShape.circle,
                                ),
                              ),
                              const SizedBox(width: AppSpacing.xs),
                              Flexible(
                                child: Text(
                                  subtitle,
                                  maxLines: 1,
                                  overflow: TextOverflow.ellipsis,
                                  style: const TextStyle(
                                    color: AppColors.textMuted,
                                    fontSize: AppTextSize.small,
                                    height: 1,
                                  ),
                                ),
                              ),
                            ],
                          ),
                        ],
                      ),
                    ),
                  ],
                ),
              ),
            ),
          ),
          IconButton(
            onPressed: onTapMore,
            icon: const Icon(
              Icons.more_vert,
              color: AppColors.textPrimary,
              size: 22,
            ),
            visualDensity: VisualDensity.compact,
          ),
          const SizedBox(width: AppSpacing.s),
        ],
      ),
    );
  }
}
