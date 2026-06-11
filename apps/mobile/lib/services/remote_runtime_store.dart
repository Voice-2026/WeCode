import 'package:codux_protocol_ffi/codux_protocol_ffi.dart'
    as codux_runtime_core;

import '../models/remote_models.dart';
import 'remote_terminal_scope.dart';

class RemoteRuntimeState {
  const RemoteRuntimeState({
    this.projects = const [],
    this.terminals = const [],
    this.selectedProjectId,
    this.activeSessionId,
    this.pendingProjectSelectId,
    this.pendingProjectSelectSent = false,
    this.projectSelectAcknowledgedId,
    this.creatingTerminalProjectId,
    this.lastTerminalIdByProject = const {},
    this.gitStatusByProject = const {},
  });

  final List<ProjectInfo> projects;
  final List<TerminalInfo> terminals;
  final String? selectedProjectId;
  final String? activeSessionId;
  final String? pendingProjectSelectId;
  final bool pendingProjectSelectSent;
  final String? projectSelectAcknowledgedId;
  final String? creatingTerminalProjectId;
  final Map<String, String> lastTerminalIdByProject;
  final Map<String, RemoteGitStatusInfo> gitStatusByProject;
}

class RemoteRuntimePlan {
  const RemoteRuntimePlan({
    this.stateChanged = false,
    this.clearTerminal = false,
    this.resetTerminalInput = false,
    this.resetTerminalBuffer = false,
    this.requestTerminalList = false,
    this.requestProjectSelectId,
    this.bindSessionId,
    this.bindFullBuffer = false,
    this.flushTerminalInput = false,
    this.removedSessionId,
  });

  final bool stateChanged;
  final bool clearTerminal;
  final bool resetTerminalInput;
  final bool resetTerminalBuffer;
  final bool requestTerminalList;
  final String? requestProjectSelectId;
  final String? bindSessionId;
  final bool bindFullBuffer;
  final bool flushTerminalInput;
  final String? removedSessionId;
}

class RemoteRuntimeStore {
  RemoteRuntimeStore();

  final codux_runtime_core.RemoteRuntimeCore _core =
      codux_runtime_core.RemoteRuntimeCore();
  Map<String, RemoteGitStatusInfo> _gitStatusByProject = const {};

  RemoteRuntimeState get state => _stateFromCore();
  List<ProjectInfo> get projects => state.projects;
  List<TerminalInfo> get terminals => state.terminals;
  String? get selectedProjectId => state.selectedProjectId;
  String? get activeSessionId => state.activeSessionId;
  String? get creatingTerminalProjectId => state.creatingTerminalProjectId;
  Map<String, String> get lastTerminalIdByProject =>
      state.lastTerminalIdByProject;

  RemoteGitStatusInfo? gitStatusForProject(String projectId) =>
      _gitStatusByProject[projectId];

  RemoteGitStatusInfo? get selectedGitStatus {
    final projectId = selectedProjectId;
    return projectId == null ? null : _gitStatusByProject[projectId];
  }

  RemoteTerminalScope? terminalScopeForProject(String projectId) {
    final current = state;
    return remoteTerminalScopeForProject(
      projectId: projectId,
      projects: current.projects,
    );
  }

  RemoteTerminalScope? terminalScopeForSession(
    String sessionId, {
    TerminalInfo? terminal,
  }) {
    final current = state;
    return remoteTerminalScopeForSession(
      sessionId: sessionId,
      projects: current.projects,
      terminals: current.terminals,
      selectedProjectId: current.selectedProjectId,
      terminal: terminal,
    );
  }

  void reset({bool keepProjects = false}) {
    _core.reset(keepProjects: keepProjects);
  }

  void restoreCachedProjects(List<ProjectInfo> projects) {
    _core.restoreCachedProjects(projects.map(_projectToJson).toList());
  }

  RemoteRuntimePlan applyProjectList({
    required List<ProjectInfo> projects,
    required String? remoteSelectedProjectId,
    required bool terminalVisible,
    required bool terminalListLoaded,
  }) {
    return _planFromCore(
      _core.applyProjectList(
        projects: projects.map(_projectToJson).toList(),
        remoteSelectedProjectId: remoteSelectedProjectId,
        terminalVisible: terminalVisible,
        terminalListLoaded: terminalListLoaded,
      ),
    );
  }

  RemoteRuntimePlan applyTerminalList({
    required List<TerminalInfo> terminals,
    required bool terminalVisible,
    required bool terminalListLoaded,
  }) {
    return _planFromCore(
      _core.applyTerminalList(
        terminals: terminals.map(_terminalToJson).toList(),
        terminalVisible: terminalVisible,
        terminalListLoaded: terminalListLoaded,
      ),
    );
  }

  RemoteRuntimePlan userSelectProject({
    required ProjectInfo project,
    required bool terminalVisible,
  }) {
    return _planFromCore(
      _core.userSelectProject(
        project: _projectToJson(project),
        terminalVisible: terminalVisible,
      ),
    );
  }

  RemoteRuntimePlan projectSelected(String? projectId) {
    return _planFromCore(_core.projectSelected(projectId));
  }

  RemoteRuntimePlan ensureTerminalForSelectedProject({
    required bool terminalVisible,
    required bool terminalListLoaded,
  }) {
    return _planFromCore(
      _core.ensureTerminalForSelectedProject(
        terminalVisible: terminalVisible,
        terminalListLoaded: terminalListLoaded,
      ),
    );
  }

  RemoteRuntimePlan selectTerminal(TerminalInfo terminal) {
    return _planFromCore(_core.selectTerminal(_terminalToJson(terminal)));
  }

  RemoteRuntimePlan removeTerminal(String terminalId) {
    return _planFromCore(_core.removeTerminal(terminalId));
  }

  void setTerminalCreatingProject(String? projectId) {
    _core.setTerminalCreatingProject(projectId);
  }

  RemoteRuntimePlan terminalCreated(TerminalInfo terminal) {
    return _planFromCore(_core.terminalCreated(_terminalToJson(terminal)));
  }

  RemoteRuntimePlan applyGitStatus(RemoteGitStatusInfo status) {
    if (status.projectId.isEmpty) return const RemoteRuntimePlan();
    _gitStatusByProject = {..._gitStatusByProject, status.projectId: status};
    return const RemoteRuntimePlan(stateChanged: true);
  }

  ProjectInfo? selectedProject() {
    final id = selectedProjectId;
    if (id == null) return null;
    for (final project in projects) {
      if (project.id == id) return project;
    }
    return null;
  }

  TerminalInfo? activeTerminal() {
    final id = activeSessionId;
    if (id == null) return null;
    for (final terminal in terminals) {
      if (terminal.id == id) return terminal;
    }
    return null;
  }

  void markProjectSelectSent(String projectId) {
    _core.markProjectSelectSent(projectId);
  }

  void clearPendingProjectSelectSent(String projectId) {
    _core.clearProjectSelectSent(projectId);
  }

  String? pendingProjectSelect({bool includeSent = false}) {
    return _core.pendingProjectSelect(includeSent: includeSent);
  }

  List<TerminalInfo> currentProjectTerminals() {
    return _core.currentProjectTerminals().map(TerminalInfo.fromJson).toList();
  }

  static bool isAccessibleTerminal(TerminalInfo terminal) =>
      terminal.id.isNotEmpty && terminal.projectId.isNotEmpty;

  RemoteRuntimeState _stateFromCore() {
    final snapshot = _core.snapshot();
    return RemoteRuntimeState(
      projects: snapshot.projects.map(ProjectInfo.fromJson).toList(),
      terminals: snapshot.terminals.map(TerminalInfo.fromJson).toList(),
      selectedProjectId: snapshot.selectedProjectId,
      activeSessionId: snapshot.activeSessionId,
      pendingProjectSelectId: snapshot.pendingProjectSelectId,
      pendingProjectSelectSent: snapshot.pendingProjectSelectSent,
      projectSelectAcknowledgedId: snapshot.projectSelectAcknowledgedId,
      creatingTerminalProjectId: snapshot.creatingTerminalProjectId,
      lastTerminalIdByProject: snapshot.lastTerminalIdByProject,
      gitStatusByProject: _gitStatusByProject,
    );
  }
}

int compareTerminals(TerminalInfo left, TerminalInfo right) {
  final createdAt = (left.createdAt ?? '').compareTo(right.createdAt ?? '');
  if (createdAt != 0) return createdAt;
  return left.id.compareTo(right.id);
}

String terminalLayoutKind(TerminalInfo terminal) {
  final value = terminal.layoutKind.trim().toLowerCase();
  if (value == 'tab') return 'tab';
  return 'split';
}

RemoteRuntimePlan _planFromCore(codux_runtime_core.RemoteRuntimeCorePlan plan) {
  return RemoteRuntimePlan(
    stateChanged: plan.stateChanged,
    clearTerminal: plan.clearTerminal,
    resetTerminalInput: plan.resetTerminalInput,
    resetTerminalBuffer: plan.resetTerminalBuffer,
    requestTerminalList: plan.requestTerminalList,
    requestProjectSelectId: plan.requestProjectSelectId,
    bindSessionId: plan.bindSessionId,
    bindFullBuffer: plan.bindFullBuffer,
    flushTerminalInput: plan.flushTerminalInput,
    removedSessionId: plan.removedSessionId,
  );
}

Map<String, dynamic> _projectToJson(ProjectInfo project) => project.toJson();

Map<String, dynamic> _terminalToJson(TerminalInfo terminal) => {
  'id': terminal.id,
  'title': terminal.title,
  'projectId': terminal.projectId,
  'layoutKind': terminal.layoutKind,
  if (terminal.cols != null) 'cols': terminal.cols,
  if (terminal.rows != null) 'rows': terminal.rows,
  if (terminal.status != null) 'status': terminal.status,
  if (terminal.createdAt != null) 'createdAt': terminal.createdAt,
  if (terminal.bufferCharacters != null)
    'bufferCharacters': terminal.bufferCharacters,
};
