import 'package:codux_flutter/services/remote_connection_sync_controller.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('does not request initial lists before protocol is ready', () {
    final controller = RemoteConnectionSyncController();
    controller.beginConnectionGeneration();

    final plan = controller.initialSyncPlan(
      transportReady: true,
      transportConnected: true,
    );

    expect(plan.hasWork, isFalse);
    expect(controller.shouldSendDeviceInfo(), isFalse);
  });

  test(
    'protocol ready opens one initial sync and deduplicates device info',
    () {
      final controller = RemoteConnectionSyncController();
      controller.beginConnectionGeneration();
      expect(controller.markProtocolReady(), isTrue);

      final plan = controller.initialSyncPlan(
        transportReady: true,
        transportConnected: true,
      );

      expect(plan.sendDeviceInfo, isTrue);
      expect(plan.requestProjectList, isTrue);
      expect(plan.requestTerminalList, isTrue);

      controller.markDeviceInfoSent();
      expect(controller.shouldSendDeviceInfo(), isFalse);
    },
  );

  test('new connection generation resets loaded list state', () {
    final controller = RemoteConnectionSyncController();
    controller.beginConnectionGeneration();
    controller.markProtocolReady();
    controller.syncState.markProjectListReceived();
    controller.syncState.markTerminalListReceived();

    controller.beginConnectionGeneration();
    controller.markProtocolReady();
    final plan = controller.initialSyncPlan(
      transportReady: true,
      transportConnected: true,
    );

    expect(plan.requestProjectList, isTrue);
    expect(plan.requestTerminalList, isTrue);
  });

  test('host info is sent once per generation unless forced', () {
    final controller = RemoteConnectionSyncController();
    controller.beginConnectionGeneration();

    expect(
      controller.shouldSendHostInfo(
        transportReady: true,
        transportConnected: true,
      ),
      isTrue,
    );
    controller.markHostInfoSent();
    expect(
      controller.shouldSendHostInfo(
        transportReady: true,
        transportConnected: true,
      ),
      isFalse,
    );
    expect(
      controller.shouldSendHostInfo(
        transportReady: true,
        transportConnected: true,
        force: true,
      ),
      isTrue,
    );
  });

  test('forced protocol ready is accepted once per connection generation', () {
    final controller = RemoteConnectionSyncController();
    controller.beginConnectionGeneration();

    expect(controller.markProtocolReady(force: true), isTrue);
    expect(controller.markProtocolReady(force: true), isFalse);

    controller.resetSyncForCurrentGeneration();
    expect(controller.markProtocolReady(force: true), isTrue);
    expect(controller.markProtocolReady(force: true), isFalse);

    controller.beginConnectionGeneration();
    expect(controller.markProtocolReady(force: true), isTrue);
  });
}
