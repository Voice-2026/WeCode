import 'package:flutter/material.dart';

import '../../../theme/app_theme.dart';

/// Pad workspace surface palette (the app dark theme). The accent is NOT here —
/// widgets read it from `Theme.of(context).colorScheme.secondary` so the pad
/// honors the user's theme color. Layout/structure follows the reference design,
/// colors follow the app theme.
class PadColors {
  PadColors._();

  // Surfaces. `bg` matches the system status-bar background; cards sit on top.
  static const bg = Color(0xFF0D1117); // app / status-bar background
  static const panel = Color(0xFF161B22); // sidebar / workspace / right cards
  static const header = Color(0xFF111820);
  static const panelTrack = Color(0xFF1E242C);
  static const card = Color(0xFF21262D); // inner rows, avatars
  static const cardActive = Color(0xFF2A313B);
  static const border = Color(0xFF30363D);
  static const statsPanel = AIStatsPanelColors(
    background: bg,
    card: panel,
    cardHeader: header,
    cardBorder: border,
    track: panelTrack,
  );

  // Text
  static const textPrimary = Color(0xFFE6EDF3);
  static const textSecondary = Color(0xFFB1BAC4);
  static const textMuted = Color(0xFF88949E);
  static const textSubtle = Color(0xFF6E7681);

  // Status / data
  static const success = Color(0xFF22C55E);
  static const warning = Color(0xFFFACC15);
  static const danger = Color(0xFFEF4444);

  // Chart / language palette
  static const chartA = Color(0xFF7C6CF0);
  static const chartB = Color(0xFF3B82F6);
  static const chartC = Color(0xFF34D399);
  static const chartD = Color(0xFFF59E0B);
  static const chartE = Color(0xFF64748B);
}

class PadMetrics {
  PadMetrics._();

  static const panelRadius = 16.0;
  static const panelBorderWidth = 0.5;
  static const leftColumnWidth = 264.0;
  static const rightColumnWidth = 304.0;
}

class PadPanelSurface extends StatelessWidget {
  const PadPanelSurface({super.key, required this.child, this.width});

  final Widget child;
  final double? width;

  @override
  Widget build(BuildContext context) {
    final radius = BorderRadius.circular(PadMetrics.panelRadius);
    return SizedBox(
      width: width,
      child: ClipRRect(
        borderRadius: radius,
        clipBehavior: Clip.antiAlias,
        child: Container(
          decoration: const BoxDecoration(color: PadColors.panel),
          foregroundDecoration: BoxDecoration(
            borderRadius: radius,
            border: Border.all(
              color: PadColors.border,
              width: PadMetrics.panelBorderWidth,
            ),
          ),
          child: child,
        ),
      ),
    );
  }
}
