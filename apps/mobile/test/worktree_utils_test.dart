import 'package:codux_flutter/models/remote_models.dart';
import 'package:codux_flutter/services/worktree_utils.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  const project = ProjectInfo(id: 'project-1', name: 'Project', path: '/repo');

  test('builds unique worktree branch options in priority order', () {
    final options = worktreeBranchOptions(
      defaultBaseBranch: 'main',
      baseBranches: ['develop', 'main'],
      worktrees: [
        _worktree(branch: 'feature/a'),
        _worktree(branch: 'develop'),
      ],
    );

    expect(options, ['main', 'develop', 'feature/a']);
  });

  test('chooses preferred base branch only when available', () {
    expect(
      defaultWorktreeBaseBranch(preferred: 'main', options: ['main', 'dev']),
      'main',
    );
    expect(
      defaultWorktreeBaseBranch(preferred: 'release', options: ['main', 'dev']),
      'main',
    );
  });

  test('worktree title falls back from name to branch to path', () {
    expect(worktreeTitle(_worktree(name: 'Task', branch: 'feature/a')), 'Task');
    expect(
      worktreeTitle(_worktree(name: '', branch: 'feature/a')),
      'feature/a',
    );
    expect(
      worktreeTitle(_worktree(name: '', branch: '', path: '/tmp/wt')),
      '/tmp/wt',
    );
  });

  test('builds worktree operation envelopes', () {
    const controller = RemoteWorktreeController();
    final worktree = _worktree();

    expect(controller.listEnvelope(project).type, 'worktree.list');
    expect(
      (controller.selectEnvelope(project, worktree).payload
          as Map)['worktreeId'],
      worktree.id,
    );

    final create = controller.createEnvelope(
      project: project,
      baseBranch: 'main',
      name: 'feature-a',
    );
    expect(create.type, 'worktree.create');
    expect((create.payload as Map)['branchName'], 'feature-a');

    final merge = controller.mergeEnvelope(project, worktree);
    expect(merge.type, 'worktree.merge');
    expect((merge.payload as Map)['removeBranch'], isFalse);

    final delete = controller.deleteEnvelope(project, worktree);
    expect(delete.type, 'worktree.delete');
    expect((delete.payload as Map)['removeBranch'], isTrue);
  });

  test('fast worktree switching keeps project and worktree ids explicit', () {
    const controller = RemoteWorktreeController();
    final worktrees = [
      _worktree(id: 'wt-a', branch: 'feature/a', path: '/repo-wt-a'),
      _worktree(id: 'wt-b', branch: 'feature/b', path: '/repo-wt-b'),
      _worktree(id: 'wt-c', branch: 'feature/c', path: '/repo-wt-c'),
    ];

    final payloads = worktrees
        .map((worktree) => controller.selectEnvelope(project, worktree).payload)
        .cast<Map>()
        .toList();

    expect(payloads.map((payload) => payload['projectId']), [
      'project-1',
      'project-1',
      'project-1',
    ]);
    expect(payloads.map((payload) => payload['worktreeId']), [
      'wt-a',
      'wt-b',
      'wt-c',
    ]);
    expect(payloads.map((payload) => payload['projectPath']), [
      '/repo',
      '/repo',
      '/repo',
    ]);
  });

  test('parses worktree state payload', () {
    const controller = RemoteWorktreeController();

    final state = controller.stateFromPayload({
      'selectedWorktreeId': 'wt-1',
      'defaultBaseBranch': 'main',
      'baseBranches': ['main', 'develop', 'main'],
      'worktrees': [
        {
          'id': 'wt-1',
          'projectId': 'project-1',
          'name': 'Task',
          'branch': 'feature/a',
          'path': '/repo-wt',
          'status': 'ready',
        },
      ],
    });

    expect(state, isNotNull);
    expect(state!.selectedWorktreeId, 'wt-1');
    expect(state.baseBranches, ['main', 'develop']);
    expect(state.defaultBaseBranch, 'main');
    expect(state.worktrees.single.name, 'Task');
  });
}

RemoteWorktreeInfo _worktree({
  String id = 'wt-1',
  String name = '',
  String branch = 'main',
  String path = '/tmp/project',
}) {
  return RemoteWorktreeInfo(
    id: id,
    projectId: 'project-1',
    name: name,
    branch: branch,
    path: path,
    status: '',
    isDefault: false,
    exists: true,
  );
}
