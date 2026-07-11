import 'package:wecode_flutter/services/remote_terminal_subscription_controller.dart';
import 'package:wecode_flutter/services/remote_protocol_service.dart';
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

  test('unsubscribes previous project during switching', () {
    final controller = RemoteTerminalSubscriptionController();

    final first = controller.replaceProject('project-a');
    controller.markProjectSubscribed(
      first.subscribeProjectId!,
      baselineRequested: true,
    );
    final plan = controller.replaceProject('project-b');

    expect(plan.unsubscribe?.type, RemoteMessageType.resourceUnsubscribe);
    final unsubscribePayload = plan.unsubscribe?.payload as Map;
    expect(unsubscribePayload['resource'], RemoteResourceType.terminals);
    expect(unsubscribePayload['projectId'], 'project-a');
    expect(plan.unsubscribeProjectId, 'project-a');
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

    expect(backToA.unsubscribe?.type, RemoteMessageType.resourceUnsubscribe);
    expect((backToA.unsubscribe?.payload as Map)['projectId'], 'project-b');
    expect(backToA.subscribe?.type, RemoteMessageType.resourceSubscribe);
    expect((backToA.subscribe?.payload as Map)['projectId'], 'project-a');
  });

  test('switching back to stale project subscribes with baseline', () {
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

    expect(refreshA.unsubscribe?.type, RemoteMessageType.resourceUnsubscribe);
    expect((refreshA.unsubscribe?.payload as Map)['projectId'], 'project-b');
    expect(refreshA.subscribe?.type, RemoteMessageType.resourceSubscribe);
    final refreshPayload = refreshA.subscribe?.payload as Map;
    expect(refreshPayload['projectId'], 'project-a');
    expect(refreshPayload['baseline'], isTrue);
  });

  test(
    'project switch requests baseline even when caller skips same-project refresh',
    () {
      final controller = RemoteTerminalSubscriptionController();

      final first = controller.replaceProject('project-a');
      controller.markProjectSubscribed(
        first.subscribeProjectId!,
        baselineRequested: true,
      );

      final switchPlan = controller.replaceProject(
        'project-b',
        baseline: false,
      );

      expect(
        switchPlan.unsubscribe?.type,
        RemoteMessageType.resourceUnsubscribe,
      );
      expect(
        (switchPlan.unsubscribe?.payload as Map)['projectId'],
        'project-a',
      );
      final payload = switchPlan.subscribe?.payload as Map;
      expect(payload['projectId'], 'project-b');
      expect(payload['baseline'], isTrue);
    },
  );

  test('project baseline carries target session viewport metadata', () {
    final controller = RemoteTerminalSubscriptionController();

    final plan = controller.replaceProject(
      'project-a',
      requestId: 'request-1',
      baselineSessionId: 'term-a',
      viewportCols: 72,
      viewportRows: 18,
    );

    final payload = plan.subscribe?.payload as Map;
    expect(payload['projectId'], 'project-a');
    expect(payload['baseline'], isTrue);
    expect(payload['requestId'], 'request-1');
    expect(payload['baselineSessionId'], 'term-a');
    expect(payload['viewportCols'], 72);
    expect(payload['viewportRows'], 18);
    expect(plan.subscribe?.sessionId, isNull);
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
