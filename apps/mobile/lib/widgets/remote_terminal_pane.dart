import 'dart:io';

import 'package:flutter/material.dart';

import '../i18n.dart';
import '../services/native_terminal_replay_controller.dart';
import '../theme/app_theme.dart';
import 'connect_hint.dart';
import 'native_terminal_view.dart';
import 'toolbar.dart';

class RemoteTerminalPane extends StatelessWidget {
  const RemoteTerminalPane({
    super.key,
    required this.connected,
    required this.showTerminal,
    required this.hasDevice,
    required this.status,
    required this.workspaceMode,
    required this.projectListLoaded,
    required this.projectCount,
    required this.terminalUploadLoading,
    required this.terminalUploadStatus,
    required this.terminalBufferLoading,
    required this.sessionId,
    required this.pendingBufferSessionId,
    required this.connectionStatusText,
    required this.terminalHistoryLoadingText,
    required this.keyboardVisible,
    required this.keyboardRequested,
    required this.keyboardRequestSerial,
    required this.terminalReplay,
    required this.terminalFontSize,
    required this.onConnect,
    required this.onInput,
    required this.onResize,
    required this.onSelectionChanged,
    required this.onSendKey,
    required this.onToggleKeyboard,
    required this.onPaste,
    required this.onCopy,
    required this.onUpload,
    required this.onVoiceInput,
  });

  final bool connected;
  final bool showTerminal;
  final bool hasDevice;
  final String status;
  final String workspaceMode;
  final bool projectListLoaded;
  final int projectCount;
  final bool terminalUploadLoading;
  final String terminalUploadStatus;
  final bool terminalBufferLoading;
  final String? sessionId;
  final String? pendingBufferSessionId;
  final String connectionStatusText;
  final String terminalHistoryLoadingText;
  final bool keyboardVisible;
  final bool keyboardRequested;
  final int keyboardRequestSerial;
  final NativeTerminalReplay terminalReplay;
  final double terminalFontSize;
  final VoidCallback onConnect;
  final ValueChanged<String> onInput;
  final void Function(int cols, int rows) onResize;
  final ValueChanged<String?> onSelectionChanged;
  final ValueChanged<String> onSendKey;
  final VoidCallback onToggleKeyboard;
  final VoidCallback onPaste;
  final VoidCallback onCopy;
  final VoidCallback onUpload;
  final VoidCallback onVoiceInput;

  @override
  Widget build(BuildContext context) {
    final showTerminalToolbar = workspaceMode == 'terminal' && connected;
    final keyboardHeight = MediaQuery.viewInsetsOf(context).bottom;
    final bottomInset = MediaQuery.viewPaddingOf(context).bottom;
    final keyboardActiveThreshold = bottomInset + 8.0;
    final effectiveKeyboardHeight = keyboardHeight > keyboardActiveThreshold
        ? keyboardHeight
        : 0.0;
    final toolbarBottom = effectiveKeyboardHeight > 0
        ? effectiveKeyboardHeight
        : bottomInset;
    const toolbarBaseHeight = 76.0;
    final keyboardLift = effectiveKeyboardHeight > 0
        ? (effectiveKeyboardHeight - bottomInset).clamp(0.0, double.infinity)
        : 0.0;
    final terminalPadding = Platform.isIOS
        ? EdgeInsets.zero
        : const EdgeInsets.symmetric(horizontal: 8);

    return MediaQuery.removeViewInsets(
      context: context,
      removeBottom: true,
      child: ClipRect(
        child: LayoutBuilder(
          builder: (context, constraints) {
            final terminalToolbarHeight = toolbarBaseHeight + bottomInset;
            final viewportHeight = constraints.maxHeight.isFinite
                ? constraints.maxHeight
                : MediaQuery.sizeOf(context).height;
            final terminalHeight =
                (viewportHeight -
                        (showTerminalToolbar ? terminalToolbarHeight : 0.0))
                    .clamp(120.0, viewportHeight);
            final showHostSyncOverlay =
                connected && !projectListLoaded && projectCount == 0;
            final showUploadOverlay =
                showTerminal &&
                workspaceMode == 'terminal' &&
                terminalUploadLoading &&
                terminalUploadStatus.isNotEmpty;
            final showHistoryOverlay =
                showTerminal &&
                workspaceMode == 'terminal' &&
                !terminalUploadLoading &&
                terminalBufferLoading &&
                sessionId != null &&
                pendingBufferSessionId == sessionId;

            return Stack(
              clipBehavior: Clip.none,
              children: [
                Positioned(
                  left: 0,
                  right: 0,
                  top: 0,
                  height: terminalHeight,
                  child: Transform.translate(
                    offset: Offset(0, -keyboardLift),
                    child: ColoredBox(
                      key: const ValueKey('remote-terminal-body'),
                      color: AppColors.bgBase,
                      child: Padding(
                        padding: terminalPadding,
                        child: Stack(
                          children: [
                            if (showTerminal && NativeTerminalView.supported)
                              NativeTerminalView(
                                key: Platform.isIOS
                                    ? ValueKey(
                                        'native-terminal-view-ios-${terminalReplay.sessionId}',
                                      )
                                    : const ValueKey('native-terminal-view'),
                                replay: terminalReplay,
                                fontSize: terminalFontSize,
                                keyboardRequested: keyboardRequested,
                                keyboardRequestSerial: keyboardRequestSerial,
                                onInput: onInput,
                                onResize: onResize,
                                onSelectionChanged: onSelectionChanged,
                              )
                            else if (showTerminal)
                              const _TerminalUnavailable()
                            else
                              ConnectHint(
                                status: status.isEmpty
                                    ? AppPreferences.of(
                                        context,
                                      ).t('app.notConnected')
                                    : status,
                                hasDevice: hasDevice,
                                onConnect: onConnect,
                              ),
                            if (showTerminal &&
                                showHostSyncOverlay &&
                                !terminalUploadLoading &&
                                !terminalBufferLoading)
                              _TerminalOverlay(message: connectionStatusText),
                            if (showTerminal &&
                                (showUploadOverlay || showHistoryOverlay))
                              _TerminalOverlay(
                                message: showUploadOverlay
                                    ? terminalUploadStatus
                                    : terminalHistoryLoadingText,
                                opacity: 0.72,
                              ),
                          ],
                        ),
                      ),
                    ),
                  ),
                ),
                if (showTerminalToolbar)
                  Positioned(
                    left: 0,
                    right: 0,
                    bottom: toolbarBottom,
                    child: Toolbar(
                      onSendKey: onSendKey,
                      applicationCursor: false,
                      keyboardVisible: keyboardVisible,
                      bottomInset: 0,
                      onToggleKeyboard: onToggleKeyboard,
                      uploading: terminalUploadLoading,
                      onPaste: onPaste,
                      onCopy: onCopy,
                      onUpload: onUpload,
                      onVoiceInput: onVoiceInput,
                    ),
                  ),
              ],
            );
          },
        ),
      ),
    );
  }
}

class _TerminalUnavailable extends StatelessWidget {
  const _TerminalUnavailable();

  @override
  Widget build(BuildContext context) {
    return const ColoredBox(color: AppColors.bgBase);
  }
}

class _TerminalOverlay extends StatelessWidget {
  const _TerminalOverlay({required this.message, this.opacity = 0.58});

  final String message;
  final double opacity;

  @override
  Widget build(BuildContext context) {
    return Positioned.fill(
      child: IgnorePointer(
        child: DecoratedBox(
          decoration: BoxDecoration(
            color: AppColors.bgBase.withValues(alpha: opacity),
          ),
          child: Center(
            child: Row(
              mainAxisSize: MainAxisSize.min,
              children: [
                SizedBox(
                  width: 16,
                  height: 16,
                  child: CircularProgressIndicator(
                    strokeWidth: 2,
                    color: Theme.of(context).colorScheme.secondary,
                  ),
                ),
                const SizedBox(width: AppSpacing.s),
                Text(
                  message,
                  style: const TextStyle(
                    color: AppColors.textSecondary,
                    fontSize: 13,
                    fontWeight: FontWeight.w600,
                  ),
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}
