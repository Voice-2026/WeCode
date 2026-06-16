import 'package:codux_flutter/services/native_terminal_replay_controller.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('replaceSession publishes replay revisions only for real changes', () {
    final controller = NativeTerminalReplayController();

    expect(controller.replay('session-1').content, isEmpty);
    expect(controller.replay('session-1').revision, 0);

    expect(controller.replaceSession('session-1', 'hello'), isTrue);
    final first = controller.replay('session-1');
    expect(first.content, 'hello');
    expect(first.append, isEmpty);
    expect(first.reset, isTrue);
    expect(first.revision, greaterThan(0));

    expect(controller.replaceSession('session-1', 'hello'), isFalse);
    expect(controller.replay('session-1').revision, first.revision);

    expect(controller.replaceSession('session-1', 'hello\nworld'), isTrue);
    final second = controller.replay('session-1');
    expect(second.content, 'hello\nworld');
    expect(second.append, isEmpty);
    expect(second.reset, isTrue);
    expect(second.revision, greaterThan(first.revision));
  });

  test('syncSession appends continuous live output without resetting', () {
    final controller = NativeTerminalReplayController();

    expect(controller.syncSession('session-1', 'hello'), isTrue);
    final baseline = controller.replay('session-1');
    expect(baseline.content, 'hello');
    expect(baseline.append, isEmpty);
    expect(baseline.reset, isTrue);

    expect(controller.syncSession('session-1', 'hello world'), isTrue);
    final live = controller.replay('session-1');
    expect(live.content, 'hello world');
    expect(live.append, ' world');
    expect(live.reset, isFalse);

    expect(controller.syncSession('session-1', 'fresh'), isTrue);
    final replaced = controller.replay('session-1');
    expect(replaced.content, 'fresh');
    expect(replaced.append, isEmpty);
    expect(replaced.reset, isTrue);
  });

  test(
    'syncSession keeps native replay mounted when live keyframes append raw',
    () {
      final controller = NativeTerminalReplayController();

      expect(
        controller.syncSession(
          'session-1',
          'raw-history\u001b[2J\u001b[Hkeyframe',
        ),
        isTrue,
      );
      final baseline = controller.replay('session-1');
      expect(baseline.reset, isTrue);

      expect(
        controller.syncSession(
          'session-1',
          'raw-history\u001b[2J\u001b[Hkeyframe\nworking',
        ),
        isTrue,
      );
      final live = controller.replay('session-1');
      expect(live.append, '\nworking');
      expect(live.reset, isFalse);
    },
  );

  test('removeSession and resetAll clear replay state', () {
    final controller = NativeTerminalReplayController();

    controller.replaceSession('session-1', 'one');
    controller.replaceSession('session-2', 'two');
    controller.removeSession('session-1');

    expect(controller.replay('session-1').content, isEmpty);
    expect(controller.replay('session-2').content, 'two');

    controller.resetAll();
    expect(controller.replay('session-2').content, isEmpty);
  });

  test('switching active sessions keeps cached replay for later remount', () {
    final controller = NativeTerminalReplayController();

    controller.replaceSession('session-1', 'one');
    controller.replaceSession('session-2', 'two');

    expect(controller.replay('session-2').content, 'two');
    expect(controller.replay('session-1').content, 'one');
  });
}
