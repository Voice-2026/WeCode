import 'package:flutter/material.dart';

import '../theme/app_theme.dart';

class LatencyBadge extends StatelessWidget {
  const LatencyBadge({
    super.key,
    required this.latencyMs,
    this.connected = true,
    this.compact = false,
  });

  final int? latencyMs;
  final bool connected;
  final bool compact;

  @override
  Widget build(BuildContext context) {
    final color = _latencyColor(latencyMs, connected);
    final label = latencyMs == null ? '-- ms' : '${latencyMs}ms';
    return Container(
      height: compact ? 24 : 26,
      padding: EdgeInsets.symmetric(horizontal: compact ? 7 : 8),
      decoration: BoxDecoration(
        color: color.withValues(alpha: connected ? 0.12 : 0.08),
        borderRadius: BorderRadius.circular(999),
        border: Border.all(color: color.withValues(alpha: 0.18), width: 0.5),
      ),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          Icon(
            Icons.network_ping_rounded,
            size: compact ? 13 : 14,
            color: color,
          ),
          const SizedBox(width: 4),
          Text(
            label,
            style: TextStyle(
              color: color,
              fontSize: compact ? 11 : AppTextSize.small,
              height: 1,
              fontWeight: FontWeight.w800,
            ),
          ),
        ],
      ),
    );
  }

  Color _latencyColor(int? value, bool connected) {
    if (!connected || value == null) return AppColors.textSubtle;
    if (value <= 120) return AppColors.success;
    if (value <= 300) return AppColors.warning;
    return AppColors.danger;
  }
}
