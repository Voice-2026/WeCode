import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter/services.dart';

typedef CoduxTerminalInputCallback = void Function(String data);
typedef CoduxTerminalResponseCallback = void Function(String data);
typedef CoduxTerminalResizeCallback = void Function(int cols, int rows);
typedef CoduxTerminalMetricsCallback =
    void Function(CoduxTerminalMetrics metrics);

class CoduxTerminalMetrics {
  const CoduxTerminalMetrics({
    required this.rows,
    required this.cursorRow,
    required this.cursorBottomPx,
    required this.historyRows,
    required this.topRow,
  });

  final int rows;
  final int cursorRow;
  final int cursorBottomPx;
  final int historyRows;
  final int topRow;

  factory CoduxTerminalMetrics.fromMap(Map<dynamic, dynamic> map) {
    int value(String key) => (map[key] as num?)?.toInt() ?? 0;
    return CoduxTerminalMetrics(
      rows: value('rows'),
      cursorRow: value('cursorRow'),
      cursorBottomPx: value('cursorBottomPx'),
      historyRows: value('historyRows'),
      topRow: value('topRow'),
    );
  }
}

class CoduxNativeTerminalController {
  CoduxNativeTerminalController._(int viewId)
    : _methods = MethodChannel(
        'codux_native_terminal/terminal_view_$viewId/methods',
      ),
      _events = EventChannel(
        'codux_native_terminal/terminal_view_$viewId/events',
      );

  final MethodChannel _methods;
  final EventChannel _events;
  StreamSubscription<dynamic>? _subscription;
  CoduxTerminalInputCallback? _onInput;
  CoduxTerminalResponseCallback? _onTerminalResponse;
  CoduxTerminalResizeCallback? _onResize;
  CoduxTerminalMetricsCallback? _onMetrics;
  bool _disposed = false;
  Future<void> _operationChain = Future<void>.value();

  Future<void> write(String data) async {
    if (data.isEmpty) return Future.value();
    return _enqueueVoid('write', {'data': data});
  }

  Future<void> replace(String data) {
    return _enqueueVoid('replace', {'data': data});
  }

  Future<void> clear() => _enqueueVoid('clear');

  Future<void> focusKeyboard() => _enqueueVoid('focusKeyboard');

  Future<void> hideKeyboard() => _enqueueVoid('hideKeyboard');

  Future<void> setScrollEnabled(bool enabled) {
    return _enqueueVoid('setScrollEnabled', {'enabled': enabled});
  }

  Future<bool> copySelection() async {
    if (_disposed) return false;
    try {
      return await _methods.invokeMethod<bool>('copySelection') ?? false;
    } on MissingPluginException {
      return false;
    }
  }

  Future<void> requestResize() => _enqueueVoid('resize');

  Future<void> setLogLevel(String level) {
    return _enqueueVoid('setLogLevel', {'level': level});
  }

  void listen({
    CoduxTerminalInputCallback? onInput,
    CoduxTerminalResponseCallback? onTerminalResponse,
    CoduxTerminalResizeCallback? onResize,
    CoduxTerminalMetricsCallback? onMetrics,
  }) {
    _onInput = onInput;
    _onTerminalResponse = onTerminalResponse;
    _onResize = onResize;
    _onMetrics = onMetrics;
    if (_subscription != null) return;
    _subscription = _events.receiveBroadcastStream().listen((event) {
      if (event is! Map) return;
      switch (event['type']) {
        case 'input':
          final data = event['data']?.toString() ?? '';
          if (data.isNotEmpty) _onInput?.call(data);
          break;
        case 'response':
          final data = event['data']?.toString() ?? '';
          if (data.isNotEmpty) _onTerminalResponse?.call(data);
          break;
        case 'resize':
          final cols = (event['cols'] as num?)?.toInt() ?? 0;
          final rows = (event['rows'] as num?)?.toInt() ?? 0;
          if (cols > 0 && rows > 0) _onResize?.call(cols, rows);
          break;
        case 'metrics':
          _onMetrics?.call(CoduxTerminalMetrics.fromMap(event));
          break;
      }
    });
  }

  Future<void> dispose() async {
    _disposed = true;
    await _subscription?.cancel();
    _subscription = null;
  }

  Future<void> _invokeVoid(String method, [Object? arguments]) async {
    if (_disposed) return;
    try {
      await _methods.invokeMethod<void>(method, arguments);
    } on MissingPluginException {
      // Android can destroy and recreate the platform view while delayed UI
      // callbacks are still pending. The next live view will report its size.
    }
  }

  Future<void> _enqueueVoid(String method, [Object? arguments]) {
    if (_disposed) return Future<void>.value();
    final next = _operationChain.then(
      (_) => _invokeVoid(method, arguments),
      onError: (_) => _invokeVoid(method, arguments),
    );
    _operationChain = next.catchError((_) {});
    return next;
  }
}

class CoduxNativeTerminalView extends StatefulWidget {
  const CoduxNativeTerminalView({
    super.key,
    this.onControllerCreated,
    this.onControllerDisposed,
    this.onInput,
    this.onTerminalResponse,
    this.onResize,
    this.onMetrics,
    this.scrollEnabled = true,
  });

  final ValueChanged<CoduxNativeTerminalController>? onControllerCreated;
  final ValueChanged<CoduxNativeTerminalController>? onControllerDisposed;
  final CoduxTerminalInputCallback? onInput;
  final CoduxTerminalResponseCallback? onTerminalResponse;
  final CoduxTerminalResizeCallback? onResize;
  final CoduxTerminalMetricsCallback? onMetrics;
  final bool scrollEnabled;

  @override
  State<CoduxNativeTerminalView> createState() =>
      _CoduxNativeTerminalViewState();
}

class _CoduxNativeTerminalViewState extends State<CoduxNativeTerminalView> {
  CoduxNativeTerminalController? _controller;

  @override
  void didUpdateWidget(covariant CoduxNativeTerminalView oldWidget) {
    super.didUpdateWidget(oldWidget);
    _controller?.listen(
      onInput: widget.onInput,
      onTerminalResponse: widget.onTerminalResponse,
      onResize: widget.onResize,
      onMetrics: widget.onMetrics,
    );
    if (oldWidget.scrollEnabled != widget.scrollEnabled) {
      _controller?.setScrollEnabled(widget.scrollEnabled);
    }
  }

  @override
  void dispose() {
    final controller = _controller;
    if (controller != null) {
      widget.onControllerDisposed?.call(controller);
      controller.dispose();
    }
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    if (defaultTargetPlatform == TargetPlatform.android) {
      return AndroidView(
        viewType: 'codux_native_terminal/terminal_view',
        creationParamsCodec: const StandardMessageCodec(),
        onPlatformViewCreated: _handlePlatformViewCreated,
        hitTestBehavior: PlatformViewHitTestBehavior.opaque,
      );
    }
    if (defaultTargetPlatform == TargetPlatform.iOS) {
      return UiKitView(
        viewType: 'codux_native_terminal/terminal_view',
        creationParamsCodec: const StandardMessageCodec(),
        onPlatformViewCreated: _handlePlatformViewCreated,
        hitTestBehavior: PlatformViewHitTestBehavior.opaque,
      );
    }
    return const ColoredBox(
      color: Color(0xFF05070A),
      child: Center(
        child: Text(
          'Codux native terminal is Android and iOS only for now.',
          style: TextStyle(color: Color(0xFF94A3B8)),
        ),
      ),
    );
  }

  void _handlePlatformViewCreated(int viewId) {
    final controller = CoduxNativeTerminalController._(viewId)
      ..listen(
        onInput: widget.onInput,
        onTerminalResponse: widget.onTerminalResponse,
        onResize: widget.onResize,
        onMetrics: widget.onMetrics,
      );
    controller.setScrollEnabled(widget.scrollEnabled);
    _controller = controller;
    widget.onControllerCreated?.call(controller);
  }
}
