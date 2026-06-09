import '../models/remote_models.dart';

class RemoteTerminalSubscriptionPlan {
  const RemoteTerminalSubscriptionPlan({
    this.unsubscribe,
    this.unsubscribeProjectId,
    this.subscribe,
    this.subscribeProjectId,
  });

  final RelayEnvelope? unsubscribe;
  final String? unsubscribeProjectId;
  final RelayEnvelope? subscribe;
  final String? subscribeProjectId;

  bool get hasWork => unsubscribe != null || subscribe != null;
}

class RemoteTerminalSubscriptionController {
  String? _projectId;

  String? get projectId => _projectId;

  void reset() {
    _projectId = null;
  }

  RemoteTerminalSubscriptionPlan replaceProject(
    String projectId, {
    bool baseline = true,
    int? maxChars,
    int? chunkChars,
  }) {
    final cleanProjectId = projectId.trim();
    if (cleanProjectId.isEmpty || _projectId == cleanProjectId) {
      return const RemoteTerminalSubscriptionPlan();
    }
    final previousProjectId = _projectId;
    _projectId = cleanProjectId;
    return RemoteTerminalSubscriptionPlan(
      unsubscribe: previousProjectId == null
          ? null
          : _projectEnvelope('terminal.unsubscribe', previousProjectId),
      unsubscribeProjectId: previousProjectId,
      subscribe: _projectEnvelope(
        'terminal.subscribe',
        cleanProjectId,
        baseline: baseline,
        maxChars: maxChars,
        chunkChars: chunkChars,
      ),
      subscribeProjectId: cleanProjectId,
    );
  }
}

RelayEnvelope _projectEnvelope(
  String type,
  String projectId, {
  bool baseline = false,
  int? maxChars,
  int? chunkChars,
}) {
  return RelayEnvelope(
    type: type,
    payload: {
      'scope': 'project',
      'projectId': projectId,
      if (baseline) 'baseline': true,
      if (maxChars != null && maxChars > 0) 'maxChars': maxChars,
      if (chunkChars != null && chunkChars > 0) 'chunkChars': chunkChars,
    },
  );
}
