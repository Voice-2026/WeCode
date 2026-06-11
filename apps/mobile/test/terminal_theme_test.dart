import 'package:codux_flutter/theme/app_theme.dart';
import 'package:codux_flutter/theme/terminal_theme.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('dark terminal theme remaps unreadable neutral foregrounds', () {
    final colors = TerminalTheme.resolveCellColors(
      fg: {'kind': 'named', 'name': 'Black'},
      bg: {'kind': 'default'},
      inverse: false,
    );

    expect(colors.bg, AppColors.bgBase);
    expect(colors.fg, AppColors.textPrimary);
    expect(colors.drawBackground, isFalse);
  });

  test('dark terminal theme keeps readable ansi colors intact', () {
    final colors = TerminalTheme.resolveCellColors(
      fg: {'kind': 'named', 'name': 'Green'},
      bg: {'kind': 'default'},
      inverse: false,
    );

    expect(colors.fg, isNot(AppColors.textPrimary));
  });

  test('dark terminal theme normalizes host light cell backgrounds', () {
    final colors = TerminalTheme.resolveCellColors(
      fg: {'kind': 'named', 'name': 'Black'},
      bg: {'kind': 'named', 'name': 'White'},
      inverse: false,
    );

    expect(colors.bg, AppColors.bgElevated);
    expect(colors.fg, AppColors.textPrimary);
    expect(colors.drawBackground, isTrue);
  });
}
