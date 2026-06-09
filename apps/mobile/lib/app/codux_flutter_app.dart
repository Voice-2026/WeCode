import 'package:flutter/material.dart';
import 'package:flutter_localizations/flutter_localizations.dart';

import '../i18n.dart';
import '../models/remote_models.dart';
import '../screens/home_page.dart';
import '../services/remote_transport.dart';
import '../theme/app_theme.dart';

class CoduxFlutterApp extends StatefulWidget {
  const CoduxFlutterApp({
    super.key,
    this.initialDevices,
    this.transportFactory,
  });

  final List<StoredDevice>? initialDevices;
  final RemoteTransportFactory? transportFactory;

  @override
  State<CoduxFlutterApp> createState() => _CoduxFlutterAppState();
}

class _CoduxFlutterAppState extends State<CoduxFlutterApp> {
  AccentOption _accent = AccentChoices.cyan;
  LocaleOption _locale = LocaleChoices.zhCN;

  void _setAccent(AccentOption next) => setState(() => _accent = next);
  void _setLocale(LocaleOption next) => setState(() => _locale = next);

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      debugShowCheckedModeBanner: false,
      title: 'Codux Mobile',
      theme: buildAppTheme(accent: _accent.color),
      locale: flutterLocaleForOption(_locale),
      supportedLocales: supportedFlutterLocales,
      localizationsDelegates: const [
        GlobalMaterialLocalizations.delegate,
        GlobalCupertinoLocalizations.delegate,
        GlobalWidgetsLocalizations.delegate,
      ],
      home: AppPreferences(
        accent: _accent,
        locale: _locale,
        child: CoduxHomePage(
          onChangeAccent: _setAccent,
          onChangeLocale: _setLocale,
          initialDevices: widget.initialDevices,
          transportFactory: widget.transportFactory,
        ),
      ),
    );
  }
}
