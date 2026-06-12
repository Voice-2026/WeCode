import 'package:codux_flutter/models/remote_models.dart';
import 'package:codux_flutter/services/remote_terminal_scope.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('parses terminal scope returned by runtime core', () {
    final scope = RemoteTerminalScope.fromJson(const {
      'projectId': 'project-2',
      'worktreeId': 'project-2',
      'projectPath': '/tmp/two',
    });

    expect(scope.toPayload(), {
      'projectId': 'project-2',
      'worktreeId': 'project-2',
      'projectPath': '/tmp/two',
    });
  });

  test('scopes terminal envelope payload without dropping existing fields', () {
    final scoped = scopedTerminalEnvelope(
      const RelayEnvelope(
        type: 'terminal.buffer',
        sessionId: 'session-1',
        payload: {'offset': 0, 'maxChars': 1024},
      ),
      const RemoteTerminalScope(projectId: 'project-1', projectPath: '/tmp/p1'),
    );

    expect(scoped.payload, {
      'offset': 0,
      'maxChars': 1024,
      'projectId': 'project-1',
      'projectPath': '/tmp/p1',
    });
  });
}
