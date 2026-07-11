import '../models/remote_models.dart';
import 'package:wecode_protocol_ffi/wecode_protocol_ffi.dart'
    as wecode_protocol_ffi;

final String remoteProtocolVersion = wecode_protocol_ffi.protocolVersion();

abstract final class RemoteResourceType {
  static final projects = wecode_protocol_ffi.resourceType('projects');
  static final terminals = wecode_protocol_ffi.resourceType('terminals');
  static final worktrees = wecode_protocol_ffi.resourceType('worktrees');
  static final gitStatus = wecode_protocol_ffi.resourceType('gitStatus');
  static final aiStats = wecode_protocol_ffi.resourceType('aiStats');
  static final files = wecode_protocol_ffi.resourceType('files');
}

abstract final class RemoteMessageType {
  static final hello = wecode_protocol_ffi.messageType('hello');
  static final error = wecode_protocol_ffi.messageType('error');
  static final hostInfo = wecode_protocol_ffi.messageType('hostInfo');
  static final hostOffline = wecode_protocol_ffi.messageType('hostOffline');
  static final deviceInfo = wecode_protocol_ffi.messageType('deviceInfo');
  static final deviceDisconnected = wecode_protocol_ffi.messageType(
    'deviceDisconnected',
  );
  static final pairingRequest = wecode_protocol_ffi.messageType(
    'pairingRequest',
  );
  static final pairingConfirmed = wecode_protocol_ffi.messageType(
    'pairingConfirmed',
  );
  static final pairingRejected = wecode_protocol_ffi.messageType(
    'pairingRejected',
  );
  static final transportPing = wecode_protocol_ffi.messageType('transportPing');
  static final transportPong = wecode_protocol_ffi.messageType('transportPong');
  static final resourceSubscribe = wecode_protocol_ffi.messageType(
    'resourceSubscribe',
  );
  static final resourceUnsubscribe = wecode_protocol_ffi.messageType(
    'resourceUnsubscribe',
  );
  static final projectList = wecode_protocol_ffi.messageType('projectList');
  static final projectSelect = wecode_protocol_ffi.messageType('projectSelect');
  static final projectSelected = wecode_protocol_ffi.messageType(
    'projectSelected',
  );
  static final projectAdd = wecode_protocol_ffi.messageType('projectAdd');
  static final projectEdit = wecode_protocol_ffi.messageType('projectEdit');
  static final projectRemove = wecode_protocol_ffi.messageType('projectRemove');
  static final projectUpdated = wecode_protocol_ffi.messageType(
    'projectUpdated',
  );
  static final worktreeList = wecode_protocol_ffi.messageType('worktreeList');
  static final worktreeSelect = wecode_protocol_ffi.messageType(
    'worktreeSelect',
  );
  static final worktreeCreate = wecode_protocol_ffi.messageType(
    'worktreeCreate',
  );
  static final worktreeMerge = wecode_protocol_ffi.messageType('worktreeMerge');
  static final worktreeDelete = wecode_protocol_ffi.messageType(
    'worktreeDelete',
  );
  static final worktreeUpdated = wecode_protocol_ffi.messageType(
    'worktreeUpdated',
  );
  static final terminalList = wecode_protocol_ffi.messageType('terminalList');
  static final terminalSubscribe = wecode_protocol_ffi.messageType(
    'terminalSubscribe',
  );
  static final terminalUnsubscribe = wecode_protocol_ffi.messageType(
    'terminalUnsubscribe',
  );
  static final terminalCreate = wecode_protocol_ffi.messageType(
    'terminalCreate',
  );
  static final terminalCreated = wecode_protocol_ffi.messageType(
    'terminalCreated',
  );
  static final terminalClose = wecode_protocol_ffi.messageType('terminalClose');
  static final terminalClosed = wecode_protocol_ffi.messageType(
    'terminalClosed',
  );
  static final terminalBuffer = wecode_protocol_ffi.messageType(
    'terminalBuffer',
  );
  static final terminalOutput = wecode_protocol_ffi.messageType(
    'terminalOutput',
  );
  static final terminalOutputAck = wecode_protocol_ffi.messageType(
    'terminalOutputAck',
  );
  static final terminalInput = wecode_protocol_ffi.messageType('terminalInput');
  static final terminalInputAck = wecode_protocol_ffi.messageType(
    'terminalInputAck',
  );
  static final terminalSignal = wecode_protocol_ffi.messageType(
    'terminalSignal',
  );
  static final terminalViewportClaim = wecode_protocol_ffi.messageType(
    'terminalViewportClaim',
  );
  static final terminalViewportResize = wecode_protocol_ffi.messageType(
    'terminalViewportResize',
  );
  static final terminalViewportRelease = wecode_protocol_ffi.messageType(
    'terminalViewportRelease',
  );
  static final terminalViewportState = wecode_protocol_ffi.messageType(
    'terminalViewportState',
  );
  static final terminalUploaded = wecode_protocol_ffi.messageType(
    'terminalUploaded',
  );
  static final fileList = wecode_protocol_ffi.messageType('fileList');
  static final fileRead = wecode_protocol_ffi.messageType('fileRead');
  static final fileWrite = wecode_protocol_ffi.messageType('fileWrite');
  static final fileWritten = wecode_protocol_ffi.messageType('fileWritten');
  static final fileRename = wecode_protocol_ffi.messageType('fileRename');
  static final fileRenamed = wecode_protocol_ffi.messageType('fileRenamed');
  static final fileDelete = wecode_protocol_ffi.messageType('fileDelete');
  static final fileDeleted = wecode_protocol_ffi.messageType('fileDeleted');
  static final gitStatus = wecode_protocol_ffi.messageType('gitStatus');
  static final gitInvoke = wecode_protocol_ffi.messageType('gitInvoke');
  static final gitRead = wecode_protocol_ffi.messageType('gitRead');
  static final aiStats = wecode_protocol_ffi.messageType('aiStats');
  static final aiSession = wecode_protocol_ffi.messageType('aiSession');
  static final aiSessionResult = wecode_protocol_ffi.messageType(
    'aiSessionResult',
  );
  static final sshList = wecode_protocol_ffi.messageType('sshList');
  static final sshListResult = wecode_protocol_ffi.messageType('sshListResult');
  static final sshUpsert = wecode_protocol_ffi.messageType('sshUpsert');
  static final sshRemove = wecode_protocol_ffi.messageType('sshRemove');
}

RelayEnvelope remoteResourceSubscribeEnvelope({
  required String resource,
  String? projectId,
  String? sessionId,
  bool baseline = true,
  int? maxChars,
  int? chunkChars,
  String? requestId,
  String? baselineSessionId,
  int? viewportCols,
  int? viewportRows,
}) {
  final envelope = RelayEnvelope.fromJson(
    wecode_protocol_ffi.resourceSubscribeEnvelope(
      resource: resource,
      projectId: projectId,
      sessionId: sessionId,
      baseline: baseline,
      maxChars: maxChars,
      chunkChars: chunkChars,
    ),
  );
  final payload = envelope.payload;
  final cleanRequestId = requestId?.trim();
  if (payload is Map) {
    if (cleanRequestId != null && cleanRequestId.isNotEmpty) {
      payload['requestId'] = cleanRequestId;
    }
    final cleanBaselineSessionId = baselineSessionId?.trim();
    if (cleanBaselineSessionId != null && cleanBaselineSessionId.isNotEmpty) {
      payload['baselineSessionId'] = cleanBaselineSessionId;
    }
    if (viewportCols != null && viewportCols > 0) {
      payload['viewportCols'] = viewportCols;
    }
    if (viewportRows != null && viewportRows > 0) {
      payload['viewportRows'] = viewportRows;
    }
  }
  return envelope;
}

RelayEnvelope remoteResourceUnsubscribeEnvelope({
  required String resource,
  String? projectId,
  String? sessionId,
}) {
  return RelayEnvelope.fromJson(
    wecode_protocol_ffi.resourceUnsubscribeEnvelope(
      resource: resource,
      projectId: projectId,
      sessionId: sessionId,
    ),
  );
}

bool remoteRelayBlocksMessage(String kind) {
  return wecode_protocol_ffi.relayBlocksMessage(kind);
}

bool remoteIsTerminalStreamMessage(String kind) {
  return wecode_protocol_ffi.isTerminalStreamMessage(kind);
}

String remoteTransportRelayUrl(String base) {
  return wecode_protocol_ffi.transportRelayUrl(base);
}

String remoteTransportRelayUrlForPreset({
  required String preset,
  String customUrl = '',
}) {
  return wecode_protocol_ffi.transportRelayUrlForPreset(
    preset: preset,
    customUrl: customUrl,
  );
}

List<Map<String, dynamic>> remoteTransportRelayPresets() {
  return wecode_protocol_ffi.transportRelayPresets();
}

String remotePreferredTransportKind(
  List<RemoteTransportCandidate> transports, {
  required bool pairing,
}) {
  return wecode_protocol_ffi.preferredTransportKind(
    transports.map((item) => item.toJson()).toList(),
    pairing: pairing,
  );
}

/// Validate a decoded pairing-payload object through the SHARED Rust parser, so
/// the client uses the same format definition as the hosts (no Dart re-impl).
Map<String, dynamic> remoteParsePairingPayload(Map<String, dynamic> payload) {
  return wecode_protocol_ffi.parsePairingPayload(payload);
}
