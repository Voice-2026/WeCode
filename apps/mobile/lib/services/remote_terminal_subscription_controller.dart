import '../models/remote_models.dart';
import 'remote_protocol.dart';

class RemoteTerminalSubscriptionPlan {
  const RemoteTerminalSubscriptionPlan({
    this.unsubscribe,
    this.unsubscribeProjectId,
    this.subscribe,
    this.subscribeProjectId,
    this.requestId,
  });

  final RelayEnvelope? unsubscribe;
  final String? unsubscribeProjectId;
  final RelayEnvelope? subscribe;
  final String? subscribeProjectId;
  final String? requestId;

  bool get hasWork => unsubscribe != null || subscribe != null;
}

class RemoteTerminalSubscriptionCommit {
  const RemoteTerminalSubscriptionCommit({
    required this.projectId,
    required this.baseline,
  });

  final String projectId;
  final bool baseline;
}

class RemoteTerminalSubscriptionController {
  final Set<String> _subscribedProjectIds = <String>{};
  final Set<String> _baselineRequestedProjectIds = <String>{};
  String? _projectId;

  String? get projectId => _projectId;

  void reset() {
    _subscribedProjectIds.clear();
    _baselineRequestedProjectIds.clear();
    _projectId = null;
  }

  void markProjectBaselineStale(String projectId) {
    _baselineRequestedProjectIds.remove(projectId.trim());
  }

  RemoteTerminalSubscriptionPlan replaceProject(
    String projectId, {
    bool baseline = true,
    int? maxChars,
    int? chunkChars,
    String? requestId,
    String? baselineSessionId,
    int? viewportCols,
    int? viewportRows,
  }) {
    final cleanProjectId = projectId.trim();
    if (cleanProjectId.isEmpty) {
      return const RemoteTerminalSubscriptionPlan();
    }
    final alreadySubscribed = _subscribedProjectIds.contains(cleanProjectId);
    final baselineAlreadyRequested = _baselineRequestedProjectIds.contains(
      cleanProjectId,
    );
    final previousProjectId = _projectId?.trim();
    final switchingProject =
        previousProjectId != null &&
        previousProjectId.isNotEmpty &&
        previousProjectId != cleanProjectId;
    if (!switchingProject &&
        alreadySubscribed &&
        (!baseline || baselineAlreadyRequested)) {
      _projectId = cleanProjectId;
      return const RemoteTerminalSubscriptionPlan();
    }
    final shouldUnsubscribePrevious =
        switchingProject && _subscribedProjectIds.contains(previousProjectId);
    final effectiveBaseline = baseline || switchingProject;
    return RemoteTerminalSubscriptionPlan(
      unsubscribe: shouldUnsubscribePrevious
          ? remoteResourceUnsubscribeEnvelope(
              resource: RemoteResourceType.terminals,
              projectId: previousProjectId,
            )
          : null,
      unsubscribeProjectId: shouldUnsubscribePrevious
          ? previousProjectId
          : null,
      subscribe: remoteResourceSubscribeEnvelope(
        resource: RemoteResourceType.terminals,
        projectId: cleanProjectId,
        baseline: effectiveBaseline,
        maxChars: maxChars,
        chunkChars: chunkChars,
        requestId: requestId,
        baselineSessionId: baselineSessionId,
        viewportCols: viewportCols,
        viewportRows: viewportRows,
      ),
      subscribeProjectId: cleanProjectId,
      requestId: requestId,
    );
  }

  RemoteTerminalSubscriptionCommit commitFor(
    RemoteTerminalSubscriptionPlan plan,
  ) {
    final projectId = plan.subscribeProjectId?.trim();
    if (projectId == null || projectId.isEmpty) {
      return const RemoteTerminalSubscriptionCommit(
        projectId: '',
        baseline: false,
      );
    }
    return RemoteTerminalSubscriptionCommit(
      projectId: projectId,
      baseline: plan.subscribe?.payload is Map
          ? ((plan.subscribe!.payload as Map)['baseline'] == true)
          : false,
    );
  }

  void markProjectSubscribed(
    String projectId, {
    required bool baselineRequested,
  }) {
    final cleanProjectId = projectId.trim();
    if (cleanProjectId.isEmpty) return;
    final previousProjectId = _projectId?.trim();
    if (previousProjectId != null &&
        previousProjectId.isNotEmpty &&
        previousProjectId != cleanProjectId) {
      _subscribedProjectIds.remove(previousProjectId);
      _baselineRequestedProjectIds.remove(previousProjectId);
    }
    _projectId = cleanProjectId;
    _subscribedProjectIds.add(cleanProjectId);
    if (baselineRequested) {
      _baselineRequestedProjectIds.add(cleanProjectId);
    }
  }
}
