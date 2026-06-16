import 'dart:async';
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../services/log_service.dart';
import '../services/native_terminal_bridge.dart';
import '../services/native_terminal_replay_controller.dart';

class NativeTerminalView extends StatefulWidget {
  const NativeTerminalView({
    super.key,
    required this.replay,
    required this.fontSize,
    required this.keyboardRequested,
    required this.keyboardRequestSerial,
    required this.onInput,
    required this.onResize,
    required this.onSelectionChanged,
  });

  final NativeTerminalReplay replay;
  final double fontSize;
  final bool keyboardRequested;
  final int keyboardRequestSerial;
  final ValueChanged<String> onInput;
  final void Function(int cols, int rows) onResize;
  final ValueChanged<String?> onSelectionChanged;

  static bool get supported => Platform.isAndroid || Platform.isIOS;

  @override
  State<NativeTerminalView> createState() => _NativeTerminalViewState();
}

class _NativeTerminalViewState extends State<NativeTerminalView> {
  StreamSubscription<NativeTerminalEvent>? _events;
  int? _viewId;
  int _viewGeneration = 0;
  int _appliedRevision = -1;
  bool _applyingReplay = false;
  bool _replayApplyRequested = false;
  bool _forceFullReplay = false;
  String? _appliedSessionId;
  String _appliedContent = '';

  @override
  void initState() {
    super.initState();
    _events = NativeTerminalBridge.events.listen(_handleEvent);
  }

  @override
  void didUpdateWidget(covariant NativeTerminalView oldWidget) {
    super.didUpdateWidget(oldWidget);
    final viewId = _viewId;
    if (viewId == null) return;
    final sessionChanged =
        widget.replay.sessionId != oldWidget.replay.sessionId;
    if (sessionChanged) {
      _appliedRevision = -1;
      _appliedSessionId = null;
      _appliedContent = '';
    }
    if (widget.fontSize != oldWidget.fontSize) {
      unawaited(NativeTerminalBridge.setFontSize(viewId, widget.fontSize));
    }
    if (widget.keyboardRequested != oldWidget.keyboardRequested ||
        widget.keyboardRequestSerial != oldWidget.keyboardRequestSerial) {
      _applyKeyboardRequest(viewId);
    }
    _scheduleReplayApply(forceFull: sessionChanged);
  }

  @override
  void dispose() {
    _viewGeneration += 1;
    _viewId = null;
    _events?.cancel();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    if (Platform.isAndroid) {
      return AndroidView(
        viewType: NativeTerminalBridge.viewType,
        creationParams: {'fontSize': widget.fontSize},
        creationParamsCodec: const StandardMessageCodec(),
        onPlatformViewCreated: _onPlatformViewCreated,
      );
    }
    if (Platform.isIOS) {
      return UiKitView(
        viewType: NativeTerminalBridge.viewType,
        creationParams: {'fontSize': widget.fontSize},
        creationParamsCodec: const StandardMessageCodec(),
        onPlatformViewCreated: _onPlatformViewCreated,
      );
    }
    return const SizedBox.shrink();
  }

  void _onPlatformViewCreated(int id) {
    _viewId = id;
    _viewGeneration += 1;
    _appliedRevision = -1;
    _applyingReplay = false;
    _replayApplyRequested = false;
    _forceFullReplay = false;
    _appliedSessionId = null;
    _appliedContent = '';
    unawaited(NativeTerminalBridge.setFontSize(id, widget.fontSize));
    unawaited(NativeTerminalBridge.focus(id));
    _applyKeyboardRequest(id);
    _scheduleReplayApply(forceFull: true);
  }

  void _applyKeyboardRequest(int viewId) {
    if (widget.keyboardRequested) {
      unawaited(NativeTerminalBridge.showKeyboard(viewId));
    } else {
      unawaited(NativeTerminalBridge.hideKeyboard(viewId));
    }
  }

  void _scheduleReplayApply({bool forceFull = false}) {
    final viewId = _viewId;
    if (viewId == null) return;
    final replay = widget.replay;
    if (!forceFull && replay.revision == _appliedRevision) return;
    _forceFullReplay = _forceFullReplay || forceFull;
    _replayApplyRequested = true;
    if (_applyingReplay) return;
    unawaited(_drainReplayApplies(viewId, _viewGeneration));
  }

  Future<void> _drainReplayApplies(int viewId, int generation) async {
    _applyingReplay = true;
    try {
      while (mounted &&
          _viewId == viewId &&
          _viewGeneration == generation &&
          _replayApplyRequested) {
        _replayApplyRequested = false;
        final forceFull = _forceFullReplay;
        _forceFullReplay = false;
        final replay = widget.replay;
        if (!forceFull && replay.revision == _appliedRevision) continue;
        final applied = await _applyReplayToView(
          viewId,
          replay,
          forceFull: forceFull,
        );
        if (!mounted ||
            _viewId != viewId ||
            _viewGeneration != generation ||
            !applied) {
          continue;
        }
        _appliedRevision = replay.revision;
        _appliedSessionId = replay.sessionId;
        _appliedContent = replay.content;
      }
    } finally {
      if (_viewId == viewId && _viewGeneration == generation) {
        _applyingReplay = false;
      }
    }
  }

  Future<bool> _applyReplayToView(
    int viewId,
    NativeTerminalReplay replay, {
    required bool forceFull,
  }) async {
    final sameSession = _appliedSessionId == replay.sessionId;
    if (!forceFull &&
        sameSession &&
        replay.content.startsWith(_appliedContent)) {
      final data = replay.content.substring(_appliedContent.length);
      if (data.isEmpty) return true;
      CoduxLog.debug(
        '[codux-flutter-terminal] native replay view=$viewId session=${replay.sessionId} revision=${replay.revision} reset=${replay.reset} full=false bytes=${data.length}',
      );
      await NativeTerminalBridge.feed(viewId, data);
      return true;
    }
    if (replay.content.isEmpty && !forceFull && sameSession) return true;
    if (forceFull || replay.reset || !sameSession) {
      CoduxLog.debug(
        '[codux-flutter-terminal] native replay view=$viewId session=${replay.sessionId} revision=${replay.revision} reset=${replay.reset} full=$forceFull bytes=${replay.content.length}',
      );
      await NativeTerminalBridge.replace(viewId, replay.content);
      return true;
    }
    final data = replay.append;
    if (data.isEmpty) return true;
    CoduxLog.debug(
      '[codux-flutter-terminal] native replay view=$viewId session=${replay.sessionId} revision=${replay.revision} reset=${replay.reset} full=$forceFull bytes=${data.length}',
    );
    await NativeTerminalBridge.feed(viewId, data);
    return true;
  }

  void _handleEvent(NativeTerminalEvent event) {
    if (event.viewId != _viewId) return;
    switch (event.type) {
      case 'input':
        final data = event.data;
        if (data != null && data.isNotEmpty) widget.onInput(data);
      case 'resize':
        final cols = event.cols;
        final rows = event.rows;
        if (cols != null && rows != null) widget.onResize(cols, rows);
      case 'selection':
        final data = event.data;
        widget.onSelectionChanged(data == null || data.isEmpty ? null : data);
    }
  }
}
