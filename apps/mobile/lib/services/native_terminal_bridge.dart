import 'dart:async';

import 'package:flutter/services.dart';

class NativeTerminalEvent {
  const NativeTerminalEvent({
    required this.viewId,
    required this.type,
    this.data,
    this.cols,
    this.rows,
  });

  final int viewId;
  final String type;
  final String? data;
  final int? cols;
  final int? rows;

  static NativeTerminalEvent? fromPlatform(Object? value) {
    if (value is! Map) return null;
    final id = value['id'];
    final type = value['type']?.toString();
    if (id is! int || type == null || type.isEmpty) return null;
    return NativeTerminalEvent(
      viewId: id,
      type: type,
      data: value['data']?.toString(),
      cols: _intValue(value['cols']),
      rows: _intValue(value['rows']),
    );
  }
}

class NativeTerminalBridge {
  NativeTerminalBridge._();

  static const viewType = 'codux/native_terminal';

  static const MethodChannel _methods = MethodChannel(
    'codux/native_terminal/methods',
  );
  static const EventChannel _events = EventChannel(
    'codux/native_terminal/events',
  );

  static Stream<NativeTerminalEvent> get events => _events
      .receiveBroadcastStream()
      .map(NativeTerminalEvent.fromPlatform)
      .where((event) => event != null)
      .cast<NativeTerminalEvent>();

  static Future<void> feed(int viewId, String data) async {
    if (data.isEmpty) return;
    await _methods.invokeMethod<void>('feed', {'id': viewId, 'data': data});
  }

  static Future<void> replace(int viewId, String data) async {
    await _methods.invokeMethod<void>('replace', {'id': viewId, 'data': data});
  }

  static Future<void> reset(int viewId) async {
    await _methods.invokeMethod<void>('reset', {'id': viewId});
  }

  static Future<void> setFontSize(int viewId, double fontSize) async {
    await _methods.invokeMethod<void>('setFontSize', {
      'id': viewId,
      'fontSize': fontSize,
    });
  }

  static Future<void> sendKey(int viewId, String key) async {
    await _methods.invokeMethod<void>('sendKey', {'id': viewId, 'key': key});
  }

  static Future<void> focus(int viewId) async {
    await _methods.invokeMethod<void>('focus', {'id': viewId});
  }

  static Future<void> showKeyboard(int viewId) async {
    await _methods.invokeMethod<void>('showKeyboard', {'id': viewId});
  }

  static Future<void> hideKeyboard(int viewId) async {
    await _methods.invokeMethod<void>('hideKeyboard', {'id': viewId});
  }
}

int? _intValue(Object? value) {
  if (value is int) return value;
  if (value is num) return value.toInt();
  return int.tryParse(value?.toString() ?? '');
}
