import 'package:wecode_flutter/models/remote_models.dart';
import 'package:wecode_flutter/services/remote_project_file_controller.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('remembers project file path per project', () {
    final controller = RemoteProjectFileController();
    const project = ProjectInfo(
      id: 'project-1',
      name: 'Project',
      path: '/repo',
    );

    expect(controller.pathForProject(project), '/repo');
    controller.remember(projectId: 'project-1', path: '/repo/lib');
    expect(controller.pathForProject(project), '/repo/lib');
    controller.forget('project-1');
    expect(controller.pathForProject(project), '/repo');
  });

  test('builds file operation envelopes', () {
    final controller = RemoteProjectFileController();
    const entry = RemoteFileEntry(
      name: 'main.dart',
      path: '/repo/lib/main.dart',
      isDirectory: false,
    );

    expect(controller.listEnvelope('/repo').type, 'file.list');
    expect((controller.readEnvelope(entry).payload as Map)['path'], entry.path);
    expect(
      (controller.writeEnvelope(path: entry.path, content: 'x').payload
          as Map)['content'],
      'x',
    );
    expect(
      (controller.deleteEnvelope(entry).payload as Map)['path'],
      entry.path,
    );
  });

  test('parses project file and picker list payloads', () {
    final controller = RemoteProjectFileController();

    final projectFiles = controller.listStateFromPayload({
      'purpose': 'projectFiles',
      'path': '/repo',
      'parent': '/Volumes/Web',
      'entries': [
        {'name': 'lib', 'path': '/repo/lib', 'isDirectory': true},
      ],
    });
    expect(projectFiles, isNotNull);
    expect(projectFiles!.isProjectFiles, isTrue);
    expect(projectFiles.path, '/repo');
    expect(projectFiles.parent, '/Volumes/Web');
    expect(projectFiles.entries.single.name, 'lib');

    final picker = controller.listStateFromPayload({
      'path': '/Users',
      'entries': <Map<String, Object?>>[],
    });
    expect(picker, isNotNull);
    expect(picker!.isProjectFiles, isFalse);
    expect(picker.path, '/Users');
  });

  test('builds rename envelope from sibling filename', () {
    final controller = RemoteProjectFileController();
    const entry = RemoteFileEntry(
      name: 'old.txt',
      path: '/repo/old.txt',
      isDirectory: false,
    );

    final plan = controller.renamePlan(entry, 'new.txt');

    expect(plan, isNotNull);
    expect(plan!.valid, isTrue);
    expect((plan.envelope!.payload as Map)['newPath'], '/repo/new.txt');
  });

  test('rejects empty, unchanged, and nested rename targets', () {
    final controller = RemoteProjectFileController();
    const entry = RemoteFileEntry(
      name: 'old.txt',
      path: '/repo/old.txt',
      isDirectory: false,
    );

    expect(controller.renamePlan(entry, ''), isNull);
    expect(controller.renamePlan(entry, 'old.txt'), isNull);
    expect(controller.renamePlan(entry, 'nested/new.txt')!.valid, isFalse);
  });

  test('builds file editor loading and read states', () {
    final controller = RemoteProjectFileController();
    const entry = RemoteFileEntry(
      name: 'main.dart',
      path: '/repo/main.dart',
      isDirectory: false,
    );

    final loading = controller.beginReadState(entry);
    expect(loading.path, '/repo/main.dart');
    expect(loading.loading, isTrue);
    expect(loading.editable, isTrue);
    expect(loading.highlightEnabled, isTrue);

    final read = controller.readStateFromPayload({
      'path': '/repo/main.dart',
      'content': 'hello',
    });
    expect(read, isNotNull);
    expect(read!.content, 'hello');
    expect(read.loading, isFalse);
    expect(read.editable, isTrue);
  });

  test('disables editing and highlighting for large file payloads', () {
    final controller = RemoteProjectFileController();

    final large = controller.readStateFromPayload({
      'path': '/repo/large.txt',
      'content': 'x' * (remoteFileEditableMaxChars + 1),
    });

    expect(large, isNotNull);
    expect(large!.editable, isFalse);
    expect(large.highlightEnabled, isFalse);
  });

  test('detects deleted active editor file', () {
    final controller = RemoteProjectFileController();
    final deletedPath = controller.deletedPathFromPayload({
      'path': '/repo/main.dart',
    });

    expect(deletedPath, '/repo/main.dart');
    expect(
      controller.shouldCloseEditorAfterDelete(
        deletedPath: deletedPath,
        editingPath: '/repo/main.dart',
      ),
      isTrue,
    );
    expect(
      controller.shouldCloseEditorAfterDelete(
        deletedPath: deletedPath,
        editingPath: '/repo/other.dart',
      ),
      isFalse,
    );
  });
}
