import '../models/remote_models.dart';
import 'remote_path_utils.dart';
import 'remote_protocol.dart';

enum ProjectFormMode { add, edit }

class ProjectFormDraft {
  const ProjectFormDraft({
    required this.mode,
    required this.name,
    required this.path,
  });

  final ProjectFormMode mode;
  final String name;
  final String path;
}

class ProjectFolderSelection {
  const ProjectFolderSelection({required this.path, required this.name});

  final String path;
  final String name;
}

class ProjectSavePlan {
  const ProjectSavePlan._({
    required this.valid,
    required this.envelope,
    required this.name,
  });

  const ProjectSavePlan.invalid()
    : this._(valid: false, envelope: null, name: '');

  const ProjectSavePlan.valid({
    required RelayEnvelope envelope,
    required String name,
  }) : this._(valid: true, envelope: envelope, name: name);

  final bool valid;
  final RelayEnvelope? envelope;
  final String name;
}

class RemoteProjectController {
  const RemoteProjectController();

  ProjectFormDraft editDraft(ProjectInfo project) {
    return ProjectFormDraft(
      mode: ProjectFormMode.edit,
      name: project.name,
      path: project.path ?? '',
    );
  }

  ProjectFormDraft addDraft() {
    return const ProjectFormDraft(
      mode: ProjectFormMode.add,
      name: '',
      path: '',
    );
  }

  ProjectSavePlan savePlan({
    required ProjectFormMode mode,
    required String path,
    required String name,
    ProjectInfo? selectedProject,
  }) {
    final cleanPath = path.trim();
    if (cleanPath.isEmpty) return const ProjectSavePlan.invalid();
    final cleanName = name.trim().isEmpty
        ? remoteLastPathComponent(cleanPath)
        : name.trim();
    if (mode == ProjectFormMode.edit) {
      final project = selectedProject;
      if (project == null) return const ProjectSavePlan.invalid();
      return ProjectSavePlan.valid(
        name: cleanName,
        envelope: RelayEnvelope(
          type: RemoteMessageType.projectEdit,
          payload: {
            'projectId': project.id,
            'path': cleanPath,
            'name': cleanName,
          },
        ),
      );
    }
    return ProjectSavePlan.valid(
      name: cleanName,
      envelope: RelayEnvelope(
        type: RemoteMessageType.projectAdd,
        payload: {'path': cleanPath, 'name': cleanName},
      ),
    );
  }

  RelayEnvelope filePickerListEnvelope(String? path) {
    final cleanPath = path?.trim() ?? '';
    return RelayEnvelope(
      type: RemoteMessageType.fileList,
      payload: cleanPath.isEmpty ? <String, Object>{} : {'path': cleanPath},
    );
  }

  RelayEnvelope removeEnvelope(ProjectInfo project) {
    return RelayEnvelope(
      type: RemoteMessageType.projectRemove,
      payload: {'projectId': project.id},
    );
  }

  RelayEnvelope aiStatsEnvelope(ProjectInfo project) {
    return RelayEnvelope(
      type: RemoteMessageType.aiStats,
      payload: {'projectId': project.id},
    );
  }

  RelayEnvelope gitStatusEnvelope(ProjectInfo project) {
    return RelayEnvelope(
      type: RemoteMessageType.gitStatus,
      payload: {
        'projectId': project.id,
        if (project.path != null) 'projectPath': project.path,
      },
    );
  }

  String folderDisplayName(RemoteFileEntry entry) {
    return entry.name.isEmpty
        ? remoteLastPathComponent(entry.path)
        : entry.name;
  }

  ProjectFolderSelection selectFolder({
    required RemoteFileEntry entry,
    required String currentName,
  }) {
    final cleanName = currentName.trim();
    return ProjectFolderSelection(
      path: entry.path,
      name: cleanName.isEmpty ? folderDisplayName(entry) : cleanName,
    );
  }
}
