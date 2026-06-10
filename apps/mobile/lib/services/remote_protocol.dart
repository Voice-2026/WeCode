import '../models/remote_models.dart';
import 'package:codux_protocol_ffi/codux_protocol_ffi.dart'
    as codux_protocol_ffi;

const String remoteProtocolVersion = 'v3.1';

abstract final class RemoteResourceType {
  static const projects = 'projects';
  static const terminals = 'terminals';
  static const worktrees = 'worktrees';
  static const gitStatus = 'git.status';
  static const aiStats = 'ai.stats';
  static const files = 'files';
}

abstract final class RemoteMessageType {
  static const hello = 'hello';
  static const error = 'error';
  static const secureMessage = 'secure.message';
  static const hostInfo = 'host.info';
  static const hostOffline = 'host.offline';
  static const secureRequired = 'secure.required';
  static const deviceInfo = 'device.info';
  static const deviceDisconnected = 'device.disconnected';
  static const transportPing = 'transport.ping';
  static const transportPong = 'transport.pong';
  static const resourceSubscribe = 'resource.subscribe';
  static const resourceUnsubscribe = 'resource.unsubscribe';
  static const projectList = 'project.list';
  static const projectSelect = 'project.select';
  static const projectSelected = 'project.selected';
  static const projectAdd = 'project.add';
  static const projectEdit = 'project.edit';
  static const projectRemove = 'project.remove';
  static const projectUpdated = 'project.updated';
  static const worktreeList = 'worktree.list';
  static const worktreeSelect = 'worktree.select';
  static const worktreeCreate = 'worktree.create';
  static const worktreeMerge = 'worktree.merge';
  static const worktreeDelete = 'worktree.delete';
  static const worktreeUpdated = 'worktree.updated';
  static const terminalList = 'terminal.list';
  static const terminalSubscribe = 'terminal.subscribe';
  static const terminalUnsubscribe = 'terminal.unsubscribe';
  static const terminalCreate = 'terminal.create';
  static const terminalCreated = 'terminal.created';
  static const terminalClose = 'terminal.close';
  static const terminalClosed = 'terminal.closed';
  static const terminalBuffer = 'terminal.buffer';
  static const terminalOutput = 'terminal.output';
  static const terminalOutputAck = 'terminal.output.ack';
  static const terminalInput = 'terminal.input';
  static const terminalInputAck = 'terminal.input.ack';
  static const terminalViewportClaim = 'terminal.viewport.claim';
  static const terminalViewportResize = 'terminal.viewport.resize';
  static const terminalViewportRelease = 'terminal.viewport.release';
  static const terminalViewportState = 'terminal.viewport.state';
  static const terminalUploadStart = 'terminal.upload.start';
  static const terminalUploadChunk = 'terminal.upload.chunk';
  static const terminalUploadFinish = 'terminal.upload.finish';
  static const terminalUploadAck = 'terminal.upload.ack';
  static const terminalUploaded = 'terminal.uploaded';
  static const fileList = 'file.list';
  static const fileRead = 'file.read';
  static const fileWrite = 'file.write';
  static const fileWritten = 'file.written';
  static const fileRename = 'file.rename';
  static const fileRenamed = 'file.renamed';
  static const fileDelete = 'file.delete';
  static const fileDeleted = 'file.deleted';
  static const gitStatus = 'git.status';
  static const aiStats = 'ai.stats';
}

RelayEnvelope remoteResourceSubscribeEnvelope({
  required String resource,
  String? projectId,
  String? sessionId,
  bool baseline = true,
  int? maxChars,
  int? chunkChars,
  String? requestId,
}) {
  final envelope = RelayEnvelope.fromJson(
    codux_protocol_ffi.resourceSubscribeEnvelope(
      resource: resource,
      projectId: projectId,
      sessionId: sessionId,
      baseline: baseline,
      maxChars: maxChars,
      chunkChars: chunkChars,
    ),
  );
  final cleanRequestId = requestId?.trim();
  if (cleanRequestId != null && cleanRequestId.isNotEmpty) {
    final payload = envelope.payload;
    if (payload is Map) {
      payload['requestId'] = cleanRequestId;
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
    codux_protocol_ffi.resourceUnsubscribeEnvelope(
      resource: resource,
      projectId: projectId,
      sessionId: sessionId,
    ),
  );
}

bool remoteRelayBlocksMessage(String kind) {
  return codux_protocol_ffi.relayBlocksMessage(kind);
}

String remoteTransportServerUrl(String base) {
  return codux_protocol_ffi.transportServerUrl(base);
}

String remoteTransportPairingTicketUrl({
  required String base,
  required String ticket,
}) {
  return codux_protocol_ffi.transportPairingTicketUrl(
    base: base,
    ticket: ticket,
  );
}

String remoteTransportPairingWebSocketUrl({
  required String base,
  required String hostId,
  required String devicePublicKey,
}) {
  return codux_protocol_ffi.transportPairingWebSocketUrl(
    base: base,
    hostId: hostId,
    devicePublicKey: devicePublicKey,
  );
}

String remoteTransportClientWebSocketUrl({
  required String base,
  required String hostId,
  required String deviceId,
  String token = '',
}) {
  return codux_protocol_ffi.transportClientWebSocketUrl(
    base: base,
    hostId: hostId,
    deviceId: deviceId,
    token: token,
  );
}

List<Map<String, dynamic>> remoteTransportDefaultIceServers() {
  return codux_protocol_ffi.transportDefaultIceServers();
}

String remotePreferredTransportKind(
  List<RemoteTransportCandidate> transports, {
  required bool pairing,
}) {
  return codux_protocol_ffi.preferredTransportKind(
    transports.map((item) => item.toJson()).toList(),
    pairing: pairing,
  );
}
