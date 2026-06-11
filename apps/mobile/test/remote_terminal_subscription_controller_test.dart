import 'package:codux_flutter/services/remote_terminal_subscription_controller.dart';
import 'package:codux_flutter/services/remote_protocol_service.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('subscribes first project without unsubscribe', () {
    final controller = RemoteTerminalSubscriptionController();

    final plan = controller.replaceProject(
      'project-a',
      maxChars: 65536,
      chunkChars: 16384,
    );

    expect(plan.unsubscribe, isNull);
    expect(plan.subscribe?.type, RemoteMessageType.resourceSubscribe);
    final payload = plan.subscribe?.payload as Map;
    expect(payload['resource'], RemoteResourceType.terminals);
    expect(payload['projectId'], 'project-a');
    expect(payload['baseline'], isTrue);
    expect(payload['maxChars'], 65536);
    expect(payload['chunkChars'], 16384);
    expect(plan.subscribeProjectId, 'project-a');
    controller.markProjectSubscribed('project-a', baselineRequested: true);
    expect(controller.projectId, 'project-a');
  });

  test('does not mark duplicate project subscription until committed', () {
    final controller = RemoteTerminalSubscriptionController();

    final first = controller.replaceProject('project-a');
    final beforeCommit = controller.replaceProject('project-a');
    expect(beforeCommit.hasWork, isTrue);

    controller.markProjectSubscribed(
      first.subscribeProjectId!,
      baselineRequested: true,
    );
    final afterCommit = controller.replaceProject('project-a');

    expect(afterCommit.hasWork, isFalse);
    expect(controller.projectId, 'project-a');
  });

  test('refreshes same project only after baseline is marked stale', () {
    final controller = RemoteTerminalSubscriptionController();

    controller.markProjectSubscribed('project-a', baselineRequested: true);
    controller.markProjectBaselineStale('project-a');
    final plan = controller.replaceProject('project-a');

    expect(plan.unsubscribe, isNull);
    expect(plan.subscribe?.type, RemoteMessageType.resourceSubscribe);
    final payload = plan.subscribe?.payload as Map;
    expect(payload['resource'], RemoteResourceType.terminals);
    expect(payload['projectId'], 'project-a');
    expect(payload['baseline'], isTrue);
  });

  test('keeps previous project subscriptions during fast switching', () {
    final controller = RemoteTerminalSubscriptionController();

    final first = controller.replaceProject('project-a');
    controller.markProjectSubscribed(
      first.subscribeProjectId!,
      baselineRequested: true,
    );
    final plan = controller.replaceProject('project-b');

    expect(plan.unsubscribe, isNull);
    expect(plan.subscribe?.type, RemoteMessageType.resourceSubscribe);
    final subscribePayload = plan.subscribe?.payload as Map;
    expect(subscribePayload['resource'], RemoteResourceType.terminals);
    expect(subscribePayload['projectId'], 'project-b');
    expect(plan.subscribeProjectId, 'project-b');
    controller.markProjectSubscribed(
      plan.subscribeProjectId!,
      baselineRequested: true,
    );
    expect(controller.projectId, 'project-b');

    final backToA = controller.replaceProject('project-a');

    expect(backToA.hasWork, isFalse);
    expect(controller.projectId, 'project-a');
  });

  test('fast switching refreshes only stale subscribed project baseline', () {
    final controller = RemoteTerminalSubscriptionController();

    final first = controller.replaceProject('project-a');
    controller.markProjectSubscribed(
      first.subscribeProjectId!,
      baselineRequested: true,
    );
    final second = controller.replaceProject('project-b');
    controller.markProjectSubscribed(
      second.subscribeProjectId!,
      baselineRequested: true,
    );
    controller.markProjectBaselineStale('project-a');

    final refreshA = controller.replaceProject('project-a');
    final duplicateB = controller.replaceProject('project-b');

    expect(refreshA.unsubscribe, isNull);
    expect(refreshA.subscribe?.type, RemoteMessageType.resourceSubscribe);
    final refreshPayload = refreshA.subscribe?.payload as Map;
    expect(refreshPayload['projectId'], 'project-a');
    expect(refreshPayload['baseline'], isTrue);
    expect(duplicateB.hasWork, isFalse);
  });

  test('reset allows fresh subscription', () {
    final controller = RemoteTerminalSubscriptionController();

    final first = controller.replaceProject('project-a');
    controller.markProjectSubscribed(
      first.subscribeProjectId!,
      baselineRequested: true,
    );
    controller.reset();
    final plan = controller.replaceProject('project-a');

    expect(plan.unsubscribe, isNull);
    expect(plan.subscribeProjectId, 'project-a');
  });
}
