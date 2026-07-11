import '../models/remote_models.dart';
import 'log_service.dart';
import 'remote_capabilities.dart';
import 'remote_protocol.dart';
import 'remote_runtime_store.dart';
import 'remote_terminal_output_controller.dart';
import 'remote_terminal_subscription_controller.dart';

typedef RemoteTerminalSend = bool Function(RelayEnvelope envelope);
typedef RemoteTerminalLookup = TerminalInfo? Function(String sessionId);
typedef RemoteTerminalRequestIdFactory = String Function(String scope);
typedef RemoteTerminalViewportSizeProvider =
    ({int cols, int rows})? Function(String sessionId);

class RemoteTerminalBindResult {
  const RemoteTerminalBindResult({
    required this.baselineRequested,
    required this.restored,
  });

  final bool baselineRequested;
  final bool restored;
}

class RemoteTerminalBindingCoordinator {
  RemoteTerminalBindingCoordinator({
    required RemoteTerminalOutputController outputController,
    required RemoteTerminalSend send,
    required RemoteTerminalLookup terminalById,
    required RemoteTerminalRequestIdFactory nextRequestId,
    RemoteTerminalViewportSizeProvider? viewportSize,
    int maxCharsLimit = TerminalBufferCapability.mobileMaxChars,
  }) : _outputController = outputController,
       _send = send,
       _terminalById = terminalById,
       _nextRequestId = nextRequestId,
       _viewportSize = viewportSize,
       _maxCharsLimit = maxCharsLimit;

  final RemoteTerminalOutputController _outputController;
  final RemoteTerminalSubscriptionController _subscriptions =
      RemoteTerminalSubscriptionController();
  final RemoteTerminalSend _send;
  final RemoteTerminalLookup _terminalById;
  final RemoteTerminalRequestIdFactory _nextRequestId;
  final RemoteTerminalViewportSizeProvider? _viewportSize;
  final int _maxCharsLimit;
  final Set<String> _baselineStaleSessionIds = <String>{};

  void reset() {
    _subscriptions.reset();
    _baselineStaleSessionIds.clear();
  }

  void markSessionBaselineStale(String sessionId) {
    final cleanSessionId = sessionId.trim();
    if (cleanSessionId.isEmpty) return;
    _baselineStaleSessionIds.add(cleanSessionId);
  }

  bool isSessionBaselineStale(String sessionId) {
    return _baselineStaleSessionIds.contains(sessionId.trim());
  }

  void clearSessionBaselineStale(String? sessionId) {
    final cleanSessionId = sessionId?.trim();
    if (cleanSessionId == null || cleanSessionId.isEmpty) return;
    _baselineStaleSessionIds.remove(cleanSessionId);
  }

  bool replaceProjectSubscription({
    required String projectId,
    required String reason,
    required TerminalBufferCapability capability,
    required String? activeSessionId,
    bool baseline = true,
  }) {
    final maxChars = capability.maxChars.clamp(1, _maxCharsLimit);
    final requestId = _nextRequestId('project-$projectId');
    final viewportSize = activeSessionId == null
        ? null
        : _viewportSize?.call(activeSessionId);
    final plan = _subscriptions.replaceProject(
      projectId,
      baseline: baseline,
      maxChars: maxChars,
      chunkChars: capability.chunking ? capability.chunkChars : null,
      requestId: requestId,
      baselineSessionId: activeSessionId,
      viewportCols: viewportSize?.cols,
      viewportRows: viewportSize?.rows,
    );
    if (!plan.hasWork) return false;

    final unsubscribe = plan.unsubscribe;
    if (unsubscribe != null) {
      WeCodeLog.debug(
        '[wecode-flutter-terminal] unsubscribe project=${plan.unsubscribeProjectId ?? ''} reason=$reason',
      );
      _send(unsubscribe);
    }

    final subscribe = plan.subscribe;
    var baselineRequested = false;
    if (subscribe != null) {
      final currentTerminal = activeSessionId == null
          ? null
          : _terminalById(activeSessionId);
      final activeBelongsToProject =
          activeSessionId != null &&
          activeSessionId.isNotEmpty &&
          currentTerminal?.projectId == projectId;
      final commit = _subscriptions.commitFor(plan);
      if (commit.baseline && activeBelongsToProject) {
        final started = _outputController.startBufferRequest(
          activeSessionId,
          requestId,
          requireBaseline: true,
          resetAssembler: true,
        );
        if (!started) return false;
      }

      WeCodeLog.debug(
        '[wecode-flutter-terminal] subscribe project=${plan.subscribeProjectId ?? ''} reason=$reason',
      );
      final sent = _send(subscribe);
      if (sent) {
        _subscriptions.markProjectSubscribed(
          commit.projectId,
          baselineRequested: commit.baseline,
        );
        baselineRequested = commit.baseline;
      } else if (commit.baseline && activeBelongsToProject) {
        _outputController.resetSessionTransient(activeSessionId);
      }
    }
    return baselineRequested;
  }

  bool subscribeSessionBaseline({
    required String sessionId,
    required String reason,
    required TerminalBufferCapability capability,
    bool baseline = true,
    bool replaceActive = false,
  }) {
    final cleanSessionId = sessionId.trim();
    if (cleanSessionId.isEmpty) return false;
    final requestId = _nextRequestId('session-$cleanSessionId');
    final maxChars = capability.maxChars.clamp(1, _maxCharsLimit);
    final viewportSize = _viewportSize?.call(cleanSessionId);
    final envelope = remoteResourceSubscribeEnvelope(
      resource: RemoteResourceType.terminals,
      sessionId: cleanSessionId,
      baseline: baseline,
      maxChars: maxChars,
      chunkChars: capability.chunking ? capability.chunkChars : null,
      requestId: requestId,
      viewportCols: viewportSize?.cols,
      viewportRows: viewportSize?.rows,
    );
    if (baseline) {
      final started = _outputController.startBufferRequest(
        cleanSessionId,
        requestId,
        requireBaseline: true,
        resetAssembler: true,
        replaceActive: replaceActive,
      );
      if (!started) return false;
    }
    WeCodeLog.info(
      '[wecode-flutter-terminal] subscribe session=$cleanSessionId reason=$reason baseline=$baseline',
    );
    final sent = _send(envelope);
    if (!sent && baseline) {
      _outputController.resetSessionTransient(cleanSessionId);
    }
    return sent && baseline;
  }

  void resubscribeVisibleTerminal({
    required bool transportConnected,
    required bool protocolReady,
    required String? activeSessionId,
    required String? selectedProjectId,
    required TerminalBufferCapability capability,
    required String reason,
    required void Function(String sessionId, bool baselineRequested)
    ensureBoundBaseline,
  }) {
    if (!transportConnected || !protocolReady) return;
    final sessionId = activeSessionId;
    if (sessionId != null && sessionId.isNotEmpty) {
      // Cache is only an instant paint source. Whenever the visible terminal
      // is rebound after foreground/reconnect/path changes, refresh the host
      // baseline so scrollback and native replay are authoritative even if
      // the desktop window has not repainted.
      final requested = subscribeSessionBaseline(
        sessionId: sessionId,
        reason: reason,
        capability: capability,
        baseline: true,
      );
      ensureBoundBaseline(sessionId, requested);
      return;
    }
    final projectId = selectedProjectId;
    if (projectId == null || projectId.isEmpty) return;
    _subscriptions.markProjectBaselineStale(projectId);
    replaceProjectSubscription(
      projectId: projectId,
      reason: reason,
      capability: capability,
      activeSessionId: activeSessionId,
    );
  }

  void ensureBoundTerminalHasBaseline({
    required String sessionId,
    required bool baselineRequested,
    required String reason,
    required TerminalBufferCapability capability,
  }) {
    if (baselineRequested || _outputController.hasCachedOutput(sessionId)) {
      WeCodeLog.debug(
        '[wecode-flutter-terminal] baseline satisfied session=$sessionId reason=$reason requested=$baselineRequested',
      );
      return;
    }
    final terminal = _terminalById(sessionId);
    if (terminal == null) return;
    final projectId = terminal.projectId;
    _subscriptions.markProjectBaselineStale(projectId);
    replaceProjectSubscription(
      projectId: projectId,
      reason: 'empty-pool-$reason',
      capability: capability,
      activeSessionId: sessionId,
    );
  }

  RemoteTerminalBindResult bindSession({
    required RemoteRuntimePlan plan,
    required String bindSessionId,
    required String reason,
    required String? selectedProjectId,
    required TerminalBufferCapability capability,
    required bool restored,
  }) {
    var baselineRequested = false;
    final hasCachedOutput =
        _outputController.hasCachedOutput(bindSessionId) &&
        !_outputController.hasSequenceGap(bindSessionId) &&
        !_baselineStaleSessionIds.contains(bindSessionId);
    // A gap-free cached session switched back to must NOT reload its baseline:
    // replaying the trimmed raw history rebuilds the screen from a truncated,
    // mid-escape byte window, which for a repainting TUI (codex/claude in the
    // normal buffer) paints residue, black rows, and stray escape fragments
    // (e.g. `5;67;78m`) into scrollback. The viewport re-claim/resize on rebind
    // already pushes a fresh host keyframe, so a gap-free switch stays current
    // without the reload. Only reload when there's no usable cache, a real
    // sequence gap, or the plan explicitly asks for a full buffer.
    final needsFullBuffer = !hasCachedOutput || plan.bindFullBuffer;
    if (selectedProjectId != null) {
      baselineRequested = replaceProjectSubscription(
        projectId: selectedProjectId,
        reason: 'bind-$reason',
        capability: capability,
        activeSessionId: bindSessionId,
        baseline: needsFullBuffer,
      );
    }
    if (needsFullBuffer && !baselineRequested) {
      _outputController.bindSession(bindSessionId, requireBaseline: true);
      baselineRequested = subscribeSessionBaseline(
        sessionId: bindSessionId,
        reason: 'bind-$reason',
        capability: capability,
      );
    }
    if (baselineRequested) {
      _baselineStaleSessionIds.remove(bindSessionId);
    }
    return RemoteTerminalBindResult(
      baselineRequested: baselineRequested,
      restored: restored,
    );
  }
}
