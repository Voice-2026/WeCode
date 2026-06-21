import 'package:flutter/material.dart';

class AppColors {
  static const bgBase = Color(0xFF0D1117);
  static const bgSurface = Color(0xFF161B22);
  static const bgElevated = Color(0xFF21262D);
  static const border = Color(0xFF30363D);
  static const accent = Color(0xFFD7FF61);
  static const cyan = Color(0xFF00B8D9);
  static const accentSoft = Color(0x2400B8D9);
  static const success = Color(0xFF22C55E);
  static const warning = Color(0xFFFACC15);
  static const danger = Color(0xFFEF4444);
  static const textPrimary = Color(0xFFE6EDF3);
  static const textSecondary = Color(0xFFB1BAC4);
  static const textMuted = Color(0xFF88949E);
  static const textSubtle = Color(0xFF6E7681);
  static const backdrop = Color(0x8B02060C);
}

class AppRadius {
  static const sm = 8.0;
  static const md = 12.0;
  static const lg = 16.0;
}

class AppSpacing {
  static const xs = 4.0;
  static const s = 8.0;
  static const m = 12.0;
  static const l = 16.0;
  static const xl = 20.0;
  static const xxl = 24.0;
}

class AppLayout {
  static const topBarHeight = 56.0;
  static const tabBarHeight = 56.0;
}

class AppTextSize {
  static const small = 12.0;
  static const body = 14.0;
  static const title = 16.0;
}

class AIStatsPanelColors {
  const AIStatsPanelColors({
    this.background = AppColors.bgBase,
    this.card = AppColors.bgSurface,
    this.cardHeader = AppColors.bgSurface,
    this.cardBorder = AppColors.border,
    this.track = AppColors.bgElevated,
  });

  final Color background;
  final Color card;
  final Color cardHeader;
  final Color cardBorder;
  final Color track;
}

ThemeData buildAppTheme({Color accent = AppColors.cyan}) {
  return ThemeData(
    useMaterial3: true,
    brightness: Brightness.dark,
    scaffoldBackgroundColor: AppColors.bgBase,
    focusColor: Colors.transparent,
    hoverColor: Colors.transparent,
    splashColor: Colors.transparent,
    highlightColor: Colors.transparent,
    colorScheme: ColorScheme.dark(
      primary: accent,
      surface: AppColors.bgSurface,
      surfaceContainerHighest: AppColors.bgElevated,
      secondary: accent,
      outline: AppColors.border,
    ),
    textTheme: const TextTheme(
      bodyMedium: TextStyle(
        color: AppColors.textPrimary,
        fontSize: AppTextSize.body,
      ),
      bodySmall: TextStyle(
        color: AppColors.textMuted,
        fontSize: AppTextSize.small,
      ),
      titleMedium: TextStyle(
        color: AppColors.textPrimary,
        fontSize: AppTextSize.title,
        fontWeight: FontWeight.w600,
      ),
    ),
    iconTheme: const IconThemeData(color: AppColors.textPrimary, size: 20),
    textButtonTheme: TextButtonThemeData(
      style: TextButton.styleFrom(foregroundColor: accent),
    ),
    progressIndicatorTheme: ProgressIndicatorThemeData(color: accent),
    inputDecorationTheme: InputDecorationTheme(
      labelStyle: const TextStyle(color: AppColors.textMuted),
      hintStyle: const TextStyle(color: AppColors.textSubtle),
      focusedBorder: UnderlineInputBorder(
        borderSide: BorderSide(color: accent),
      ),
    ),
    textSelectionTheme: TextSelectionThemeData(
      cursorColor: accent,
      selectionColor: accent.withValues(alpha: 0.24),
      selectionHandleColor: accent,
    ),
  );
}
