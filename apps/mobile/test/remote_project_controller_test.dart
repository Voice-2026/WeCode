import 'package:codux_flutter/models/remote_models.dart';
import 'package:codux_flutter/services/remote_project_controller.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  const controller = RemoteProjectController();
  const project = ProjectInfo(id: 'project-1', name: 'Project', path: '/repo');

  test('builds add project envelope with path-derived name', () {
    final plan = controller.savePlan(
      mode: ProjectFormMode.add,
      path: '/Volumes/Web/codux',
      name: '',
    );

    expect(plan.valid, isTrue);
    expect(plan.name, 'codux');
    expect(plan.envelope!.type, 'project.add');
    expect((plan.envelope!.payload as Map)['name'], 'codux');
  });

  test('builds edit project envelope with selected project', () {
    final plan = controller.savePlan(
      mode: ProjectFormMode.edit,
      path: '/repo-next',
      name: 'Repo Next',
      selectedProject: project,
    );

    expect(plan.valid, isTrue);
    expect(plan.envelope!.type, 'project.edit');
    expect((plan.envelope!.payload as Map)['projectId'], 'project-1');
    expect((plan.envelope!.payload as Map)['path'], '/repo-next');
  });

  test('builds project form drafts', () {
    final edit = controller.editDraft(project);
    expect(edit.mode, ProjectFormMode.edit);
    expect(edit.name, 'Project');
    expect(edit.path, '/repo');

    final add = controller.addDraft();
    expect(add.mode, ProjectFormMode.add);
    expect(add.name, isEmpty);
    expect(add.path, isEmpty);
  });

  test('rejects invalid save plans', () {
    expect(
      controller.savePlan(mode: ProjectFormMode.add, path: '', name: '').valid,
      isFalse,
    );
    expect(
      controller
          .savePlan(mode: ProjectFormMode.edit, path: '/repo', name: 'Repo')
          .valid,
      isFalse,
    );
  });

  test('builds project utility envelopes', () {
    expect(controller.removeEnvelope(project).type, 'project.remove');
    expect(controller.aiStatsEnvelope(project).type, 'ai.stats');
    expect(
      (controller.aiStatsEnvelope(project, worktreeId: 'worktree-1').payload
          as Map)['worktreeId'],
      'worktree-1',
    );
    expect(controller.gitStatusEnvelope(project).type, 'git.status');
    expect(
      (controller.gitStatusEnvelope(project).payload as Map)['projectPath'],
      '/repo',
    );
  });

  test('builds file picker list envelope', () {
    expect(controller.filePickerListEnvelope(null).payload, isEmpty);
    expect(
      (controller.filePickerListEnvelope('/repo').payload as Map)['path'],
      '/repo',
    );
  });

  test('uses entry name or path tail for folder display name', () {
    expect(
      controller.folderDisplayName(
        const RemoteFileEntry(
          name: 'repo',
          path: '/Volumes/Web/repo',
          isDirectory: true,
        ),
      ),
      'repo',
    );
    expect(
      controller.folderDisplayName(
        const RemoteFileEntry(
          name: '',
          path: r'C:\work\repo',
          isDirectory: true,
        ),
      ),
      'repo',
    );
  });

  test('selects folder path and only fills missing project name', () {
    const entry = RemoteFileEntry(
      name: '',
      path: '/Volumes/Web/repo',
      isDirectory: true,
    );

    final inferred = controller.selectFolder(entry: entry, currentName: '');
    expect(inferred.path, '/Volumes/Web/repo');
    expect(inferred.name, 'repo');

    final existing = controller.selectFolder(
      entry: entry,
      currentName: 'Existing',
    );
    expect(existing.name, 'Existing');
  });
}
