import '../models/remote_models.dart';
import 'remote_path_utils.dart';
import 'remote_protocol.dart';
import 'remote_runtime_payloads.dart';

const int remoteFileHighlightMaxChars = 80000;
const int remoteFileEditableMaxChars = 200000;

class RemoteProjectFileController {
  final Map<String, String> _pathMemory = {};

  String? pathForProject(ProjectInfo project, {String currentPath = ''}) {
    final rememberedPath = _pathMemory[project.id];
    if (rememberedPath != null && rememberedPath.isNotEmpty) {
      return rememberedPath;
    }
    return currentPath.isNotEmpty ? currentPath : project.path;
  }

  void remember({required String projectId, required String path}) {
    if (projectId.isEmpty || path.isEmpty) return;
    _pathMemory[projectId] = path;
  }

  void forget(String projectId) {
    _pathMemory.remove(projectId);
  }

  RelayEnvelope listEnvelope(String path) {
    return RelayEnvelope(
      type: RemoteMessageType.fileList,
      payload: {'path': path, 'purpose': 'projectFiles'},
    );
  }

  RemoteFileListState? listStateFromPayload(Object? payload) {
    if (payload is! Map) return null;
    return RemoteFileListState(
      purpose: payload['purpose']?.toString(),
      path: '${payload['path'] ?? ''}',
      parent: payload['parent']?.toString(),
      entries: remoteFileEntriesFromPayload(payload),
    );
  }

  RelayEnvelope readEnvelope(RemoteFileEntry entry) {
    return RelayEnvelope(
      type: RemoteMessageType.fileRead,
      payload: {'path': entry.path},
    );
  }

  RemoteFileEditorState beginReadState(RemoteFileEntry entry) {
    return RemoteFileEditorState(
      path: entry.path,
      content: '',
      loading: true,
      saving: false,
      editing: false,
      editable: true,
      highlightEnabled: true,
    );
  }

  RemoteFileEditorState? readStateFromPayload(Object? payload) {
    if (payload is! Map) return null;
    final content = '${payload['content'] ?? ''}';
    return RemoteFileEditorState(
      path: '${payload['path'] ?? ''}',
      content: content,
      loading: false,
      saving: false,
      editing: false,
      editable: content.length <= remoteFileEditableMaxChars,
      highlightEnabled: content.length <= remoteFileHighlightMaxChars,
    );
  }

  RelayEnvelope writeEnvelope({required String path, required String content}) {
    return RelayEnvelope(
      type: RemoteMessageType.fileWrite,
      payload: {'path': path, 'content': content},
    );
  }

  RelayEnvelope deleteEnvelope(RemoteFileEntry entry) {
    return RelayEnvelope(
      type: RemoteMessageType.fileDelete,
      payload: {'path': entry.path},
    );
  }

  String? deletedPathFromPayload(Object? payload) {
    return payload is Map ? payload['path']?.toString() : null;
  }

  bool shouldCloseEditorAfterDelete({
    required String? deletedPath,
    required String? editingPath,
  }) {
    return deletedPath != null && deletedPath == editingPath;
  }

  RemoteFileRenamePlan? renamePlan(RemoteFileEntry entry, String nextName) {
    final name = nextName.trim();
    if (name.isEmpty || name == entry.name) return null;
    if (name.contains('/')) {
      return const RemoteFileRenamePlan.invalid();
    }
    final parent = remoteParentPathOf(entry.path);
    final newPath = parent == '/' ? '/$name' : '$parent/$name';
    return RemoteFileRenamePlan.valid(
      RelayEnvelope(
        type: RemoteMessageType.fileRename,
        payload: {'path': entry.path, 'newPath': newPath},
      ),
    );
  }
}

class RemoteFileEditorState {
  const RemoteFileEditorState({
    required this.path,
    required this.content,
    required this.loading,
    required this.saving,
    required this.editing,
    required this.editable,
    required this.highlightEnabled,
  });

  final String path;
  final String content;
  final bool loading;
  final bool saving;
  final bool editing;
  final bool editable;
  final bool highlightEnabled;
}

class RemoteFileListState {
  const RemoteFileListState({
    required this.purpose,
    required this.path,
    required this.parent,
    required this.entries,
  });

  final String? purpose;
  final String path;
  final String? parent;
  final List<RemoteFileEntry> entries;

  bool get isProjectFiles => purpose == 'projectFiles';
}

class RemoteFileRenamePlan {
  const RemoteFileRenamePlan._({required this.valid, this.envelope});

  const RemoteFileRenamePlan.invalid() : this._(valid: false);

  const RemoteFileRenamePlan.valid(RelayEnvelope envelope)
    : this._(valid: true, envelope: envelope);

  final bool valid;
  final RelayEnvelope? envelope;
}
