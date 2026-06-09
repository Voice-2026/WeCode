import 'package:flutter/material.dart';
import '../theme/app_theme.dart';

class TerminalTransitionMask extends StatelessWidget {
  const TerminalTransitionMask({super.key});

  @override
  Widget build(BuildContext context) =>
      const IgnorePointer(child: ColoredBox(color: AppColors.bgBase));
}
