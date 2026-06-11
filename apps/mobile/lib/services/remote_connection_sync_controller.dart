import 'remote_sync_state.dart';

class RemoteInitialSyncPlan {
  const RemoteInitialSyncPlan({
    required this.sendDeviceInfo,
    required this.requestProjectList,
    required this.requestTerminalList,
    required this.resetTerminalBufferRetry,
  });

  final bool sendDeviceInfo;
  final bool requestProjectList;
  final bool requestTerminalList;
  final bool resetTerminalBufferRetry;

  bool get hasWork =>
      sendDeviceInfo || requestProjectList || requestTerminalList;
}

class RemoteConnectionSyncController {
  RemoteConnectionSyncController({RemoteSyncState? syncState})
    : syncState = syncState ?? RemoteSyncState();

  final RemoteSyncState syncState;

  int generation = 0;
  bool protocolReady = false;
  int _deviceInfoSentGeneration = 0;
  int _hostInfoSentGeneration = 0;
  int _forcedProtocolReadyGeneration = 0;

  int beginConnectionGeneration() {
    generation += 1;
    protocolReady = false;
    _deviceInfoSentGeneration = 0;
    _hostInfoSentGeneration = 0;
    _forcedProtocolReadyGeneration = 0;
    syncState.beginConnectionGeneration();
    return generation;
  }

  void resetProtocolReady() {
    protocolReady = false;
  }

  void resetSyncForCurrentGeneration() {
    _forcedProtocolReadyGeneration = 0;
    syncState.beginConnectionGeneration();
  }

  bool markProtocolReady({bool force = false}) {
    if (force) {
      if (_forcedProtocolReadyGeneration == generation) return false;
      _forcedProtocolReadyGeneration = generation;
    }
    if (protocolReady && !force) return false;
    protocolReady = true;
    return true;
  }

  bool shouldSendHostInfo({
    required bool transportReady,
    required bool transportConnected,
    bool force = false,
  }) {
    if (!transportConnected) return false;
    if (!force && _hostInfoSentGeneration == generation) return false;
    return true;
  }

  void markHostInfoSent() {
    _hostInfoSentGeneration = generation;
  }

  bool shouldSendDeviceInfo({bool force = false}) {
    if (!protocolReady) return false;
    if (!force && _deviceInfoSentGeneration == generation) return false;
    return true;
  }

  void markDeviceInfoSent() {
    _deviceInfoSentGeneration = generation;
  }

  RemoteInitialSyncPlan initialSyncPlan({
    required bool transportReady,
    required bool transportConnected,
    bool force = false,
  }) {
    if (!transportReady || !transportConnected || !protocolReady) {
      return const RemoteInitialSyncPlan(
        sendDeviceInfo: false,
        requestProjectList: false,
        requestTerminalList: false,
        resetTerminalBufferRetry: false,
      );
    }
    return RemoteInitialSyncPlan(
      sendDeviceInfo: shouldSendDeviceInfo(force: force),
      requestProjectList: syncState.shouldRequestProjectList(force: force),
      requestTerminalList: syncState.shouldRequestTerminalList(force: force),
      resetTerminalBufferRetry: force,
    );
  }
}
