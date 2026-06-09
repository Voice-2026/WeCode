import 'package:codux_flutter/services/remote_terminal_subscription_controller.dart';
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
    expect(plan.subscribe?.type, 'terminal.subscribe');
    final payload = plan.subscribe?.payload as Map;
    expect(payload['baseline'], isTrue);
    expect(payload['maxChars'], 65536);
    expect(payload['chunkChars'], 16384);
    expect(plan.subscribeProjectId, 'project-a');
    expect(controller.projectId, 'project-a');
  });

  test('ignores duplicate project subscription', () {
    final controller = RemoteTerminalSubscriptionController();

    controller.replaceProject('project-a');
    final plan = controller.replaceProject('project-a');

    expect(plan.hasWork, isFalse);
    expect(controller.projectId, 'project-a');
  });

  test('replaces previous project subscription', () {
    final controller = RemoteTerminalSubscriptionController();

    controller.replaceProject('project-a');
    final plan = controller.replaceProject('project-b');

    expect(plan.unsubscribe?.type, 'terminal.unsubscribe');
    expect(plan.unsubscribeProjectId, 'project-a');
    expect(plan.subscribe?.type, 'terminal.subscribe');
    expect(plan.subscribeProjectId, 'project-b');
    expect(controller.projectId, 'project-b');
  });

  test('reset allows fresh subscription', () {
    final controller = RemoteTerminalSubscriptionController();

    controller.replaceProject('project-a');
    controller.reset();
    final plan = controller.replaceProject('project-a');

    expect(plan.unsubscribe, isNull);
    expect(plan.subscribeProjectId, 'project-a');
  });
}
