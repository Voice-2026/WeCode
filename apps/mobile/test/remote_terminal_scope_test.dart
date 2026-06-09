import 'package:codux_flutter/models/remote_models.dart';
import 'package:codux_flutter/services/remote_terminal_scope.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test(
    'resolves scope from terminal session project before selected project',
    () {
      final scope = remoteTerminalScopeForSession(
        sessionId: 'session-2',
        projects: const [
          ProjectInfo(id: 'project-1', name: 'One', path: '/tmp/one'),
          ProjectInfo(id: 'project-2', name: 'Two', path: '/tmp/two'),
        ],
        terminals: const [
          TerminalInfo(id: 'session-2', title: 'Two', projectId: 'project-2'),
        ],
        selectedProjectId: 'project-1',
      );

      expect(scope?.projectId, 'project-2');
      expect(scope?.projectPath, '/tmp/two');
    },
  );

  test('uses explicit terminal when runtime state already removed it', () {
    final scope = remoteTerminalScopeForSession(
      sessionId: 'session-2',
      projects: const [
        ProjectInfo(id: 'project-2', name: 'Two', path: '/tmp/two'),
      ],
      terminals: const [],
      selectedProjectId: null,
      terminal: const TerminalInfo(
        id: 'session-2',
        title: 'Two',
        projectId: 'project-2',
      ),
    );

    expect(scope?.toPayload(), {
      'projectId': 'project-2',
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
