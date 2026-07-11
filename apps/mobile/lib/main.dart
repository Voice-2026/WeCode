import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import 'app/wecode_flutter_app.dart';
import 'theme/app_theme.dart';

export 'app/wecode_flutter_app.dart';

void main() {
  WidgetsFlutterBinding.ensureInitialized();
  FocusManager.instance.highlightStrategy = FocusHighlightStrategy.alwaysTouch;
  SystemChrome.setSystemUIOverlayStyle(
    SystemUiOverlayStyle(
      statusBarColor: Colors.transparent,
      systemNavigationBarColor: AppColors.bgSurface,
      statusBarIconBrightness: Brightness.light,
      systemNavigationBarIconBrightness: Brightness.light,
    ),
  );
  runApp(const WeCodeFlutterApp());
}
