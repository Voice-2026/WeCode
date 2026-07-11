import 'package:wecode_flutter/models/remote_models.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('parses remote git status payload', () {
    final status = RemoteGitStatusInfo.fromJson({
      'projectId': 'project-1',
      'projectPath': '/tmp/project-1',
      'branch': 'main',
      'upstream': 'origin/main',
      'ahead': 2,
      'behind': 1,
      'staged': 1,
      'unstaged': 2,
      'untracked': 3,
      'changes': 6,
      'isRepository': true,
      'changedFiles': [
        {
          'path': 'lib/main.dart',
          'indexStatus': 'modified',
          'worktreeStatus': 'modified',
        },
      ],
    });

    expect(status.projectId, 'project-1');
    expect(status.branch, 'main');
    expect(status.upstream, 'origin/main');
    expect(status.ahead, 2);
    expect(status.behind, 1);
    expect(status.changes, 6);
    expect(status.isRepository, isTrue);
    expect(status.changedFiles.single.path, 'lib/main.dart');
  });
}
