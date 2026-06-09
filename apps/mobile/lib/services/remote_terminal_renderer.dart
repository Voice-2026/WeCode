import 'dart:async';

import 'package:codux_native_terminal/codux_native_terminal.dart';

import 'log_service.dart';

abstract class NativeTerminalPort {
  Future<void> clear();
  Future<void> write(String data);
  Future<void> replace(String data);
  Future<void> dispose();
}

class CoduxNativeTerminalPort implements NativeTerminalPort {
  CoduxNativeTerminalPort(this.controller);

  final CoduxNativeTerminalController controller;

  @override
  Future<void> clear() => controller.clear();

  @override
  Future<void> write(String data) => controller.write(data);

  @override
  Future<void> replace(String data) => controller.replace(data);

  @override
  Future<void> dispose() => controller.dispose();
}

class RemoteTerminalRenderer {
  NativeTerminalPort? _controller;
  String _pendingOutput = '';
  Future<void> _operationQueue = Future<void>.value();
  int _generation = 0;

  bool get hasController => _controller != null;

  void attach(NativeTerminalPort controller) {
    _controller = controller;
  }

  void detach(NativeTerminalPort controller) {
    if (_controller == controller) {
      _controller = null;
    }
  }

  Future<void> dispose() async {
    _generation += 1;
    final controller = _controller;
    _controller = null;
    await controller?.dispose();
  }

  Future<void> clear({required String? sessionId}) {
    CoduxLog.debug('[codux-flutter-terminal] clear session=${sessionId ?? ''}');
    _generation += 1;
    _pendingOutput = '';
    final generation = _generation;
    return _enqueueNativeOperation(
      generation,
      (controller) => controller.clear(),
    );
  }

  void write(String data, {required bool replayingBuffer}) {
    final displayData = _filterStandalonePromptLines(data);
    if (displayData.isEmpty) return;
    final controller = _controller;
    if (controller == null) {
      CoduxLog.debug(
        '[codux-flutter-output] pending bytes=${displayData.codeUnits.length} replay=$replayingBuffer',
      );
      _pendingOutput += displayData;
      return;
    }
    CoduxLog.debug(
      '[codux-flutter-output] write-native bytes=${displayData.codeUnits.length} replay=$replayingBuffer',
    );
    unawaited(
      _enqueueNativeOperation(
        _generation,
        (controller) => controller.write(displayData),
      ),
    );
  }

  Future<void> replace(String data, {required bool replayingBuffer}) {
    final displayData = replayingBuffer
        ? data
        : _filterStandalonePromptLines(data);
    _pendingOutput = '';
    final controller = _controller;
    if (controller == null) {
      _pendingOutput = displayData;
      return Future<void>.value();
    }
    CoduxLog.debug(
      '[codux-flutter-output] replace-native bytes=${displayData.codeUnits.length} replay=$replayingBuffer',
    );
    return _enqueueNativeOperation(
      _generation,
      (controller) => controller.replace(displayData),
    );
  }

  bool restoreCached(String cached, {required bool clearFirst}) {
    if (cached.isEmpty) return false;
    _pendingOutput = '';
    if (_controller == null) {
      _pendingOutput = cached;
    } else if (clearFirst) {
      unawaited(replace(cached, replayingBuffer: true));
    } else {
      write(cached, replayingBuffer: true);
    }
    return true;
  }

  bool restoreControllerWithCached(String? cached) {
    if (cached != null && cached.isNotEmpty) {
      _pendingOutput = '';
      unawaited(
        _enqueueNativeOperation(
          _generation,
          (controller) => controller.replace(cached),
        ),
      );
      return true;
    }
    final pending = _pendingOutput;
    _pendingOutput = '';
    if (pending.isNotEmpty) {
      unawaited(
        _enqueueNativeOperation(
          _generation,
          (controller) => controller.write(pending),
        ),
      );
      return true;
    }
    return false;
  }

  void replayCached(
    String cached, {
    required String sessionId,
    required String reason,
  }) {
    if (cached.isEmpty || _controller == null) return;
    CoduxLog.debug(
      '[codux-flutter-output] replay-native reason=$reason bytes=${cached.codeUnits.length} session=$sessionId',
    );
    unawaited(
      _enqueueNativeOperation(
        _generation,
        (controller) => controller.replace(cached),
      ),
    );
  }

  Future<void> _enqueueNativeOperation(
    int generation,
    Future<void> Function(NativeTerminalPort controller) operation,
  ) {
    final task = _operationQueue.catchError((_) {}).then((_) async {
      if (generation != _generation) return;
      final controller = _controller;
      if (controller == null) return;
      await operation(controller);
    });
    _operationQueue = task.catchError((Object error) {
      CoduxLog.warn('[codux-flutter-output] native operation failed: $error');
    });
    return task;
  }
}

String filterStandalonePromptLines(String data) {
  return _filterStandalonePromptLines(data);
}

String stripTerminalControls(String data) {
  return _stripTerminalControls(data);
}

String _filterStandalonePromptLines(String data) {
  if (!data.contains('%')) return data;
  final output = StringBuffer();
  var index = 0;
  while (index < data.length) {
    var end = index;
    while (end < data.length) {
      final codeUnit = data.codeUnitAt(end);
      if (codeUnit == 10 || codeUnit == 13) break;
      end += 1;
    }

    var lineEnd = end;
    if (end < data.length) {
      final codeUnit = data.codeUnitAt(end);
      if (codeUnit == 13 &&
          end + 1 < data.length &&
          data.codeUnitAt(end + 1) == 10) {
        lineEnd = end + 2;
      } else {
        lineEnd = end + 1;
      }
    }

    final line = data.substring(index, end);
    final isTerminatedLine = end < data.length;
    final isStandalonePromptArtifact =
        isTerminatedLine && _stripTerminalControls(line).trim() == '%';
    if (!isStandalonePromptArtifact) {
      output.write(data.substring(index, lineEnd));
    }
    index = lineEnd;
  }
  return output.toString();
}

String _stripTerminalControls(String data) {
  return data
      .replaceAll(RegExp('\u001B\\[[0-?]*[ -/]*[@-~]'), '')
      .replaceAll(RegExp('\u001B\\][^\u0007\u001B]*(?:\u0007|\u001B\\\\)'), '');
}
