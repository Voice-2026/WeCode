import 'package:codux_flutter/screens/scanner_screen.dart';
import 'package:codux_flutter/i18n.dart';
import 'package:codux_flutter/theme/app_theme.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('manual pairing exposes custom server input and submits it', (
    tester,
  ) async {
    String? payload;
    await tester.pumpWidget(
      MaterialApp(
        theme: buildAppTheme(),
        home: Scaffold(
          body: AppPreferences(
            accent: AccentChoices.cyan,
            locale: LocaleChoices.simplifiedChinese,
            child: Stack(
              children: [
                ScannerScreen(
                  bottomInset: 0,
                  onDetected: (value) => payload = value,
                  onClose: () {},
                  scannerBuilder: (_) => const ColoredBox(color: Colors.black),
                ),
              ],
            ),
          ),
        ),
      ),
    );

    await tester.tap(find.text('手动连接'));
    await tester.pumpAndSettle();
    await tester.tap(find.text('自定义'));
    await tester.pumpAndSettle();

    final serverField = find.widgetWithText(
      TextField,
      'https://your-relay.example',
    );
    expect(serverField, findsOneWidget);

    await tester.enterText(serverField, 'https://relay.example');
    await tester.enterText(find.widgetWithText(TextField, '6 位配对码'), '123456');
    await tester.pump();
    await tester.tap(find.text('配对'));
    await tester.pump();

    expect(payload, contains('server=https%3A%2F%2Frelay.example'));
    expect(payload, contains('code=123456'));
  });
}
