import 'package:codux_flutter/i18n.dart';
import 'package:codux_flutter/theme/app_theme.dart';
import 'package:codux_flutter/widgets/remote_terminal_pane.dart';
import 'package:codux_flutter/widgets/terminal_screen_view.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('terminal content starts at top of terminal body', (tester) async {
    await tester.pumpWidget(
      MaterialApp(
        theme: buildAppTheme(),
        home: AppPreferences(
          accent: AccentChoices.cyan,
          locale: LocaleChoices.english,
          child: SizedBox(
            width: 360,
            height: 720,
            child: _pane(),
          ),
        ),
      ),
    );
    await tester.pump();

    final paneTop = tester.getTopLeft(find.byType(RemoteTerminalPane)).dy;
    final terminalTop = tester.getTopLeft(find.byType(TerminalScreenView)).dy;

    expect(terminalTop, paneTop);
  });
}

RemoteTerminalPane _pane() {
  return RemoteTerminalPane(
    connected: true,
    showTerminal: true,
    hasDevice: true,
    status: '',
    workspaceMode: 'terminal',
    projectListLoaded: true,
    projectCount: 1,
    terminalUploadLoading: false,
    terminalUploadStatus: '',
    terminalBufferLoading: false,
    sessionId: 'session-1',
    pendingBufferSessionId: null,
    connectionStatusText: 'connecting',
    terminalHistoryLoadingText: 'loading',
    maskOpacity: const AlwaysStoppedAnimation(0),
    keyboardRequested: false,
    keyboardVisible: false,
    terminalCursorBottom: 0,
    terminalScreen: null,
    terminalFontSize: 16,
    onConnect: () {},
    onInput: (_) {},
    onResize: (_, _) {},
    onScrollPixels: (_, _) {},
    onSettleScroll: () {},
    onScrollToBottom: () {},
    onMetricsCursorBottom: (_) {},
    onSendKey: (_) {},
    onToggleKeyboard: () {},
    onPaste: () {},
    onCopy: () {},
    onUpload: () {},
    onVoiceInput: () {},
  );
}
