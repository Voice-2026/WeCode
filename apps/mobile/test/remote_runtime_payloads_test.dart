import 'package:codux_flutter/services/remote_runtime_payloads.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('parses project and selected project payload', () {
    final payload = {
      'selectedProjectId': 'project-2',
      'projects': [
        {'id': 'project-1', 'name': 'One'},
        {'id': 'project-2', 'name': 'Two', 'path': '/tmp/two'},
      ],
    };

    final projects = remoteProjectsFromPayload(payload);

    expect(projects.length, 2);
    expect(projects[1].path, '/tmp/two');
    expect(remoteSelectedProjectIdFromPayload(payload), 'project-2');
  });

  test('parses terminal, file, worktree, and git payloads', () {
    expect(
      remoteTerminalsFromPayload({
        'terminals': [
          {'id': 'term-1', 'title': 'Shell', 'projectId': 'project-1'},
        ],
      }).single.id,
      'term-1',
    );
    expect(
      remoteFileEntriesFromPayload({
        'entries': [
          {'name': 'lib', 'path': '/tmp/project/lib', 'isDirectory': true},
        ],
      }).single.isDirectory,
      isTrue,
    );
    expect(
      remoteWorktreesFromPayload({
        'tasks': [
          {'worktreeId': 'worktree-1', 'baseBranch': 'main'},
        ],
        'worktrees': [
          {
            'id': 'worktree-1',
            'projectId': 'project-1',
            'name': 'Task',
            'branch': 'task',
            'path': '/tmp/task',
            'status': 'active',
          },
        ],
      }).single.branch,
      'task',
    );
    expect(
      remoteWorktreesFromPayload({
        'tasks': [
          {'worktreeId': 'worktree-1', 'baseBranch': 'main'},
        ],
        'worktrees': [
          {
            'id': 'worktree-1',
            'projectId': 'project-1',
            'name': 'Task',
            'branch': 'task',
            'path': '/tmp/task',
            'status': 'active',
          },
        ],
      }).single.baseBranch,
      'main',
    );
    expect(
      remoteGitStatusFromPayload({
        'projectId': 'project-1',
        'projectPath': '/tmp/project',
        'branch': 'main',
        'changes': 3,
      })?.changes,
      3,
    );
  });
}
