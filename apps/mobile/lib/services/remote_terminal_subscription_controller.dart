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
  }) {
    final cleanProjectId = projectId.trim();
    if (cleanProjectId.isEmpty) {
      return const RemoteTerminalSubscriptionPlan();
    }
    final alreadySubscribed = _subscribedProjectIds.contains(cleanProjectId);
    final baselineAlreadyRequested = _baselineRequestedProjectIds.contains(
      cleanProjectId,
    );
    if (alreadySubscribed && (!baseline || baselineAlreadyRequested)) {
      _projectId = cleanProjectId;
      return const RemoteTerminalSubscriptionPlan();
    }
    _projectId = cleanProjectId;
    _subscribedProjectIds.add(cleanProjectId);
    if (baseline) {
      _baselineRequestedProjectIds.add(cleanProjectId);
    }
    return RemoteTerminalSubscriptionPlan(
      subscribe: remoteResourceSubscribeEnvelope(
        resource: RemoteResourceType.terminals,
        projectId: cleanProjectId,
        baseline: baseline,
        maxChars: maxChars,
        chunkChars: chunkChars,
        requestId: requestId,
      ),
      subscribeProjectId: cleanProjectId,
      requestId: requestId,
    );
  }
}
