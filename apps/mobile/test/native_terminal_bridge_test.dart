import 'package:codux_flutter/services/native_terminal_bridge.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  const channel = MethodChannel('codux/native_terminal/methods');

  tearDown(() {
    TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
        .setMockMethodCallHandler(channel, null);
  });

  test('replace sends a single native terminal method call', () async {
    final calls = <MethodCall>[];
    TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
        .setMockMethodCallHandler(channel, (call) async {
          calls.add(call);
          return true;
        });

    await NativeTerminalBridge.replace(7, 'hello');

    expect(calls, hasLength(1));
    expect(calls.single.method, 'replace');
    expect(calls.single.arguments, {'id': 7, 'data': 'hello'});
  });
}
