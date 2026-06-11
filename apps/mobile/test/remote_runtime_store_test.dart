import 'package:codux_flutter/models/remote_models.dart';
import 'package:codux_flutter/services/remote_runtime_store.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('project list uses host selected project before local cache', () {
    final store = RemoteRuntimeStore();
    store.restoreCachedProjects([
      const ProjectInfo(id: 'project-1', name: 'Project 1'),
      const ProjectInfo(id: 'project-2', name: 'Project 2'),
    ]);

    store.applyProjectList(
      projects: _projects,
      remoteSelectedProjectId: 'project-2',
      terminalVisible: false,
      terminalListLoaded: false,
    );

    expect(store.selectedProjectId, 'project-2');
  });

  test('visible terminal binds existing session for selected project', () {
    final store = RemoteRuntimeStore();
    store.applyProjectList(
      projects: _projects,
      remoteSelectedProjectId: 'project-2',
      terminalVisible: true,
      terminalListLoaded: false,
    );

    final plan = store.applyTerminalList(
      terminals: const [
        TerminalInfo(id: 'session-1', title: 'One', projectId: 'project-1'),
        TerminalInfo(id: 'session-2', title: 'Two', projectId: 'project-2'),
      ],
      terminalVisible: true,
      terminalListLoaded: true,
    );

    expect(store.activeSessionId, 'session-2');
    expect(plan.bindSessionId, 'session-2');
    expect(plan.requestProjectSelectId, isNull);
  });

  test(
    'missing terminal requests one project select until terminal appears',
    () {
      final store = RemoteRuntimeStore();
      store.applyProjectList(
        projects: _projects,
        remoteSelectedProjectId: 'project-2',
        terminalVisible: true,
        terminalListLoaded: false,
      );

      final first = store.applyTerminalList(
        terminals: const [
          TerminalInfo(id: 'session-1', title: 'One', projectId: 'project-1'),
        ],
        terminalVisible: true,
        terminalListLoaded: true,
      );
      final second = store.ensureTerminalForSelectedProject(
        terminalVisible: true,
        terminalListLoaded: true,
      );

      expect(first.requestProjectSelectId, 'project-2');
      expect(second.requestProjectSelectId, 'project-2');
      store.markProjectSelectSent('project-2');
      final third = store.ensureTerminalForSelectedProject(
        terminalVisible: true,
        terminalListLoaded: true,
      );
      expect(third.requestProjectSelectId, isNull);

      final fourth = store.applyTerminalList(
        terminals: const [
          TerminalInfo(id: 'session-1', title: 'One', projectId: 'project-1'),
          TerminalInfo(id: 'session-2', title: 'Two', projectId: 'project-2'),
        ],
        terminalVisible: true,
        terminalListLoaded: true,
      );

      expect(store.activeSessionId, 'session-2');
      expect(fourth.bindSessionId, 'session-2');
    },
  );

  test('user project selection immediately binds known project terminal', () {
    final store = RemoteRuntimeStore();
    store.applyProjectList(
      projects: _projects,
      remoteSelectedProjectId: 'project-1',
      terminalVisible: true,
      terminalListLoaded: false,
    );
    store.applyTerminalList(
      terminals: const [
        TerminalInfo(id: 'session-1', title: 'One', projectId: 'project-1'),
        TerminalInfo(id: 'session-2', title: 'Two', projectId: 'project-2'),
      ],
      terminalVisible: true,
      terminalListLoaded: true,
    );

    final select = store.userSelectProject(
      project: _projects[1],
      terminalVisible: true,
    );
    final beforeHost = store.ensureTerminalForSelectedProject(
      terminalVisible: true,
      terminalListLoaded: true,
    );

    expect(select.requestProjectSelectId, 'project-2');
    expect(select.clearTerminal, isTrue);
    expect(select.bindSessionId, 'session-2');
    expect(select.bindFullBuffer, isTrue);
    expect(beforeHost.requestProjectSelectId, isNull);
    expect(beforeHost.bindSessionId, isNull);
    expect(store.activeSessionId, 'session-2');
  });

  test(
    'user project selection requests host select when local terminal is unknown',
    () {
      final store = RemoteRuntimeStore();
      store.applyProjectList(
        projects: _projects,
        remoteSelectedProjectId: 'project-1',
        terminalVisible: true,
        terminalListLoaded: false,
      );
      store.applyTerminalList(
        terminals: const [
          TerminalInfo(id: 'session-1', title: 'One', projectId: 'project-1'),
        ],
        terminalVisible: true,
        terminalListLoaded: true,
      );

      final select = store.userSelectProject(
        project: _projects[1],
        terminalVisible: true,
      );

      expect(select.requestProjectSelectId, 'project-2');
      expect(select.clearTerminal, isTrue);
      expect(select.requestTerminalList, isTrue);
      expect(select.bindSessionId, isNull);
      expect(store.activeSessionId, isNull);

      final afterHost = store.applyTerminalList(
        terminals: const [
          TerminalInfo(id: 'session-1', title: 'One', projectId: 'project-1'),
          TerminalInfo(id: 'session-2', title: 'Two', projectId: 'project-2'),
        ],
        terminalVisible: true,
        terminalListLoaded: true,
      );

      expect(afterHost.bindSessionId, 'session-2');
      expect(store.activeSessionId, 'session-2');
    },
  );

  test(
    'pending project with empty terminal list does not repeat project select',
    () {
      final store = RemoteRuntimeStore();
      store.applyProjectList(
        projects: _projects,
        remoteSelectedProjectId: 'project-1',
        terminalVisible: true,
        terminalListLoaded: false,
      );

      final select = store.userSelectProject(
        project: _projects[1],
        terminalVisible: true,
      );
      final emptyList = store.applyTerminalList(
        terminals: const [
          TerminalInfo(id: 'session-1', title: 'One', projectId: 'project-1'),
        ],
        terminalVisible: true,
        terminalListLoaded: true,
      );
      final ensure = store.ensureTerminalForSelectedProject(
        terminalVisible: true,
        terminalListLoaded: true,
      );

      expect(select.requestProjectSelectId, 'project-2');
      expect(select.requestTerminalList, isTrue);
      expect(emptyList.requestProjectSelectId, 'project-2');
      expect(emptyList.requestTerminalList, isFalse);
      expect(ensure.requestProjectSelectId, 'project-2');
      expect(ensure.requestTerminalList, isFalse);
      store.markProjectSelectSent('project-2');
      final afterSent = store.ensureTerminalForSelectedProject(
        terminalVisible: true,
        terminalListLoaded: true,
      );
      expect(afterSent.requestProjectSelectId, isNull);
      expect(afterSent.requestTerminalList, isFalse);
      expect(store.activeSessionId, isNull);
    },
  );

  test('pending user project selection beats stale host selected project', () {
    final store = RemoteRuntimeStore();
    store.applyProjectList(
      projects: _projects,
      remoteSelectedProjectId: 'project-1',
      terminalVisible: true,
      terminalListLoaded: false,
    );

    store.userSelectProject(project: _projects[1], terminalVisible: true);
    store.applyProjectList(
      projects: _projects,
      remoteSelectedProjectId: 'project-1',
      terminalVisible: true,
      terminalListLoaded: false,
    );

    expect(store.selectedProjectId, 'project-2');
  });

  test('failed pending project select is planned again until sent', () {
    final store = RemoteRuntimeStore();
    store.applyProjectList(
      projects: _projects,
      remoteSelectedProjectId: 'project-1',
      terminalVisible: true,
      terminalListLoaded: false,
    );

    final select = store.userSelectProject(
      project: _projects[1],
      terminalVisible: true,
    );
    expect(select.requestProjectSelectId, 'project-2');

    final retry = store.ensureTerminalForSelectedProject(
      terminalVisible: true,
      terminalListLoaded: true,
    );
    expect(retry.requestProjectSelectId, 'project-2');

    store.markProjectSelectSent('project-2');
    final afterSent = store.ensureTerminalForSelectedProject(
      terminalVisible: true,
      terminalListLoaded: true,
    );
    expect(afterSent.requestProjectSelectId, isNull);
  });

  test('project selected confirmation clears pending project select', () {
    final store = RemoteRuntimeStore();
    store.applyProjectList(
      projects: _projects,
      remoteSelectedProjectId: 'project-1',
      terminalVisible: true,
      terminalListLoaded: false,
    );
    store.userSelectProject(project: _projects[1], terminalVisible: true);
    store.markProjectSelectSent('project-2');

    expect(store.pendingProjectSelect(), isNull);
    expect(store.pendingProjectSelect(includeSent: true), 'project-2');

    final confirmed = store.projectSelected('project-2');
    final retry = store.ensureTerminalForSelectedProject(
      terminalVisible: true,
      terminalListLoaded: true,
    );

    expect(confirmed.requestTerminalList, isTrue);
    expect(store.pendingProjectSelect(), isNull);
    expect(store.pendingProjectSelect(includeSent: true), isNull);
    expect(retry.requestProjectSelectId, isNull);
  });

  test('project selected confirmation waits for terminal list', () {
    final store = RemoteRuntimeStore();
    store.applyProjectList(
      projects: _projects,
      remoteSelectedProjectId: 'project-1',
      terminalVisible: true,
      terminalListLoaded: false,
    );
    store.userSelectProject(project: _projects[1], terminalVisible: true);

    final confirmed = store.projectSelected('project-2');

    expect(store.selectedProjectId, 'project-2');
    expect(confirmed.requestTerminalList, isTrue);
    expect(confirmed.requestProjectSelectId, isNull);

    final beforeList = store.applyProjectList(
      projects: _projects,
      remoteSelectedProjectId: 'project-2',
      terminalVisible: true,
      terminalListLoaded: true,
    );

    expect(beforeList.requestProjectSelectId, isNull);

    final terminalList = store.applyTerminalList(
      terminals: const [
        TerminalInfo(id: 'session-2', title: 'Two', projectId: 'project-2'),
      ],
      terminalVisible: true,
      terminalListLoaded: true,
    );

    expect(terminalList.bindSessionId, 'session-2');
    expect(store.activeSessionId, 'session-2');
  });

  test('project selected confirmation drops old active terminal session', () {
    final store = RemoteRuntimeStore();
    store.applyProjectList(
      projects: _projects,
      remoteSelectedProjectId: 'project-1',
      terminalVisible: true,
      terminalListLoaded: false,
    );
    store.applyTerminalList(
      terminals: const [
        TerminalInfo(id: 'session-1', title: 'One', projectId: 'project-1'),
      ],
      terminalVisible: true,
      terminalListLoaded: true,
    );

    final confirmed = store.projectSelected('project-2');

    expect(confirmed.requestTerminalList, isTrue);
    expect(confirmed.resetTerminalBuffer, isTrue);
    expect(store.selectedProjectId, 'project-2');
    expect(store.activeSessionId, isNull);

    final terminalList = store.applyTerminalList(
      terminals: const [
        TerminalInfo(id: 'session-1', title: 'One', projectId: 'project-1'),
        TerminalInfo(id: 'session-2', title: 'Two', projectId: 'project-2'),
      ],
      terminalVisible: true,
      terminalListLoaded: true,
    );

    expect(terminalList.bindSessionId, 'session-2');
    expect(store.activeSessionId, 'session-2');
  });

  test('background reset keeps project but drops terminal session state', () {
    final store = RemoteRuntimeStore();
    store.applyProjectList(
      projects: _projects,
      remoteSelectedProjectId: 'project-2',
      terminalVisible: true,
      terminalListLoaded: false,
    );
    store.applyTerminalList(
      terminals: const [
        TerminalInfo(id: 'session-2', title: 'Two', projectId: 'project-2'),
      ],
      terminalVisible: true,
      terminalListLoaded: true,
    );

    store.reset(keepProjects: true);

    expect(
      store.projects.map((project) => project.toJson()).toList(),
      _projects.map((project) => project.toJson()).toList(),
    );
    expect(store.selectedProjectId, 'project-2');
    expect(store.activeSessionId, isNull);
    expect(store.terminals, isEmpty);
  });

  test('stores git status by project in runtime state', () {
    final store = RemoteRuntimeStore();
    store.applyProjectList(
      projects: _projects,
      remoteSelectedProjectId: 'project-2',
      terminalVisible: false,
      terminalListLoaded: false,
    );

    final plan = store.applyGitStatus(
      const RemoteGitStatusInfo(
        projectId: 'project-2',
        projectPath: '/tmp/project-2',
        branch: 'main',
        ahead: 2,
        behind: 1,
        staged: 1,
        unstaged: 2,
        untracked: 3,
        changes: 6,
        isRepository: true,
      ),
    );

    expect(plan.stateChanged, isTrue);
    expect(store.selectedGitStatus?.branch, 'main');
    expect(store.selectedGitStatus?.changes, 6);
  });

  test('terminal scope follows active terminal project and path', () {
    final store = RemoteRuntimeStore();
    store.applyProjectList(
      projects: _projects,
      remoteSelectedProjectId: 'project-1',
      terminalVisible: true,
      terminalListLoaded: false,
    );
    store.applyTerminalList(
      terminals: const [
        TerminalInfo(id: 'session-1', title: 'One', projectId: 'project-1'),
        TerminalInfo(id: 'session-2', title: 'Two', projectId: 'project-2'),
      ],
      terminalVisible: true,
      terminalListLoaded: true,
    );

    final scope = store.terminalScopeForSession('session-2');

    expect(scope?.projectId, 'project-2');
    expect(scope?.projectPath, '/tmp/project-2');
  });

  test(
    'terminal close removes active session without changing selected project',
    () {
      final store = RemoteRuntimeStore();
      store.applyProjectList(
        projects: _projects,
        remoteSelectedProjectId: 'project-1',
        terminalVisible: true,
        terminalListLoaded: false,
      );
      store.applyTerminalList(
        terminals: const [
          TerminalInfo(id: 'session-1', title: 'One', projectId: 'project-1'),
        ],
        terminalVisible: true,
        terminalListLoaded: true,
      );

      final plan = store.removeTerminal('session-1');

      expect(plan.clearTerminal, isTrue);
      expect(plan.removedSessionId, 'session-1');
      expect(store.selectedProjectId, 'project-1');
      expect(store.activeSessionId, isNull);
    },
  );
}

const _projects = [
  ProjectInfo(id: 'project-1', name: 'Project 1', path: '/tmp/project-1'),
  ProjectInfo(id: 'project-2', name: 'Project 2', path: '/tmp/project-2'),
];
