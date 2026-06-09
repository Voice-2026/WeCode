import '../models/remote_models.dart';
import 'remote_terminal_scope.dart';

class RemoteRuntimeState {
  const RemoteRuntimeState({
    this.projects = const [],
    this.terminals = const [],
    this.selectedProjectId,
    this.activeSessionId,
    this.pendingProjectSelectId,
    this.creatingTerminalProjectId,
    this.lastTerminalIdByProject = const {},
    this.gitStatusByProject = const {},
  });

  final List<ProjectInfo> projects;
  final List<TerminalInfo> terminals;
  final String? selectedProjectId;
  final String? activeSessionId;
  final String? pendingProjectSelectId;
  final String? creatingTerminalProjectId;
  final Map<String, String> lastTerminalIdByProject;
  final Map<String, RemoteGitStatusInfo> gitStatusByProject;

  RemoteRuntimeState copyWith({
    List<ProjectInfo>? projects,
    List<TerminalInfo>? terminals,
    Object? selectedProjectId = _unset,
    Object? activeSessionId = _unset,
    Object? pendingProjectSelectId = _unset,
    Object? creatingTerminalProjectId = _unset,
    Map<String, String>? lastTerminalIdByProject,
    Map<String, RemoteGitStatusInfo>? gitStatusByProject,
  }) => RemoteRuntimeState(
    projects: projects ?? this.projects,
    terminals: terminals ?? this.terminals,
    selectedProjectId: selectedProjectId == _unset
        ? this.selectedProjectId
        : selectedProjectId as String?,
    activeSessionId: activeSessionId == _unset
        ? this.activeSessionId
        : activeSessionId as String?,
    pendingProjectSelectId: pendingProjectSelectId == _unset
        ? this.pendingProjectSelectId
        : pendingProjectSelectId as String?,
    creatingTerminalProjectId: creatingTerminalProjectId == _unset
        ? this.creatingTerminalProjectId
        : creatingTerminalProjectId as String?,
    lastTerminalIdByProject:
        lastTerminalIdByProject ?? this.lastTerminalIdByProject,
    gitStatusByProject: gitStatusByProject ?? this.gitStatusByProject,
  );
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

  RemoteRuntimeState _state = const RemoteRuntimeState();

  RemoteRuntimeState get state => _state;
  List<ProjectInfo> get projects => _state.projects;
  List<TerminalInfo> get terminals => _state.terminals;
  String? get selectedProjectId => _state.selectedProjectId;
  String? get activeSessionId => _state.activeSessionId;
  String? get creatingTerminalProjectId => _state.creatingTerminalProjectId;
  Map<String, String> get lastTerminalIdByProject =>
      _state.lastTerminalIdByProject;
  RemoteGitStatusInfo? gitStatusForProject(String projectId) =>
      _state.gitStatusByProject[projectId];
  RemoteGitStatusInfo? get selectedGitStatus {
    final projectId = _state.selectedProjectId;
    return projectId == null ? null : _state.gitStatusByProject[projectId];
  }

  RemoteTerminalScope? terminalScopeForProject(String projectId) {
    return remoteTerminalScopeForProject(
      projectId: projectId,
      projects: _state.projects,
    );
  }

  RemoteTerminalScope? terminalScopeForSession(
    String sessionId, {
    TerminalInfo? terminal,
  }) {
    return remoteTerminalScopeForSession(
      sessionId: sessionId,
      projects: _state.projects,
      terminals: _state.terminals,
      selectedProjectId: _state.selectedProjectId,
      terminal: terminal,
    );
  }

  void reset({bool keepProjects = false}) {
    final projects = keepProjects ? _state.projects : const <ProjectInfo>[];
    final selected =
        keepProjects &&
            _state.selectedProjectId != null &&
            projects.any((item) => item.id == _state.selectedProjectId)
        ? _state.selectedProjectId
        : null;
    _state = RemoteRuntimeState(
      projects: projects,
      selectedProjectId: selected,
    );
  }

  void restoreCachedProjects(List<ProjectInfo> projects) {
    if (projects.isEmpty || _state.projects.isNotEmpty) return;
    _state = _state.copyWith(
      projects: projects,
      selectedProjectId: projects.first.id,
    );
  }

  RemoteRuntimePlan applyProjectList({
    required List<ProjectInfo> projects,
    required String? remoteSelectedProjectId,
    required bool terminalVisible,
    required bool terminalListLoaded,
  }) {
    final previousSelected = _state.selectedProjectId;
    final selected = _selectedProjectFromList(
      projects: projects,
      pendingProjectSelectId: _state.pendingProjectSelectId,
      remoteSelectedProjectId: remoteSelectedProjectId,
      currentSelectedProjectId: previousSelected,
    );
    final projectChanged = selected != previousSelected;
    _state = _state.copyWith(
      projects: projects,
      selectedProjectId: selected,
      activeSessionId: projectChanged ? null : _state.activeSessionId,
    );
    final bind = ensureTerminalForSelectedProject(
      terminalVisible: terminalVisible,
      terminalListLoaded: terminalListLoaded,
    );
    return RemoteRuntimePlan(
      stateChanged: true,
      clearTerminal: projectChanged && terminalVisible,
      resetTerminalInput: projectChanged && terminalVisible,
      resetTerminalBuffer: projectChanged && terminalVisible,
      requestTerminalList: bind.requestTerminalList,
      requestProjectSelectId: bind.requestProjectSelectId,
      bindSessionId: bind.bindSessionId,
      bindFullBuffer: bind.bindFullBuffer,
      flushTerminalInput: bind.flushTerminalInput,
    );
  }

  RemoteRuntimePlan applyTerminalList({
    required List<TerminalInfo> terminals,
    required bool terminalVisible,
    required bool terminalListLoaded,
  }) {
    final activeMissing =
        _state.activeSessionId != null &&
        !terminals.any(
          (item) =>
              item.id == _state.activeSessionId && _isAccessibleTerminal(item),
        );
    var lastByProject = Map<String, String>.from(
      _state.lastTerminalIdByProject,
    );
    String? removedSessionId;
    if (activeMissing) {
      removedSessionId = _state.activeSessionId;
      lastByProject.removeWhere(
        (_, terminalId) => terminalId == removedSessionId,
      );
    }
    final selected = _state.selectedProjectId;
    var pending = _state.pendingProjectSelectId;
    if (selected != null &&
        terminals.any(
          (item) => item.projectId == selected && _isAccessibleTerminal(item),
        ) &&
        pending == selected) {
      pending = null;
    }
    _state = _state.copyWith(
      terminals: terminals,
      activeSessionId: activeMissing ? null : _state.activeSessionId,
      pendingProjectSelectId: pending,
      lastTerminalIdByProject: lastByProject,
    );
    final bind = ensureTerminalForSelectedProject(
      terminalVisible: terminalVisible,
      terminalListLoaded: terminalListLoaded,
    );
    return RemoteRuntimePlan(
      stateChanged: true,
      resetTerminalInput: activeMissing,
      resetTerminalBuffer: activeMissing || bind.resetTerminalBuffer,
      removedSessionId: removedSessionId,
      requestTerminalList: bind.requestTerminalList,
      requestProjectSelectId: bind.requestProjectSelectId,
      bindSessionId: bind.bindSessionId,
      bindFullBuffer: bind.bindFullBuffer,
      flushTerminalInput: bind.flushTerminalInput,
    );
  }

  RemoteRuntimePlan userSelectProject({
    required ProjectInfo project,
    required bool terminalVisible,
  }) {
    final projectChanged = _state.selectedProjectId != project.id;
    final previousProjectId = _state.selectedProjectId;
    final lastByProject = Map<String, String>.from(
      _state.lastTerminalIdByProject,
    );
    if (projectChanged &&
        previousProjectId != null &&
        _state.activeSessionId != null &&
        _state.terminals.any(
          (item) =>
              item.id == _state.activeSessionId &&
              item.projectId == previousProjectId &&
              _isAccessibleTerminal(item),
        )) {
      lastByProject[previousProjectId] = _state.activeSessionId!;
    }
    final existing = terminalVisible
        ? _state.terminals
              .where(
                (item) =>
                    item.projectId == project.id && _isAccessibleTerminal(item),
              )
              .toList()
        : const <TerminalInfo>[];
    final terminal = existing.isEmpty
        ? null
        : _preferredTerminalForProject(project.id, existing);
    if (terminal != null) {
      lastByProject[project.id] = terminal.id;
    }
    _state = _state.copyWith(
      selectedProjectId: project.id,
      activeSessionId:
          terminal?.id ??
          (projectChanged && terminalVisible ? null : _state.activeSessionId),
      pendingProjectSelectId: project.id,
      lastTerminalIdByProject: lastByProject,
    );
    return RemoteRuntimePlan(
      stateChanged: true,
      resetTerminalInput: projectChanged && terminalVisible,
      resetTerminalBuffer: projectChanged && terminalVisible,
      requestProjectSelectId: project.id,
      bindSessionId: terminal?.id,
      bindFullBuffer: terminal != null,
      flushTerminalInput: terminal != null,
    );
  }

  RemoteRuntimePlan projectSelected(String? projectId) {
    final selected = projectId?.trim();
    if (selected == null || selected.isEmpty) {
      return const RemoteRuntimePlan();
    }
    if (_state.selectedProjectId != selected &&
        !_state.projects.any((item) => item.id == selected)) {
      return const RemoteRuntimePlan();
    }
    _state = _state.copyWith(selectedProjectId: selected);
    return const RemoteRuntimePlan(
      stateChanged: true,
      requestTerminalList: true,
    );
  }

  RemoteRuntimePlan ensureTerminalForSelectedProject({
    required bool terminalVisible,
    required bool terminalListLoaded,
  }) {
    if (!terminalVisible) return const RemoteRuntimePlan();
    final projectId = _state.selectedProjectId;
    if (projectId == null) return const RemoteRuntimePlan();
    if (!terminalListLoaded) {
      return const RemoteRuntimePlan(requestTerminalList: true);
    }
    final activeId = _state.activeSessionId;
    if (activeId != null &&
        _state.terminals.any(
          (item) =>
              item.id == activeId &&
              item.projectId == projectId &&
              _isAccessibleTerminal(item),
        )) {
      return const RemoteRuntimePlan();
    }
    final existing = _state.terminals
        .where(
          (item) => item.projectId == projectId && _isAccessibleTerminal(item),
        )
        .toList();
    if (existing.isEmpty) {
      if (_state.pendingProjectSelectId == projectId) {
        return const RemoteRuntimePlan();
      }
      _state = _state.copyWith(pendingProjectSelectId: projectId);
      return RemoteRuntimePlan(requestProjectSelectId: projectId);
    }
    final terminal = _preferredTerminalForProject(projectId, existing);
    final lastByProject = Map<String, String>.from(
      _state.lastTerminalIdByProject,
    )..[projectId] = terminal.id;
    _state = _state.copyWith(
      activeSessionId: terminal.id,
      pendingProjectSelectId: null,
      creatingTerminalProjectId: null,
      lastTerminalIdByProject: lastByProject,
    );
    return RemoteRuntimePlan(
      stateChanged: true,
      resetTerminalBuffer: true,
      bindSessionId: terminal.id,
      bindFullBuffer: true,
      flushTerminalInput: true,
    );
  }

  RemoteRuntimePlan selectTerminal(TerminalInfo terminal) {
    if (!_isAccessibleTerminal(terminal)) return const RemoteRuntimePlan();
    final lastByProject = Map<String, String>.from(
      _state.lastTerminalIdByProject,
    )..[terminal.projectId] = terminal.id;
    _state = _state.copyWith(
      selectedProjectId: terminal.projectId,
      activeSessionId: terminal.id,
      pendingProjectSelectId: null,
      creatingTerminalProjectId: null,
      lastTerminalIdByProject: lastByProject,
    );
    return RemoteRuntimePlan(
      stateChanged: true,
      resetTerminalInput: true,
      resetTerminalBuffer: true,
      bindSessionId: terminal.id,
      bindFullBuffer: true,
    );
  }

  RemoteRuntimePlan removeTerminal(String terminalId) {
    final closingActive = _state.activeSessionId == terminalId;
    final lastByProject = Map<String, String>.from(
      _state.lastTerminalIdByProject,
    )..removeWhere((_, id) => id == terminalId);
    _state = _state.copyWith(
      terminals: _state.terminals
          .where((item) => item.id != terminalId)
          .toList(),
      activeSessionId: closingActive ? null : _state.activeSessionId,
      lastTerminalIdByProject: lastByProject,
    );
    return RemoteRuntimePlan(
      stateChanged: true,
      clearTerminal: closingActive,
      resetTerminalInput: closingActive,
      resetTerminalBuffer: closingActive,
      removedSessionId: terminalId,
    );
  }

  void setTerminalCreatingProject(String? projectId) {
    _state = _state.copyWith(creatingTerminalProjectId: projectId);
  }

  RemoteRuntimePlan terminalCreated(TerminalInfo terminal) {
    if (!_isAccessibleTerminal(terminal)) return const RemoteRuntimePlan();
    final terminals = [
      terminal,
      ..._state.terminals.where((item) => item.id != terminal.id),
    ];
    final lastByProject = Map<String, String>.from(
      _state.lastTerminalIdByProject,
    )..[terminal.projectId] = terminal.id;
    _state = _state.copyWith(
      terminals: terminals,
      selectedProjectId: terminal.projectId,
      activeSessionId: terminal.id,
      pendingProjectSelectId: null,
      creatingTerminalProjectId: null,
      lastTerminalIdByProject: lastByProject,
    );
    return RemoteRuntimePlan(
      stateChanged: true,
      clearTerminal: true,
      resetTerminalBuffer: true,
      bindSessionId: terminal.id,
      bindFullBuffer: true,
      flushTerminalInput: true,
    );
  }

  RemoteRuntimePlan applyGitStatus(RemoteGitStatusInfo status) {
    if (status.projectId.isEmpty) return const RemoteRuntimePlan();
    final next = Map<String, RemoteGitStatusInfo>.from(
      _state.gitStatusByProject,
    )..[status.projectId] = status;
    _state = _state.copyWith(gitStatusByProject: next);
    return const RemoteRuntimePlan(stateChanged: true);
  }

  ProjectInfo? selectedProject() {
    final id = _state.selectedProjectId;
    if (id == null) return null;
    for (final project in _state.projects) {
      if (project.id == id) return project;
    }
    return null;
  }

  TerminalInfo? activeTerminal() {
    final id = _state.activeSessionId;
    if (id == null) return null;
    for (final terminal in _state.terminals) {
      if (terminal.id == id) return terminal;
    }
    return null;
  }

  List<TerminalInfo> currentProjectTerminals() {
    final projectId = _state.selectedProjectId;
    if (projectId == null) return const [];
    final list = _state.terminals
        .where(
          (item) => item.projectId == projectId && _isAccessibleTerminal(item),
        )
        .toList();
    list.sort(compareTerminals);
    return list;
  }

  static bool isAccessibleTerminal(TerminalInfo terminal) =>
      _isAccessibleTerminal(terminal);

  static bool _isAccessibleTerminal(TerminalInfo terminal) =>
      terminal.id.isNotEmpty && terminal.projectId.isNotEmpty;

  TerminalInfo _preferredTerminalForProject(
    String projectId,
    Iterable<TerminalInfo> terminals,
  ) {
    final list = terminals.toList()..sort(compareTerminals);
    final rememberedId = _state.lastTerminalIdByProject[projectId];
    if (rememberedId != null) {
      for (final terminal in list) {
        if (terminal.id == rememberedId) return terminal;
      }
    }
    final splits = list
        .where((terminal) => terminalLayoutKind(terminal) == 'split')
        .toList();
    if (splits.isNotEmpty) return splits.first;
    return list.first;
  }
}

String? _selectedProjectFromList({
  required List<ProjectInfo> projects,
  required String? pendingProjectSelectId,
  required String? remoteSelectedProjectId,
  required String? currentSelectedProjectId,
}) {
  final pending = pendingProjectSelectId?.trim();
  if (pending != null &&
      pending.isNotEmpty &&
      projects.any((item) => item.id == pending)) {
    return pending;
  }
  final remote = remoteSelectedProjectId?.trim();
  if (remote != null &&
      remote.isNotEmpty &&
      projects.any((item) => item.id == remote)) {
    return remote;
  }
  if (projects.any((item) => item.id == currentSelectedProjectId)) {
    return currentSelectedProjectId;
  }
  return projects.isNotEmpty ? projects.first.id : null;
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

const Object _unset = Object();
