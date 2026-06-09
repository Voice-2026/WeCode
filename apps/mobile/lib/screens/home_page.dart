import 'dart:async';
import 'dart:io';

import 'package:device_info_plus/device_info_plus.dart';
import 'package:file_picker/file_picker.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:package_info_plus/package_info_plus.dart';
import 'package:url_launcher/url_launcher.dart';
import 'package:codux_native_terminal/codux_native_terminal.dart';
import '../i18n.dart';
import '../models/remote_models.dart';
import '../screens/settings_screen.dart';
import '../services/e2e_crypto.dart';
import '../services/log_service.dart';
import '../services/log_export_service.dart';
import '../services/local_voice_recognition_service.dart';
import '../services/mobile_settings_controller.dart';
import '../services/connection_status_presenter.dart';
import '../services/device_selection_service.dart';
import '../services/remote_device_controller.dart';
import '../services/remote_envelope_send_queue.dart';
import '../services/remote_capabilities.dart';
import '../services/remote_connection_sync_controller.dart';
import '../services/remote_project_controller.dart';
import '../services/remote_protocol_service.dart';
import '../services/remote_runtime_payloads.dart';
import '../services/remote_runtime_store.dart';
import '../services/remote_sequence_guard.dart';
import '../services/remote_sync_state.dart';
import '../services/remote_project_file_controller.dart';
import '../services/remote_terminal_output_controller.dart';
import '../services/remote_terminal_scope.dart';
import '../services/remote_terminal_subscription_controller.dart';
import '../services/remote_transport.dart';
import '../services/remote_transport_state_controller.dart';
import '../services/storage_service.dart';
import '../services/terminal_buffer_request.dart';
import '../services/terminal_buffer_retry.dart';
import '../services/terminal_input_batcher.dart';
import '../services/terminal_input_payload.dart';
import '../services/terminal_input_reliable_sender.dart';
import '../services/terminal_upload_metadata.dart';
import '../services/terminal_upload_sender.dart';
import '../services/update_check_service.dart';
import '../services/remote_terminal_renderer.dart';
import '../services/terminal_viewport_controller.dart';
import '../theme/app_theme.dart';
import '../services/worktree_utils.dart';
import '../widgets/codux_home_shell.dart';
import '../widgets/device_home_screen.dart';
import '../widgets/project_files_panel.dart';
import '../widgets/remote_terminal_pane.dart';
import '../widgets/remote_workspace_view.dart';
import '../widgets/terminal_switcher_screen.dart';
import '../widgets/worktree_create_dialog.dart';
import '../widgets/worktree_action_dialog.dart';
import '../widgets/terminal_upload_source_sheet.dart';
import '../widgets/update_available_dialog.dart';
import '../widgets/codux_about_dialog.dart';
import '../widgets/debug_log_dialog.dart';
import '../widgets/device_action_dialogs.dart';
import '../widgets/file_action_dialogs.dart';

const String _remoteProtocolVersion = remoteProtocolVersion;
const Duration _remoteStartupProbeTimeout = Duration(seconds: 15);
const Duration _remotePingInterval = Duration(seconds: 10);
const Duration _remotePingTimeout = Duration(seconds: 12);
const int _remoteMaxPingMisses = 3;

class CoduxHomePage extends StatefulWidget {
  const CoduxHomePage({
    super.key,
    required this.onChangeAccent,
    required this.onChangeLocale,
    this.initialDevices,
    this.transportFactory,
  });

  final ValueChanged<AccentOption> onChangeAccent;
  final ValueChanged<LocaleOption> onChangeLocale;
  final List<StoredDevice>? initialDevices;
  final RemoteTransportFactory? transportFactory;

  @override
  State<CoduxHomePage> createState() => _CoduxHomePageState();
}

class _CoduxHomePageState extends State<CoduxHomePage>
    with TickerProviderStateMixin, WidgetsBindingObserver {
  static const int _terminalBufferMaxChars =
      TerminalBufferCapability.mobileMaxChars;

  final _storage = StorageService();
  final _deviceSelection = const DeviceSelectionService();
  final _connectionStatusPresenter = const ConnectionStatusPresenter();
  final _updateCheckService = const UpdateCheckService();
  final _logExportService = const LogExportService();
  final _mobileSettingsController = const MobileSettingsController();
  final _deviceController = const RemoteDeviceController();
  final _sendQueue = RemoteEnvelopeSendQueue();
  final _projectController = const RemoteProjectController();
  final _projectFileController = RemoteProjectFileController();
  final _worktreeController = const RemoteWorktreeController();
  final _remoteSyncController = RemoteConnectionSyncController();
  final _transportStateController = RemoteTransportStateController();
  final _remoteRuntime = RemoteRuntimeStore();
  final _terminalSubscriptions = RemoteTerminalSubscriptionController();
  final _settingsNameController = TextEditingController();
  final _fileEditorController = CodeEditingController();
  final _projectNameController = TextEditingController();
  final _projectPathController = TextEditingController();

  late final AnimationController _maskController;
  late final Animation<double> _maskOpacity;
  late final AnimationController _edgeBackController;
  late final TerminalBufferRetryCoordinator _terminalBufferRetry;
  late final TerminalInputBatcher _terminalInputBatcher;
  late final TerminalInputReliableSender _terminalInputSender;
  late final TerminalUploadSender _terminalUploadSender;
  late final LocalVoiceRecognitionService _voiceService;
  RemoteTransport? _activeTransport;
  Completer<void>? _terminalUploadCompletion;
  CoduxNativeTerminalController? _nativeTerminalController;
  NativeTerminalPort? _nativeTerminalPort;
  late final RemoteTerminalRenderer _terminalRenderer;
  final TerminalViewportController _terminalViewportController =
      TerminalViewportController();
  final RemoteTerminalOutputController _terminalOutputController =
      RemoteTerminalOutputController(maxBufferChars: _terminalBufferMaxChars);
  final Set<String> _protocolBlockedHostIds = {};
  int _terminalBufferRequestCounter = 0;
  double _terminalCursorBottom = 0;
  bool _keyboardVisible = false;

  List<StoredDevice> _devices = [];
  List<ProjectInfo> _projects = [];
  List<TerminalInfo> _terminals = [];
  List<RemoteWorktreeInfo> _worktrees = [];
  List<String> _worktreeBaseBranches = [];
  StoredDevice? _activeDevice;
  TerminalBufferCapability _terminalBufferCapability =
      TerminalBufferCapability.fallback;
  String? _hostRuntimeInstanceId;
  MobileSettings _settings = const MobileSettings(localName: '');
  String _detectedDeviceName = 'Codux Mobile';
  String _status = '';
  String? _selectedProjectId;
  String? _sessionId;
  String? _creatingTerminalProjectId;
  String? _defaultWorktreeBaseBranch;
  bool _showSettings = false;
  bool _showScanner = false;
  PairingPayload? _pendingPairing;
  bool _pairingInFlight = false;
  bool _pairingCancelled = false;
  String? _pairingError;
  bool _showTerminal = false;
  bool _showTerminalSwitcher = false;
  bool _terminalReady = false;
  RemoteTerminalBufferPhase _terminalBufferPhase =
      RemoteTerminalBufferPhase.idle;
  double? _terminalBufferProgress;
  bool _terminalUploadLoading = false;
  String _terminalUploadStatus = '';
  RemoteSyncState get _remoteSync => _remoteSyncController.syncState;
  bool get _terminalListLoaded => _remoteSync.terminalListLoaded;
  bool get _projectListLoaded => _remoteSync.projectListLoaded;
  bool _backgroundConnect = false;
  bool _shouldReconnect = true;
  bool _transportReady = false;
  bool get _remoteProtocolReady => _remoteSyncController.protocolReady;
  bool _hostResponsive = false;
  bool _appSuspended = false;
  bool _disposing = false;
  bool _hasShownTerminal = false;
  bool _aiStatsLoading = false;
  bool _showProjectForm = false;
  bool _showFilePicker = false;
  bool _showVoiceOverlay = false;
  bool _filePickerLoading = false;
  ProjectFormMode _projectFormMode = ProjectFormMode.add;
  String _filePickerMode = 'projectForm';
  String _filePickerPath = '';
  String? _filePickerParent;
  List<RemoteFileEntry> _filePickerEntries = [];
  List<RemoteFileEntry> _projectFileEntries = [];
  AIStatsInfo? _currentAIStats;
  String _workspaceMode = 'terminal';
  String _projectFilesPath = '';
  String? _projectFilesParent;
  String? _editingFilePath;
  String? _toastMessage;
  String? _blockingLoadingMessage;
  bool _projectFilesLoading = false;
  bool _worktreeListLoading = false;
  bool _fileEditorLoading = false;
  bool _fileEditorSaving = false;
  bool _fileEditorEditing = false;
  bool _fileEditorEditable = true;
  int _reconnectAttempt = 0;
  bool _appInForeground = true;

  bool _transportConnected = false;
  int get _transportGeneration => _remoteSyncController.generation;
  int _remoteRuntimeEpoch = 0;
  final _receiveSequenceGuard = RemoteSequenceGuard();
  Future<void> _receiveChain = Future<void>.value();
  Timer? _reconnectTimer;
  Timer? _healthTimer;
  Timer? _toastTimer;
  Timer? _filePickerTimeoutTimer;
  Timer? _projectListRetryTimer;
  Timer? _terminalListRetryTimer;
  Timer? _hostResponseTimer;
  Timer? _latencyProbeTimer;
  Timer? _pingTimeoutTimer;
  Timer? _transportCloseTimer;
  int get _projectListRetryAttempt => _remoteSync.projectListRetryAttempt;
  int get _terminalListRetryAttempt => _remoteSync.terminalListRetryAttempt;
  double? _edgeBackDragStartX;
  double _edgeBackDragDeltaX = 0;
  double _edgeBackDragDeltaY = 0;
  String _lastTransportState = RemoteTransportKind.websocketRelay;
  String _connectionPath = 'unknown';
  DateTime? _lastConnectedAt;
  DateTime? _connectionGraceUntil;
  String? _selectedWorktreeId;
  int? _latencyMs;
  Timer? _connectionGraceTimer;

  bool get _isConnected => _transportConnected && _transportReady;
  bool get _isHostReady =>
      _isConnected &&
      _hostResponsive &&
      _connectionPath != 'unknown' &&
      _connectionPath != 'none';
  bool get _isRecoveringConnection {
    final graceUntil = _connectionGraceUntil;
    return _appInForeground &&
        _activeDevice != null &&
        _shouldReconnect &&
        graceUntil != null &&
        DateTime.now().isBefore(graceUntil);
  }

  bool get _isDeviceListConnected => _isHostReady;

  String _t(String key, {Map<String, String>? params}) =>
      AppPreferences.of(context).t(key, params: params);

  ConnectionStatusSnapshot get _connectionStatusSnapshot =>
      ConnectionStatusSnapshot(
        connected: _isConnected,
        hostResponsive: _hostResponsive,
        connectionPath: _connectionPath,
        projectListLoaded: _projectListLoaded,
        hasProjects: _projects.isNotEmpty,
        recovering: _isRecoveringConnection,
        hasActiveDevice: _activeDevice != null,
        backgroundConnect: _backgroundConnect,
        status: _status,
        connectedText: _t('app.connected'),
      );

  String get _connectionStatusText {
    final key = _connectionStatusPresenter.connectionStatusKey(
      _connectionStatusSnapshot,
    );
    return key.isEmpty ? _status : _t(key);
  }

  String get _deviceListStatusText {
    final key = _connectionStatusPresenter.deviceListStatusKey(
      _connectionStatusSnapshot,
    );
    return key.isEmpty ? _status : _t(key);
  }

  void _clearConnectionGrace() {
    _connectionGraceTimer?.cancel();
    _connectionGraceTimer = null;
    _connectionGraceUntil = null;
  }

  void _startConnectionGrace({
    required String reason,
    Duration duration = const Duration(seconds: 8),
  }) {
    if (!_shouldReconnect || !_appInForeground) return;
    _connectionGraceTimer?.cancel();
    _connectionGraceUntil = DateTime.now().add(duration);
    CoduxLog.info(
      '[codux-flutter-remote] grace reason=$reason until=${_connectionGraceUntil!.toIso8601String()} transport=$_lastTransportState lastConnectedAt=${_lastConnectedAt?.toIso8601String() ?? 'null'}',
    );
    _connectionGraceTimer = Timer(duration, () {
      if (!mounted || _disposing) return;
      if (_connectionGraceUntil == null) return;
      if (DateTime.now().isBefore(_connectionGraceUntil!)) return;
      setState(() {
        _connectionGraceUntil = null;
      });
      CoduxLog.info('[codux-flutter-remote] grace expired reason=$reason');
    });
  }

  void _markTransportConnected(String transport) {
    _lastTransportState = transport;
    _lastConnectedAt = DateTime.now();
    _clearConnectionGrace();
  }

  void _cancelHostResponseProbe() {
    _hostResponseTimer?.cancel();
    _hostResponseTimer = null;
  }

  void _startHostResponseProbe({
    required String reason,
    Duration duration = _remoteStartupProbeTimeout,
  }) {
    final device = _activeDevice;
    final generation = _transportGeneration;
    if (!_transportReady || device == null) return;
    _cancelHostResponseProbe();
    CoduxLog.info(
      '[codux-flutter-remote] host probe start reason=$reason timeoutMs=${duration.inMilliseconds}',
    );
    _hostResponseTimer = Timer(duration, () {
      if (!mounted || _disposing || !_appInForeground) return;
      if (_transportGeneration != generation ||
          !_transportReady ||
          _hostResponsive) {
        return;
      }
      _failHostConnection(device, 'host_response_timeout:$reason');
    });
  }

  void _markTransportOpen({String? path}) {
    _reconnectAttempt = 0;
    setState(() {
      _transportConnected = true;
      _transportReady = true;
      _hasShownTerminal = true;
      if (path != null) _connectionPath = path;
      if (!_backgroundConnect) _status = _t('app.connected');
    });
    _markTransportConnected(
      _activeDevice?.transport ?? RemoteTransportKind.websocketRelay,
    );
  }

  void _markTransportPath(String path) {
    final connected = path != 'none';
    setState(() {
      _transportConnected = connected;
      _transportReady = connected;
      if (connected) {
        _hasShownTerminal = true;
        if (!_backgroundConnect) _status = _t('app.connected');
      }
      _connectionPath = path;
    });
    if (connected) {
      _markTransportConnected(
        _activeDevice?.transport ?? RemoteTransportKind.websocketRelay,
      );
    }
  }

  bool _isCompatibleRemoteProtocol(Object? payload) {
    if (payload is! Map) return false;
    return payload['protocolVersion'] == _remoteProtocolVersion;
  }

  void _markRemoteProtocolReady({bool force = false}) {
    if (!_remoteSyncController.markProtocolReady(force: force)) return;
    CoduxLog.info('[codux-flutter-remote] protocol ready force=$force');
    _sendInitialTransportRequests(force: force);
    _ensureTerminalForSelectedProject();
  }

  void _failRemoteProtocol(StoredDevice target, Object? payload) {
    final version = payload is Map ? '${payload['protocolVersion'] ?? ''}' : '';
    CoduxLog.warn(
      '[codux-flutter-remote] incompatible protocol expected=$_remoteProtocolVersion received=$version host=${target.hostId} device=${target.deviceId}',
    );
    _shouldReconnect = false;
    final shouldPrompt = _protocolBlockedHostIds.add(target.hostId);
    _reconnectTimer?.cancel();
    _reconnectTimer = null;
    _cancelHostResponseProbe();
    _clearConnectionGrace();
    _clearLatencyProbe();
    _transportConnected = false;
    unawaited(_closeActiveTransport());
    _terminalInputBatcher.reset();
    _terminalInputSender.clear();
    _terminalSubscriptions.reset();
    final message = _t('connection.upgradeRequired');
    setState(() {
      _transportReady = false;
      _remoteSyncController.resetProtocolReady();
      _hostResponsive = false;
      _backgroundConnect = false;
      _showTerminal = false;
      _workspaceMode = 'terminal';
      _worktrees = [];
      _worktreeBaseBranches = [];
      _resetRemoteSyncState();
      _defaultWorktreeBaseBranch = null;
      _selectedWorktreeId = null;
      _showTerminalSwitcher = false;
      _status = message;
      _terminalBufferRetry.reset();
      _terminalOutputController.resetTransient();
      _setTerminalBufferLoading(false);
    });
    if (shouldPrompt) {
      _showToast(message);
    }
  }

  void _markHostResponsive(String source, {String? transport}) {
    final wasResponsive = _hostResponsive;
    _hostResponsive = true;
    _transportStateController.markResponsive();
    _cancelHostResponseProbe();
    _markTransportConnected(
      transport ??
          _activeDevice?.transport ??
          RemoteTransportKind.websocketRelay,
    );
    if (!wasResponsive) {
      CoduxLog.info('[codux-flutter-remote] host responsive source=$source');
    }
  }

  void _clearLatencyProbe() {
    _latencyProbeTimer?.cancel();
    _latencyProbeTimer = null;
    _pingTimeoutTimer?.cancel();
    _pingTimeoutTimer = null;
    _transportStateController.clearLatency();
    _latencyMs = null;
  }

  void _pauseLatencyProbe() {
    _latencyProbeTimer?.cancel();
    _latencyProbeTimer = null;
    _pingTimeoutTimer?.cancel();
    _pingTimeoutTimer = null;
    _transportStateController.pauseLatency();
  }

  void _recordTransportPong(Object? payload) {
    final result = _transportStateController.recordPong(payload);
    if (!result.accepted) return;
    _pingTimeoutTimer?.cancel();
    _pingTimeoutTimer = null;
    final nextLatency = result.latencyMs;
    if (nextLatency == null) return;
    CoduxLog.info(
      '[codux-flutter-remote] latency rtt=${nextLatency}ms path=$_connectionPath',
    );
    if (_latencyMs == nextLatency) return;
    setState(() => _latencyMs = nextLatency);
  }

  void _sendHostInfoRequest({bool force = false}) {
    if (!_remoteSyncController.shouldSendHostInfo(
      transportReady: _transportReady,
      transportConnected: _transportConnected,
      force: force,
    )) {
      return;
    }
    CoduxLog.info('[codux-flutter-remote] request host.info');
    final sent = _send(const RelayEnvelope(type: 'host.info'));
    if (sent) _remoteSyncController.markHostInfoSent();
  }

  void _sendTransportPing() {
    final target = _activeDevice;
    final ping = _transportStateController.beginPing(
      transportReady: _transportReady,
      transportConnected: _transportConnected,
      hasDevice: target != null,
    );
    if (target == null || ping == null) return;
    final sent = _send(
      RelayEnvelope(type: 'transport.ping', payload: {'id': ping.id}),
    );
    if (!sent) {
      _transportStateController.cancelPendingPing();
      return;
    }
    _pingTimeoutTimer?.cancel();
    _pingTimeoutTimer = Timer(_remotePingTimeout, () {
      if (!mounted || _disposing || !_appInForeground) return;
      if (_transportStateController.latency.pendingPingId != ping.id) return;
      final missCount = _transportStateController.recordPingTimeoutMiss();
      CoduxLog.warn(
        '[codux-flutter-remote] latency timeout miss=$missCount path=$_connectionPath',
      );
      if (missCount >= _remoteMaxPingMisses) {
        setState(() => _latencyMs = null);
        _failHostConnection(target, 'transport_ping_timeout');
        return;
      }
      _sendHostInfoRequest();
    });
  }

  void _startLatencyProbe() {
    if (_latencyProbeTimer != null) return;
    _sendTransportPing();
    _latencyProbeTimer = Timer.periodic(
      _remotePingInterval,
      (_) => _sendTransportPing(),
    );
  }

  void _failHostConnection(StoredDevice target, String reason) {
    CoduxLog.warn(
      '[codux-flutter-remote] host unavailable reason=$reason host=${target.hostId} device=${target.deviceId}',
    );
    _disconnectTransport(
      status: _t('connection.failedRetry'),
      closeTerminal: false,
      notifyHost: false,
    );
    if (_appSuspended || !_appInForeground) {
      CoduxLog.info(
        '[codux-flutter-remote] reconnect deferred reason=$reason appSuspended=$_appSuspended',
      );
      return;
    }
    _scheduleReconnect(target);
  }

  void _resetRemoteSyncState() {
    _remoteRuntimeEpoch += 1;
    _cancelRemoteSyncTimers();
    _remoteSyncController.resetSyncForCurrentGeneration();
    _remoteRuntime.reset();
    _terminalSubscriptions.reset();
    _syncRuntimeViewState();
  }

  void _resetRemoteRuntime({bool keepProjects = false}) {
    _remoteRuntimeEpoch += 1;
    _remoteRuntime.reset(keepProjects: keepProjects);
    _terminalSubscriptions.reset();
    _syncRuntimeViewState();
  }

  void _resetRemoteRuntimeAfterHostRestart(String reason) {
    CoduxLog.info('[codux-flutter-remote] reset runtime reason=$reason');
    _remoteRuntimeEpoch += 1;
    _cancelRemoteSyncTimers();
    _remoteSyncController.resetSyncForCurrentGeneration();
    _remoteSyncController.resetProtocolReady();
    _terminalSubscriptions.reset();
    _terminalInputBatcher.reset();
    _terminalInputSender.clear();
    _terminalBufferRetry.reset();
    _terminalOutputController.resetAll();
    _receiveSequenceGuard.reset();
    _receiveChain = Future<void>.value();
    _hostResponsive = false;
    _remoteRuntime.reset(keepProjects: true);
    _syncRuntimeViewState();
    _terminalCursorBottom = 0;
    _setTerminalBufferLoading(false);
    _clearTerminal();
  }

  bool _recordHostRuntimeInstance(Object? payload) {
    if (payload is! Map) return false;
    final next = payload['runtimeInstanceId']?.toString().trim();
    if (next == null || next.isEmpty) return false;
    final previous = _hostRuntimeInstanceId;
    _hostRuntimeInstanceId = next;
    if (previous == null || previous == next) return false;
    _resetRemoteRuntimeAfterHostRestart(
      'host-runtime-instance-changed:$previous->$next',
    );
    return true;
  }

  void _cancelRemoteSyncTimers() {
    _projectListRetryTimer?.cancel();
    _projectListRetryTimer = null;
    _terminalListRetryTimer?.cancel();
    _terminalListRetryTimer = null;
  }

  void _disconnectTransport({
    required String status,
    bool closeTerminal = false,
    bool notifyHost = true,
  }) {
    if (notifyHost && _transportConnected) {
      _releaseTerminalViewport();
      _send(const RelayEnvelope(type: 'device.disconnected'));
    }
    _cancelHostResponseProbe();
    _clearConnectionGrace();
    _lastConnectedAt = null;
    _healthTimer?.cancel();
    _healthTimer = null;
    _clearLatencyProbe();
    _transportConnected = false;
    unawaited(_closeActiveTransport());
    _terminalInputBatcher.reset();
    _terminalInputSender.clear();
    _terminalOutputController.resetAll();
    _terminalSubscriptions.reset();
    setState(() {
      _transportReady = false;
      _remoteSyncController.resetProtocolReady();
      _hostResponsive = false;
      _backgroundConnect = false;
      if (closeTerminal) {
        _showTerminal = false;
        _workspaceMode = 'terminal';
      }
      _status = status;
      _terminalBufferRetry.reset();
      _setTerminalBufferLoading(false);
    });
    _clearTerminal();
  }

  void _recoverForegroundState() {
    if (!_transportReady) {
      final device = _activeDevice;
      if (device != null) _connect(device, true);
      return;
    }
    _backgroundConnect = false;
    _requestProjectList(resetRetry: true);
    _requestTerminalList(resetRetry: true);
    _sendHostInfoRequest();
    _restoreVisibleTerminalFromCache();
    _claimTerminalViewport();
    _flushPendingTerminalResize(force: true);
    _requestBufferForCurrentSession(force: true, preferFull: true);
    _terminalInputBatcher.flush();
    _startLatencyProbe();
  }

  void _restoreVisibleTerminalFromCache() {
    final id = _sessionId;
    if (id == null) return;
    _restoreTerminalSessionFromCache(id);
  }

  bool _restoreTerminalSessionFromCache(
    String sessionId, {
    bool clearFirst = true,
  }) {
    final cached = _terminalOutputController.cachedOutput(sessionId);
    return _terminalRenderer.restoreCached(
      cached ?? '',
      clearFirst: clearFirst,
    );
  }

  ProjectInfo? get _selectedProject {
    return _remoteRuntime.selectedProject();
  }

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addObserver(this);
    _maskController = AnimationController(
      vsync: this,
      duration: const Duration(milliseconds: 120),
    );
    _maskOpacity = CurvedAnimation(
      parent: _maskController,
      curve: Curves.easeOutCubic,
    );
    _edgeBackController = AnimationController(
      vsync: this,
      duration: const Duration(milliseconds: 220),
    );
    _terminalBufferRetry = TerminalBufferRetryCoordinator(
      onRetryExhausted: (sessionId) {
        if (!mounted || _sessionId != sessionId) return;
        _terminalOutputController.resetSessionTransient(
          sessionId,
          resetSequence: true,
        );
        _terminalBufferRetry.resetLastBuffered();
        setState(() => _setTerminalBufferLoading(false));
      },
    );
    _terminalInputBatcher = TerminalInputBatcher(
      send: (data) => _sendInputNow(data, source: 'typed-batch'),
    );
    _terminalInputSender = TerminalInputReliableSender(
      send: _sendTerminalEnvelope,
      activeSessionId: () => _sessionId,
    );
    _terminalRenderer = RemoteTerminalRenderer();
    _terminalUploadSender = TerminalUploadSender(
      send: _sendTerminalUploadEnvelopeReliable,
      afterChunkAck: () => Future<void>.delayed(Duration.zero),
    );
    _voiceService = LocalVoiceRecognitionService(
      onLog: (message) => CoduxLog.info('[codux-flutter-voice] $message'),
    );
    WidgetsBinding.instance.addPostFrameCallback((_) => _bootstrap());
  }

  @override
  void dispose() {
    WidgetsBinding.instance.removeObserver(this);
    _disposing = true;
    _shouldReconnect = false;
    _reconnectTimer?.cancel();
    _healthTimer?.cancel();
    _connectionGraceTimer?.cancel();
    _latencyProbeTimer?.cancel();
    _pingTimeoutTimer?.cancel();
    _transportCloseTimer?.cancel();
    _toastTimer?.cancel();
    _filePickerTimeoutTimer?.cancel();
    _projectListRetryTimer?.cancel();
    _terminalListRetryTimer?.cancel();
    _hostResponseTimer?.cancel();
    _terminalBufferRetry.dispose();
    _terminalInputBatcher.dispose();
    _terminalInputSender.dispose();
    _terminalUploadCompletion?.completeError(
      StateError('Terminal upload cancelled'),
    );
    _terminalUploadCompletion = null;
    _terminalUploadSender.dispose();
    _voiceService.dispose();
    unawaited(_closeActiveTransport());
    unawaited(_terminalRenderer.dispose());
    _nativeTerminalPort = null;
    _nativeTerminalController = null;
    _settingsNameController.dispose();
    _fileEditorController.dispose();
    _projectNameController.dispose();
    _projectPathController.dispose();
    _maskController.dispose();
    _edgeBackController.dispose();
    super.dispose();
  }

  @override
  void didChangeAppLifecycleState(AppLifecycleState state) {
    CoduxLog.info('[codux-flutter-lifecycle] state=${state.name}');
    if (state == AppLifecycleState.resumed) {
      _appInForeground = true;
      _appSuspended = false;
      final device = _activeDevice;
      if (device == null) return;
      if (_transportConnected) {
        CoduxLog.info(
          '[codux-flutter-lifecycle] resume keep existing transport host=${device.hostId} device=${device.deviceId}',
        );
        _recoverForegroundState();
        return;
      }
      CoduxLog.info(
        '[codux-flutter-lifecycle] resume reconnect host=${device.hostId} device=${device.deviceId}',
      );
      _connect(device, true);
      return;
    }
    if (state == AppLifecycleState.inactive) {
      return;
    }
    if (state == AppLifecycleState.paused && _showVoiceOverlay) {
      CoduxLog.info('[codux-flutter-lifecycle] pause ignored for voice input');
      return;
    }
    if (state == AppLifecycleState.detached) {
      _appInForeground = false;
      _appSuspended = true;
      _disconnectTransport(
        status: _t('app.disconnected'),
        closeTerminal: false,
      );
      if (mounted) {
        setState(() {
          _terminalBufferRetry.reset();
          _terminalOutputController.resetTransient();
          _setTerminalBufferLoading(false);
        });
      }
      return;
    }
    if (state == AppLifecycleState.paused ||
        state == AppLifecycleState.hidden) {
      _appInForeground = false;
      _appSuspended = true;
      _pauseLatencyProbe();
      CoduxLog.info(
        '[codux-flutter-lifecycle] background keep transport state=${state.name}',
      );
    }
  }

  Future<void> _bootstrap() async {
    final initialDevices = widget.initialDevices;
    if (initialDevices != null) {
      if (!mounted) return;
      setState(() {
        _devices = initialDevices;
        _activeDevice = initialDevices.isNotEmpty ? initialDevices.first : null;
        _showTerminal = false;
      });
      if (initialDevices.isNotEmpty) {
        unawaited(_restoreCachedProjects(initialDevices.first));
        _connect(initialDevices.first, true);
      }
      return;
    }
    await _loadDeviceName();
    final loadedSettings = await _storage.loadSettings();
    final devices = await _storage.loadDevices();
    final lastDeviceId = await _storage.loadLastDeviceId();
    if (!mounted) return;
    final next = _mobileSettingsController.startupSettings(
      stored: loadedSettings,
      detectedDeviceName: _detectedDeviceName,
    );
    final startupDevice = _deviceSelection.selectStartupDevice(
      devices,
      lastDeviceId,
    );
    CoduxLog.setLevelName(next.logLevel);
    widget.onChangeAccent(AccentChoices.byId(next.accentId));
    widget.onChangeLocale(LocaleChoices.byId(next.localeId));
    setState(() {
      _settings = next;
      _settingsNameController.text = next.localName;
      _devices = devices;
      _activeDevice = startupDevice.displayedDevice;
      _showTerminal = false;
    });
    final autoConnectDevice = startupDevice.autoConnectDevice;
    if (autoConnectDevice != null) {
      unawaited(_restoreCachedProjects(autoConnectDevice));
      _connect(autoConnectDevice, true);
    }
  }

  Future<void> _restoreCachedProjects(StoredDevice device) async {
    try {
      final cached = await _storage.loadCachedProjects(device);
      if (!mounted ||
          _activeDevice?.hostId != device.hostId ||
          cached.isEmpty ||
          _projects.isNotEmpty) {
        return;
      }
      _remoteRuntime.restoreCachedProjects(cached);
      setState(() {
        _syncRuntimeViewState();
      });
      CoduxLog.info(
        '[codux-flutter-projects] cache restored count=${cached.length} host=${device.hostId}',
      );
    } catch (error) {
      CoduxLog.warn('[codux-flutter-projects] cache restore failed: $error');
    }
  }

  Future<void> _loadDeviceName() async {
    try {
      final plugin = DeviceInfoPlugin();
      final info = await plugin.deviceInfo;
      _detectedDeviceName = _mobileSettingsController
          .detectedNameFromDeviceInfo(info.data);
    } catch (_) {
      _detectedDeviceName = MobileSettingsController.fallbackDeviceName;
    }
  }

  Future<void> _saveDevices(List<StoredDevice> devices) async {
    final nextState = _deviceController.preserveActive(
      devices: devices,
      activeDevice: _activeDevice,
    );
    setState(() {
      _devices = nextState.devices;
      _activeDevice = nextState.activeDevice;
    });
    await _storage.saveDevices(nextState.devices);
  }

  void _rememberActiveDevice(StoredDevice device) {
    unawaited(_storage.saveLastDeviceId(device.deviceId));
  }

  Future<void> _saveDevice(StoredDevice device) async {
    final nextState = _deviceController.upsertAndActivate(
      devices: _devices,
      device: device,
    );
    await _saveDevices(nextState.devices);
    setState(() {
      _activeDevice = nextState.activeDevice;
      _showTerminal = false;
      _status = _t('pair.success');
    });
    _connect(device);
  }

  void _handleScannedPayload(String raw) {
    if (!_showScanner || _pendingPairing != null) return;
    unawaited(_prepareScannedPayload(raw));
  }

  Future<void> _prepareScannedPayload(String raw) async {
    try {
      final payload = await parsePairingPayload(raw);
      if (!mounted || !_showScanner || _pendingPairing != null) return;
      setState(() {
        _showScanner = false;
        _pendingPairing = payload;
        _pairingInFlight = false;
        _pairingCancelled = false;
        _pairingError = null;
      });
    } catch (error) {
      if (!mounted) return;
      setState(() => _showScanner = false);
      _showToast(error.toString().replaceFirst('Exception: ', ''));
    }
  }

  void _cancelPairing() {
    if (_pairingInFlight) {
      setState(() => _pairingCancelled = true);
      return;
    }
    setState(() {
      _pendingPairing = null;
      _pairingInFlight = false;
      _pairingCancelled = false;
      _pairingError = null;
    });
  }

  Future<void> _confirmPairing() async {
    final payload = _pendingPairing;
    if (payload == null || _pairingInFlight) return;
    final name = _settings.localName.isNotEmpty
        ? _settings.localName
        : _detectedDeviceName;
    setState(() {
      _pairingInFlight = true;
      _pairingCancelled = false;
      _pairingError = null;
      _status = _t('pair.submitting');
    });
    try {
      final confirmed = await _confirmRelayPairing(payload, name);
      if (!mounted) return;
      final hostName = confirmed.hostName?.trim().isNotEmpty == true
          ? confirmed.hostName!.trim()
          : confirmed.name;
      setState(() {
        _pendingPairing = null;
        _pairingInFlight = false;
        _pairingCancelled = false;
        _pairingError = null;
      });
      await _saveDevice(confirmed);
      _showToast(_t('device.bound', params: {'name': hostName}));
    } on PairingCancelledException {
      if (!mounted) return;
      setState(() {
        _pendingPairing = null;
        _pairingInFlight = false;
        _pairingCancelled = false;
        _pairingError = null;
        _status = _t('pair.cancelled');
      });
    } on PairingRejectedException {
      if (!mounted) return;
      setState(() {
        _pendingPairing = null;
        _pairingInFlight = false;
        _pairingCancelled = false;
        _pairingError = null;
        _status = _t('pair.rejected');
      });
      _showToast(_t('pair.rejected'));
    } catch (error) {
      if (!mounted) return;
      setState(() {
        _pairingInFlight = false;
        _pairingCancelled = false;
        _pairingError = error.toString().replaceFirst('Exception: ', '');
        _status = _pairingError ?? _t('pair.failed');
      });
    }
  }

  Future<StoredDevice> _confirmRelayPairing(
    PairingPayload payload,
    String name,
  ) async {
    setState(() => _status = _t('pair.waiting'));
    try {
      return await Future.any<StoredDevice>([
        claimPairingOverRelay(
          payload: payload,
          name: name,
          timeout: const Duration(seconds: 90),
        ),
        _waitPairingCancelled(),
      ]);
    } on PairingRejectedException {
      rethrow;
    }
  }

  Future<StoredDevice> _waitPairingCancelled() async {
    while (!_pairingCancelled) {
      await Future<void>.delayed(const Duration(milliseconds: 100));
    }
    throw const PairingCancelledException();
  }

  Future<void> _saveSettings() async {
    final next = _mobileSettingsController.saveSettings(
      current: _settings,
      inputLocalName: _settingsNameController.text,
      detectedDeviceName: _detectedDeviceName,
    );
    await _storage.saveSettings(next);
    CoduxLog.setLevelName(next.logLevel);
    _nativeTerminalController?.setLogLevel(CoduxLog.nativeLevelName);
    setState(() {
      _settings = next;
      _status = _t('settings.saved');
    });
    _popCupertinoPage(() {
      _showSettings = false;
    });
    _sendDeviceInfo(force: true);
  }

  void _connect([StoredDevice? device, bool background = false]) {
    final target = device ?? _activeDevice;
    if (target == null) {
      setState(() => _showScanner = true);
      return;
    }
    if (_protocolBlockedHostIds.contains(target.hostId)) {
      if (!background) {
        setState(() => _status = _t('connection.upgradeRequired'));
      }
      return;
    }
    _shouldReconnect = true;
    _backgroundConnect = background;
    _cancelRemoteSyncTimers();
    final generation = _remoteSyncController.beginConnectionGeneration();
    _resetRemoteRuntime(keepProjects: background);
    _hostRuntimeInstanceId = null;
    CoduxLog.info(
      '[codux-flutter-remote] connect start gen=$generation background=$background host=${target.hostId} device=${target.deviceId} transport=${target.transport}',
    );
    _cancelHostResponseProbe();
    _reconnectTimer?.cancel();
    _transportCloseTimer?.cancel();
    _healthTimer?.cancel();
    _clearLatencyProbe();
    unawaited(_closeActiveTransport());
    _transportConnected = false;
    _sendQueue.reset(seed: DateTime.now().microsecondsSinceEpoch);
    _receiveSequenceGuard.reset();
    _receiveChain = Future<void>.value();
    RemoteE2ECrypto.clearCache();
    if (background && _lastConnectedAt != null) {
      _startConnectionGrace(reason: 'background_connect');
    }
    if (!background) _clearTerminal();
    if (!background) _terminalInputBatcher.reset();
    setState(() {
      _transportReady = false;
      _remoteSyncController.resetProtocolReady();
      _hostResponsive = false;
      _connectionPath = 'unknown';
      _latencyMs = null;
      if (!background) {
        _status = _t('app.connecting');
        _worktrees = [];
        _worktreeBaseBranches = [];
        _defaultWorktreeBaseBranch = null;
        _selectedWorktreeId = null;
        _showTerminalSwitcher = false;
        _terminalBufferRetry.reset();
        _terminalOutputController.resetTransient();
        _setTerminalBufferLoading(false);
      }
      _activeDevice = target;
    });
    unawaited(_restoreCachedProjects(target));
    if (target.preferredTransport.url.trim().isEmpty) {
      setState(() => _status = _t('pair.repairRequired'));
      return;
    }
    final transport = (widget.transportFactory ?? createRemoteTransport)(target)
      ..onState = _handleTransportState
      ..onEnvelope = (envelope) {
        _handleTransportEnvelopeQueued(RelayEnvelope.fromJson(envelope));
      };
    _activeTransport = transport;
    transport.connect(target).catchError((Object error) {
      CoduxLog.warn(
        '[codux-flutter-remote] connect failed gen=$generation error=$error',
      );
      if (generation != _transportGeneration) return;
      if (!_backgroundConnect && mounted) {
        setState(() => _status = _t('connection.failedRetry'));
      }
      _handleTransportClosed('connect_failed');
    });
    _healthTimer = Timer(const Duration(seconds: 16), () {
      if (generation != _transportGeneration) return;
      if (!_transportConnected) {
        CoduxLog.warn('[codux-flutter-remote] connect timeout gen=$generation');
        if (!_backgroundConnect && mounted) {
          setState(() => _status = _t('connection.failedRetry'));
        }
        _handleTransportClosed('hello_timeout');
      }
    });
  }

  void _scheduleReconnect(StoredDevice target) {
    if (!_shouldReconnect) return;
    _reconnectTimer?.cancel();
    _reconnectAttempt += 1;
    final delay = Duration(
      milliseconds: (800 * (1 << (_reconnectAttempt - 1).clamp(0, 5))).clamp(
        800,
        30000,
      ),
    );
    CoduxLog.info(
      '[codux-flutter-remote] reconnect scheduled host=${target.hostId} device=${target.deviceId} attempt=$_reconnectAttempt delayMs=${delay.inMilliseconds}',
    );
    _reconnectTimer = Timer(delay, () => _connect(target, true));
  }

  void _sendInitialTransportRequests({bool force = false}) {
    final plan = _remoteSyncController.initialSyncPlan(
      transportReady: _transportReady,
      transportConnected: _transportConnected,
      force: force,
    );
    if (!plan.hasWork) {
      return;
    }
    if (plan.resetTerminalBufferRetry) {
      _terminalBufferRetry.reset();
    }
    CoduxLog.info('[codux-flutter-remote] request initial sync force=$force');
    if (plan.sendDeviceInfo) {
      _sendDeviceInfo(force: force);
    }
    if (plan.requestProjectList) {
      _requestProjectList(resetRetry: force);
    }
    if (plan.requestTerminalList) {
      _requestTerminalList(resetRetry: force);
    }
  }

  void _sendDeviceInfo({bool force = false}) {
    if (!_remoteSyncController.shouldSendDeviceInfo(force: force)) return;
    final target = _activeDevice;
    final sent = _send(
      RelayEnvelope(
        type: 'device.info',
        payload: {
          'name': _settings.localName.isNotEmpty
              ? _settings.localName
              : (target?.name ?? _detectedDeviceName),
        },
      ),
    );
    if (sent) {
      _remoteSyncController.markDeviceInfoSent();
    }
  }

  void _requestProjectList({bool resetRetry = false}) {
    if (!_remoteProtocolReady) return;
    if (resetRetry) {
      _projectListRetryTimer?.cancel();
      _projectListRetryTimer = null;
      _remoteSync.resetProjectListRetry();
    }
    if (!_remoteSync.shouldRequestProjectList(force: resetRetry)) return;
    final sent = _send(const RelayEnvelope(type: 'project.list'));
    if (sent && !_projectListLoaded) {
      _remoteSync.markProjectListRequested();
      CoduxLog.info(
        '[codux-flutter-projects] request project.list attempt=$_projectListRetryAttempt',
      );
      _scheduleProjectListRetry();
    }
  }

  void _scheduleProjectListRetry() {
    if (!_transportReady || _projectListLoaded) return;
    _projectListRetryTimer?.cancel();
    if (!_remoteSync.canRetryProjectList(6)) return;
    final delay = Duration(
      milliseconds: (800 * (1 << _projectListRetryAttempt)).clamp(800, 5000),
    );
    _projectListRetryTimer = Timer(delay, () {
      if (!mounted || !_transportReady || _projectListLoaded) return;
      final attempt = _remoteSync.nextProjectListRetryAttempt();
      CoduxLog.info(
        '[codux-flutter-projects] retry project.list attempt=$attempt',
      );
      _requestProjectList();
    });
  }

  void _markProjectListReceived() {
    _remoteSync.markProjectListReceived();
    _projectListRetryTimer?.cancel();
    _projectListRetryTimer = null;
    CoduxLog.debug('[codux-flutter-projects] project.list received');
  }

  void _requestTerminalList({bool resetRetry = false}) {
    if (!_remoteProtocolReady) return;
    if (resetRetry) {
      _terminalListRetryTimer?.cancel();
      _terminalListRetryTimer = null;
      _remoteSync.resetTerminalListRetry();
    }
    if (!_remoteSync.shouldRequestTerminalList(force: resetRetry)) return;
    final sent = _send(const RelayEnvelope(type: 'terminal.list'));
    if (sent && !_terminalListLoaded) {
      _remoteSync.markTerminalListRequested();
      CoduxLog.info(
        '[codux-flutter-terminal] request terminal.list attempt=$_terminalListRetryAttempt',
      );
      _scheduleTerminalListRetry();
    }
  }

  void _requestWorktreeList({bool loading = false}) {
    final project = _selectedProject;
    if (!_remoteProtocolReady || project == null) return;
    if (loading) {
      setState(() => _worktreeListLoading = true);
    }
    _send(_worktreeController.listEnvelope(project));
  }

  void _scheduleTerminalListRetry() {
    if (!_transportReady || _terminalListLoaded) return;
    _terminalListRetryTimer?.cancel();
    if (!_remoteSync.canRetryTerminalList(6)) return;
    final delay = Duration(
      milliseconds: (800 * (1 << _terminalListRetryAttempt)).clamp(800, 5000),
    );
    _terminalListRetryTimer = Timer(delay, () {
      if (!mounted || !_transportReady || _terminalListLoaded) return;
      final attempt = _remoteSync.nextTerminalListRetryAttempt();
      CoduxLog.info(
        '[codux-flutter-terminal] retry terminal.list attempt=$attempt',
      );
      _requestTerminalList();
    });
  }

  void _markTerminalListReceived() {
    _remoteSync.markTerminalListReceived();
    _terminalListRetryTimer?.cancel();
    _terminalListRetryTimer = null;
    CoduxLog.debug('[codux-flutter-terminal] terminal.list received');
  }

  void _markActiveDeviceResponsive() {
    final device = _activeDevice;
    if (device != null) _rememberActiveDevice(device);
  }

  void _sendProjectSelect(String projectId, {required String reason}) {
    CoduxLog.info(
      '[codux-flutter-projects] send project.select reason=$reason project=$projectId',
    );
    _send(
      RelayEnvelope(type: 'project.select', payload: {'projectId': projectId}),
    );
  }

  bool _replaceTerminalProjectSubscription(
    String projectId, {
    required String reason,
  }) {
    final maxChars = _terminalBufferCapability.maxChars.clamp(
      1,
      _terminalBufferMaxChars,
    );
    final plan = _terminalSubscriptions.replaceProject(
      projectId,
      baseline: true,
      maxChars: maxChars,
      chunkChars: _terminalBufferCapability.chunking
          ? _terminalBufferCapability.chunkChars
          : null,
    );
    if (!plan.hasWork) return false;
    final unsubscribe = plan.unsubscribe;
    if (unsubscribe != null) {
      CoduxLog.debug(
        '[codux-flutter-terminal] unsubscribe project=${plan.unsubscribeProjectId ?? ''} reason=$reason',
      );
      _send(unsubscribe);
    }
    final subscribe = plan.subscribe;
    var baselineRequested = false;
    if (subscribe != null) {
      CoduxLog.debug(
        '[codux-flutter-terminal] subscribe project=${plan.subscribeProjectId ?? ''} reason=$reason',
      );
      _send(subscribe);
      baselineRequested = true;
    }
    return baselineRequested;
  }

  void _syncRuntimeViewState() {
    _projects = _remoteRuntime.projects;
    _terminals = _remoteRuntime.terminals;
    _selectedProjectId = _remoteRuntime.selectedProjectId;
    _sessionId = _remoteRuntime.activeSessionId;
    _creatingTerminalProjectId = _remoteRuntime.creatingTerminalProjectId;
  }

  bool get _terminalBufferLoading =>
      _terminalBufferPhase != RemoteTerminalBufferPhase.idle;

  void _setTerminalBufferLoading(
    bool loading, {
    double? progress,
    RemoteTerminalBufferPhase phase = RemoteTerminalBufferPhase.requesting,
  }) {
    _terminalBufferPhase = loading ? phase : RemoteTerminalBufferPhase.idle;
    _terminalBufferProgress = loading ? progress : null;
  }

  String _terminalHistoryLoadingText() {
    if (_terminalBufferPhase == RemoteTerminalBufferPhase.rendering) {
      return _t('terminal.renderingHistory');
    }
    final progress = _terminalBufferProgress;
    if (progress == null) return _t('terminal.loadingHistory');
    final percent = (progress.clamp(0.0, 1.0) * 100).round();
    return _t(
      'terminal.loadingHistoryProgress',
      params: {'percent': '$percent'},
    );
  }

  void _applyRuntimePlan(RemoteRuntimePlan plan, {String reason = ''}) {
    if (plan.removedSessionId != null) {
      final removed = plan.removedSessionId!;
      _terminalOutputController.removeSession(removed);
      _terminalInputSender.clear(sessionId: removed);
    }
    if (plan.resetTerminalInput) {
      _terminalInputBatcher.reset();
    }
    if (plan.resetTerminalBuffer) {
      _terminalBufferRetry.reset();
      _terminalOutputController.resetTransient();
      _setTerminalBufferLoading(false);
      _terminalCursorBottom = 0;
    }
    if (plan.stateChanged && mounted) {
      setState(_syncRuntimeViewState);
    } else {
      _syncRuntimeViewState();
    }
    if (plan.clearTerminal) {
      _clearTerminal();
    }
    if (plan.requestTerminalList) {
      _requestTerminalList();
    }
    if (plan.requestProjectSelectId != null) {
      _sendProjectSelect(plan.requestProjectSelectId!, reason: reason);
    }
    if (plan.bindSessionId != null) {
      var baselineRequested = false;
      final selectedProjectId = _selectedProjectId;
      if (selectedProjectId != null) {
        baselineRequested = _replaceTerminalProjectSubscription(
          selectedProjectId,
          reason: 'bind-$reason',
        );
      }
      final restored = _restoreTerminalSessionFromCache(plan.bindSessionId!);
      CoduxLog.info(
        '[codux-flutter-terminal] bind session=${plan.bindSessionId} project=${_selectedProjectId ?? ''} cached=$restored',
      );
      final needsFullBuffer =
          plan.bindFullBuffer &&
          !_terminalOutputController.hasCachedOutput(plan.bindSessionId!);
      _terminalOutputController.bindSession(
        plan.bindSessionId!,
        requireSnapshot: needsFullBuffer,
      );
      _claimTerminalViewport(sessionId: plan.bindSessionId!);
      _flushPendingTerminalResize(force: true);
      if (!baselineRequested) {
        _requestBufferIfReady(force: true, full: needsFullBuffer);
      }
      if (plan.flushTerminalInput) {
        _terminalInputBatcher.flush();
      }
    }
  }

  Future<void> _cacheProjects(List<ProjectInfo> projects) async {
    final device = _activeDevice;
    if (device == null) return;
    try {
      await _storage.saveCachedProjects(device, projects);
    } catch (error) {
      CoduxLog.warn('[codux-flutter-projects] cache save failed: $error');
    }
  }

  bool _send(RelayEnvelope message) {
    if (!_transportConnected) {
      setState(() => _status = _t('app.remoteNotConnected'));
      CoduxLog.warn(
        '[codux-flutter-remote] drop type=${message.type} reason=not_ready',
      );
      return false;
    }
    final transport = _activeTransport;
    if (transport == null) return false;
    CoduxLog.debug(
      '[codux-flutter-remote] send type=${message.type} session=${message.sessionId ?? ''}',
    );
    unawaited(
      _sendQueue.send(
        message: message,
        transport: transport,
        connected: () => _transportConnected,
        activeDevice: _activeDevice,
        onError: (error) {
          CoduxLog.error('[codux-flutter-e2e] encrypt failed: $error');
          if (mounted) setState(() => _status = _t('pair.repairRequired'));
        },
      ),
    );
    return true;
  }

  bool _sendTerminalEnvelope(RelayEnvelope message, {TerminalInfo? terminal}) {
    final scoped = _scopeTerminalEnvelope(message, terminal: terminal);
    if (scoped == null) return false;
    return _send(scoped);
  }

  Future<bool> _sendTerminalEnvelopeReliable(
    RelayEnvelope message, {
    TerminalInfo? terminal,
  }) async {
    final scoped = _scopeTerminalEnvelope(message, terminal: terminal);
    if (scoped == null) return false;
    return _send(scoped);
  }

  Future<bool> _sendTerminalUploadEnvelopeReliable(
    RelayEnvelope message,
  ) async {
    if (!_canUploadOverCurrentPath) {
      CoduxLog.warn(
        '[codux-flutter-upload] blocked upload on path=$_connectionPath type=${message.type}',
      );
      if (mounted) {
        setState(() => _status = _t('upload.directRequired'));
      }
      return false;
    }
    return _sendTerminalEnvelopeReliable(message);
  }

  Future<void> _handleTransportEnvelope(
    RelayEnvelope message,
    StoredDevice target,
  ) async {
    try {
      if (message.type == 'secure.message') {
        message = await RemoteE2ECrypto.decryptEnvelope(
          outer: message,
          device: target,
        );
        final seq = message.seq;
        if (!_receiveSequenceGuard.accept(
          type: message.type,
          sessionId: message.sessionId,
          seq: seq,
        )) {
          CoduxLog.debug(
            '[codux-flutter-e2e] drop duplicate seq=$seq type=${message.type} session=${message.sessionId ?? ''}',
          );
          return;
        }
      }
      _healthTimer?.cancel();
      _healthTimer = null;
      CoduxLog.debug(
        '[codux-flutter-remote] recv type=${message.type} session=${message.sessionId ?? ''}',
      );
      switch (message.type) {
        case 'hello':
          _reconnectAttempt = 0;
          CoduxLog.info('[codux-flutter-remote] hello received');
          if (!_transportReady) {
            setState(() {
              _transportReady = true;
              _hasShownTerminal = true;
              if (!_backgroundConnect) _status = _t('app.connected');
            });
            _markTransportConnected(target.transport);
          }
          _sendHostInfoRequest(force: true);
          _sendInitialTransportRequests();
          _startHostResponseProbe(reason: 'hello');
        case 'host.offline':
          final payload = message.payload;
          final messageText = payload is Map
              ? '${payload['message'] ?? _t('connection.macDisconnected')}'
              : _t('connection.macDisconnected');
          _terminalInputBatcher.reset();
          _terminalInputSender.clear();
          _clearLatencyProbe();
          setState(() {
            _transportReady = false;
            _remoteSyncController.resetProtocolReady();
            _hostResponsive = false;
            _showTerminal = false;
            _workspaceMode = 'terminal';
            _worktrees = [];
            _worktreeBaseBranches = [];
            _resetRemoteSyncState();
            _defaultWorktreeBaseBranch = null;
            _selectedWorktreeId = null;
            _showTerminalSwitcher = false;
            _status = messageText;
            _terminalBufferRetry.reset();
            _terminalOutputController.resetTransient();
            _setTerminalBufferLoading(false);
          });
          _clearConnectionGrace();
          _cancelHostResponseProbe();
          _scheduleReconnect(target);
        case 'secure.required':
          setState(() {
            _status = _t('pair.repairRequired');
          });
        case 'host.info':
          if (!_isCompatibleRemoteProtocol(message.payload)) {
            _failRemoteProtocol(target, message.payload);
            return;
          }
          final hostRuntimeChanged = _recordHostRuntimeInstance(
            message.payload,
          );
          _markHostResponsive('host.info', transport: target.transport);
          _markActiveDeviceResponsive();
          final payload = message.payload;
          if (payload is Map) {
            _terminalBufferCapability = TerminalBufferCapability.fromHostInfo(
              payload,
            );
            if (payload['name'] != null) {
              _updateDevice(
                target.deviceId,
                hostName: payload['name']?.toString(),
              );
            }
          }
          _markRemoteProtocolReady(
            force:
                hostRuntimeChanged ||
                !_projectListLoaded ||
                !_terminalListLoaded,
          );
          _startLatencyProbe();
        case 'transport.pong':
          _markHostResponsive('transport.pong');
          _recordTransportPong(message.payload);
        case 'project.selected':
          _handleProjectSelected(message);
        case 'project.list':
          _handleProjectList(message);
        case 'terminal.list':
          _handleTerminalList(message);
        case 'terminal.created':
          _handleTerminalCreated(message);
        case 'terminal.closed':
          _handleTerminalClosed(message);
        case 'terminal.viewport.state':
          _handleTerminalViewportState(message);
        case 'worktree.list':
          _handleWorktreeList(message);
        case 'worktree.updated':
          _handleWorktreeUpdated(message);
          _requestTerminalList(resetRetry: true);
        case 'terminal.output':
          _handleTerminalOutput(message);
        case 'error':
          _handleRemoteError(message);
        case 'file.list':
          _handleFileList(message);
        case 'project.updated':
          _refreshLists();
          _showToast(_t('project.updated'));
        case 'ai.stats':
          final payload = message.payload;
          if (payload is Map<String, dynamic>) {
            setState(() {
              _currentAIStats = AIStatsInfo.fromJson(payload);
              _aiStatsLoading = false;
              _workspaceMode = 'stats';
            });
          }
        case 'git.status':
          final status = remoteGitStatusFromPayload(message.payload);
          if (status != null) {
            final plan = _remoteRuntime.applyGitStatus(status);
            _applyRuntimePlan(plan, reason: 'git-status');
          }
        case 'file.read':
          _handleFileRead(message);
        case 'file.written':
          setState(() => _fileEditorSaving = false);
          _showToast(_t('file.saved'));
        case 'file.renamed':
          _requestProjectFiles(_projectFilesPath);
          _showToast(_t('file.renamed'));
        case 'file.deleted':
          _handleFileDeleted(message);
          _requestProjectFiles(_projectFilesPath);
          _showToast(_t('file.deleted'));
        case 'terminal.uploaded':
          _handleTerminalUploaded(message);
        case 'terminal.upload.ack':
          _terminalUploadSender.handleAck(message);
        case 'terminal.input.ack':
          _terminalInputSender.handleAck(message);
      }
    } catch (error) {
      CoduxLog.error('[codux-flutter-e2e] receive failed: $error');
    }
  }

  void _handleProjectSelected(RelayEnvelope message) {
    _markHostResponsive('project.selected');
    _markActiveDeviceResponsive();
    final payload = message.payload;
    final projectId = payload is Map ? payload['projectId']?.toString() : null;
    final plan = _remoteRuntime.projectSelected(projectId);
    _applyRuntimePlan(plan, reason: 'project-selected');
    _requestProjectList();
  }

  void _handleProjectList(RelayEnvelope message) {
    _markHostResponsive('project.list');
    _markActiveDeviceResponsive();
    _markProjectListReceived();
    final payload = message.payload;
    final next = remoteProjectsFromPayload(payload);
    final remoteSelectedProjectId = remoteSelectedProjectIdFromPayload(payload);
    final plan = _remoteRuntime.applyProjectList(
      projects: next,
      remoteSelectedProjectId: remoteSelectedProjectId,
      terminalVisible: _showTerminal && _workspaceMode == 'terminal',
      terminalListLoaded: _terminalListLoaded,
    );
    _applyRuntimePlan(plan, reason: 'missing-terminal');
    CoduxLog.debug(
      '[codux-flutter-projects] project.list count=${next.length} selected=${_selectedProjectId ?? ''}',
    );
    unawaited(_cacheProjects(next));
  }

  void _handleTerminalList(RelayEnvelope message) {
    _markHostResponsive('terminal.list');
    _markActiveDeviceResponsive();
    _markTerminalListReceived();
    final next = remoteTerminalsFromPayload(message.payload);
    CoduxLog.debug(
      '[codux-flutter-terminal] terminal.list count=${next.length}',
    );
    final plan = _remoteRuntime.applyTerminalList(
      terminals: next,
      terminalVisible: _showTerminal && _workspaceMode == 'terminal',
      terminalListLoaded: _terminalListLoaded,
    );
    _applyRuntimePlan(plan, reason: 'missing-terminal');
  }

  void _handleTerminalCreated(RelayEnvelope message) {
    final terminal = remoteTerminalFromPayload(message.payload);
    if (terminal == null) return;
    CoduxLog.info(
      '[codux-flutter-terminal] created session=${terminal.id} project=${terminal.projectId}',
    );
    final plan = _remoteRuntime.terminalCreated(terminal);
    _applyRuntimePlan(plan, reason: 'terminal-created');
  }

  void _handleTerminalClosed(RelayEnvelope message) {
    final closedSessionId = message.sessionId;
    if (closedSessionId == null) return;
    final plan = _remoteRuntime.removeTerminal(closedSessionId);
    _applyRuntimePlan(plan, reason: 'terminal-closed');
  }

  void _handleWorktreeList(RelayEnvelope message) {
    _markHostResponsive('worktree.list');
    _applyWorktreeState(message);
  }

  void _handleWorktreeUpdated(RelayEnvelope message) {
    _applyWorktreeState(message);
  }

  void _applyWorktreeState(RelayEnvelope message) {
    final worktreeState = _worktreeController.stateFromPayload(message.payload);
    if (worktreeState == null) return;
    setState(() {
      _worktrees = worktreeState.worktrees;
      _selectedWorktreeId = worktreeState.selectedWorktreeId;
      _worktreeBaseBranches = worktreeState.baseBranches;
      _defaultWorktreeBaseBranch = worktreeState.defaultBaseBranch;
      _worktreeListLoading = false;
    });
  }

  void _handleRemoteError(RelayEnvelope message) {
    final payload = message.payload;
    final errorMessage =
        message.error ??
        (payload is Map
            ? '${payload['message'] ?? _t('remote.error')}'
            : _t('remote.error'));
    CoduxLog.warn(
      '[codux-flutter-remote] error type=${message.type} session=${message.sessionId ?? ''} message=$errorMessage',
    );
    final isActiveTerminalError =
        message.sessionId != null && message.sessionId == _sessionId;
    if (isActiveTerminalError) {
      _terminalBufferRetry.reset();
    }
    setState(() {
      _aiStatsLoading = false;
      _filePickerLoading = false;
      _worktreeListLoading = false;
      _blockingLoadingMessage = null;
      if (isActiveTerminalError) {
        _terminalOutputController.resetSessionTransient(message.sessionId!);
        _setTerminalBufferLoading(false);
      }
      _status = errorMessage;
    });
  }

  void _handleFileList(RelayEnvelope message) {
    final listState = _projectFileController.listStateFromPayload(
      message.payload,
    );
    if (listState != null) {
      _applyFileListState(listState);
    }
  }

  void _handleFileRead(RelayEnvelope message) {
    final fileState = _projectFileController.readStateFromPayload(
      message.payload,
    );
    if (fileState == null) return;
    setState(() {
      _applyFileEditorState(fileState);
    });
    if (!fileState.editable) {
      _showToast(_t('file.readOnlyLarge'));
    }
  }

  void _handleFileDeleted(RelayEnvelope message) {
    final deletedPath = _projectFileController.deletedPathFromPayload(
      message.payload,
    );
    if (_projectFileController.shouldCloseEditorAfterDelete(
      deletedPath: deletedPath,
      editingPath: _editingFilePath,
    )) {
      setState(() => _editingFilePath = null);
    }
  }

  void _handleTransportState(String rawState) {
    final event = RemoteTransportStateEvent.parse(rawState);
    final state = event.state;
    final detail = event.detail;
    CoduxLog.info(
      detail.isEmpty
          ? '[codux-flutter-remote] state=$state'
          : '[codux-flutter-remote] state=$state detail=$detail',
    );
    if (!mounted || _disposing) return;
    if (event.isPathUpdate) {
      final path = event.path;
      if (path != null) {
        final previousPath = _connectionPath;
        final changed = path != _connectionPath;
        _pingTimeoutTimer?.cancel();
        _pingTimeoutTimer = null;
        _transportStateController.cancelPendingPing();
        final shouldResetRuntime =
            changed &&
            _hostResponsive &&
            (previousPath == 'direct' || previousPath == 'mixed') &&
            (path == 'relay' || path == 'none');
        if (shouldResetRuntime) {
          _resetRemoteRuntimeAfterHostRestart(
            'path-changed:$previousPath->$path',
          );
        }
        _markTransportPath(path);
        if (path != 'none') {
          _sendHostInfoRequest(
            force:
                shouldResetRuntime ||
                !_remoteProtocolReady ||
                !_projectListLoaded ||
                !_terminalListLoaded,
          );
          _sendInitialTransportRequests(force: shouldResetRuntime);
          _startHostResponseProbe(reason: 'transport_path');
          if (changed) _sendTransportPing();
        }
        return;
      }
    }
    if (event.isConnected) {
      _markTransportOpen();
      _sendHostInfoRequest(
        force:
            !_remoteProtocolReady ||
            !_projectListLoaded ||
            !_terminalListLoaded,
      );
      _sendInitialTransportRequests();
      _startHostResponseProbe(reason: 'transport');
      return;
    }
    if (event.isClosed) {
      _handleTransportClosed(state);
    }
  }

  void _handleTransportEnvelopeQueued(RelayEnvelope message) {
    CoduxLog.debug(
      '[codux-flutter-remote] envelope type=${message.type} session=${message.sessionId ?? ''}',
    );
    final target = _activeDevice;
    if (target == null) return;
    final runtimeEpoch = _remoteRuntimeEpoch;
    final previous = _receiveChain.catchError((_) {});
    final task = previous
        .then((_) {
          if (runtimeEpoch != _remoteRuntimeEpoch) {
            CoduxLog.debug(
              '[codux-flutter-remote] drop stale envelope epoch=$runtimeEpoch current=$_remoteRuntimeEpoch type=${message.type} session=${message.sessionId ?? ''}',
            );
            return Future<void>.value();
          }
          return _handleTransportEnvelope(message, target);
        })
        .catchError((Object error) {
          CoduxLog.error('[codux-flutter-remote] receive queue failed: $error');
        });
    _receiveChain = task;
  }

  void _handleTransportClosed(String reason) {
    _transportConnected = false;
    _resetRemoteRuntimeAfterHostRestart('transport-closed:$reason');
    setState(() {
      _transportReady = false;
      _hostResponsive = false;
      _status = _t('app.reconnecting');
    });
    if (_lastConnectedAt == null) {
      _clearConnectionGrace();
    } else {
      _startConnectionGrace(reason: reason);
    }
    final target = _activeDevice;
    if (target != null && _appInForeground && !_appSuspended) {
      _scheduleReconnect(target);
    }
  }

  Future<void> _closeActiveTransport() async {
    final transport = _activeTransport;
    _activeTransport = null;
    await transport?.close();
  }

  void _handleTerminalViewportState(RelayEnvelope message) {
    _terminalViewportController.applyRemoteState(message);
  }

  void _handleTerminalOutput(RelayEnvelope message) {
    final effects = _terminalOutputController.accept(
      message,
      activeSessionId: _sessionId,
    );
    _applyTerminalOutputEffects(effects);
  }

  void _applyTerminalOutputEffects(List<RemoteTerminalOutputEffect> effects) {
    for (final effect in effects) {
      switch (effect.kind) {
        case RemoteTerminalOutputEffectKind.loading:
          if (mounted) {
            setState(
              () => _setTerminalBufferLoading(
                effect.loading,
                progress: effect.progress,
                phase: effect.phase ?? RemoteTerminalBufferPhase.requesting,
              ),
            );
          } else {
            _setTerminalBufferLoading(
              effect.loading,
              progress: effect.progress,
              phase: effect.phase ?? RemoteTerminalBufferPhase.requesting,
            );
          }
        case RemoteTerminalOutputEffectKind.ack:
          final sessionId = effect.sessionId;
          if (sessionId != null) {
            _ackTerminalOutputIfNeeded(
              sessionId,
              effect.outputSeq,
              effect.bufferLength,
            );
          }
        case RemoteTerminalOutputEffectKind.requestFullBuffer:
          _terminalBufferRetry.resetLastBuffered();
          _requestBufferIfReady(force: true, full: true);
        case RemoteTerminalOutputEffectKind.requestBufferPage:
          final sessionId = effect.sessionId;
          final offset = effect.offset;
          if (sessionId != null && offset != null) {
            _requestBufferPage(sessionId, offset);
          }
        case RemoteTerminalOutputEffectKind.markBufferReceived:
          _markTerminalBufferReceived(effect.sessionId);
        case RemoteTerminalOutputEffectKind.renderSnapshot:
          final sessionId = effect.sessionId;
          final data = effect.data;
          if (sessionId != null && data != null) {
            _renderTerminalSnapshot(data, sessionId: sessionId);
          }
        case RemoteTerminalOutputEffectKind.writeData:
          final data = effect.data;
          if (data != null) {
            _writeTerminalData(data, replayingBuffer: effect.replayingBuffer);
          }
      }
    }
  }

  void _handleTerminalUploaded(RelayEnvelope message) {
    final payload = message.payload;
    if (payload is Map && payload['path'] != null) {
      final completion = _terminalUploadCompletion;
      if (completion != null && !completion.isCompleted) {
        completion.complete();
      }
      _terminalUploadCompletion = null;
      final inserted = payload['inserted'] == true;
      final mode = payload['mode']?.toString();
      final tool = payload['tool']?.toString();
      final kind = payload['kind']?.toString();
      if (!inserted) {
        final path = '${payload['path']}';
        _insertTerminalText('$path ');
      }
      setState(() {
        _terminalUploadLoading = false;
        _terminalUploadStatus = '';
        _status = kind == 'file'
            ? _t('upload.fileSentPath')
            : mode == 'clipboard'
            ? _t(
                'upload.imageSentTool',
                params: {'tool': tool ?? _t('upload.aiTool')},
              )
            : _t('upload.imageSentPath');
      });
    }
  }

  StoredDevice? _updateDevice(String deviceId, {String? hostName}) {
    final result = _deviceController.updateHostName(
      devices: _devices,
      activeDevice: _activeDevice,
      deviceId: deviceId,
      hostName: hostName,
    );
    final updated = result.updatedDevice;
    if (updated != null) {
      setState(() {
        _devices = result.state.devices;
        _activeDevice = result.state.activeDevice;
      });
      unawaited(_storage.saveDevices(result.state.devices));
    }
    return updated;
  }

  void _requestBufferIfReady({bool force = false, bool full = false}) {
    final sent = _terminalBufferRetry.requestIfReady(
      sessionId: _sessionId,
      force: force,
      send: (sessionId) {
        final requestId = _nextTerminalBufferRequestId(sessionId);
        _terminalOutputController.startBufferRequest(
          sessionId,
          requestId,
          requireSnapshot: full,
        );
        final maxChars = _terminalBufferCapability.maxChars.clamp(
          1,
          _terminalBufferMaxChars,
        );
        final sequenceFor = _terminalOutputController.sequenceFor(sessionId);
        return _sendTerminalEnvelope(
          RelayEnvelope(
            type: 'terminal.buffer',
            sessionId: sessionId,
            payload: buildTerminalBufferRequestPayload(
              requestId: requestId,
              mode: full
                  ? TerminalBufferRequestMode.historyRestore
                  : TerminalBufferRequestMode.liveResume,
              offset: _terminalOutputController.bufferOffset(sessionId),
              maxChars: maxChars,
              chunking: _terminalBufferCapability.chunking,
              chunkChars: _terminalBufferCapability.chunkChars,
              resumeFromSeq: sequenceFor,
            ),
          ),
        );
      },
    );
    if (sent && !_terminalBufferLoading && mounted) {
      CoduxLog.info(
        '[codux-flutter-terminal] request terminal.buffer session=${_sessionId ?? ''} full=$full tail=false',
      );
      setState(() => _setTerminalBufferLoading(true));
    }
  }

  void _requestBufferForCurrentSession({
    bool force = false,
    bool preferFull = false,
  }) {
    final sessionId = _sessionId;
    final full =
        preferFull &&
        sessionId != null &&
        !_terminalOutputController.hasCachedOutput(sessionId);
    _requestBufferIfReady(force: force, full: full);
  }

  void _requestBufferPage(String sessionId, int offset) {
    final sent = _terminalBufferRetry.requestIfReady(
      sessionId: sessionId,
      force: true,
      send: (id) {
        final requestId =
            _terminalOutputController.activeBufferRequestId(id) ??
            _nextTerminalBufferRequestId(id);
        _terminalOutputController.startBufferRequest(id, requestId);
        final maxChars = _terminalBufferCapability.maxChars.clamp(
          1,
          _terminalBufferMaxChars,
        );
        return _sendTerminalEnvelope(
          RelayEnvelope(
            type: 'terminal.buffer',
            sessionId: id,
            payload: buildTerminalBufferRequestPayload(
              requestId: requestId,
              mode: TerminalBufferRequestMode.historyPage,
              offset: offset,
              maxChars: maxChars,
              chunking: _terminalBufferCapability.chunking,
              chunkChars: _terminalBufferCapability.chunkChars,
            ),
          ),
        );
      },
    );
    if (sent && mounted) {
      setState(
        () => _setTerminalBufferLoading(
          true,
          phase: RemoteTerminalBufferPhase.receiving,
        ),
      );
    }
  }

  String _nextTerminalBufferRequestId(String sessionId) {
    _terminalBufferRequestCounter += 1;
    return '${DateTime.now().microsecondsSinceEpoch}-$_terminalBufferRequestCounter-$sessionId';
  }

  void _markTerminalBufferReceived(String? sessionId) {
    _terminalBufferRetry.markReceived(
      sessionId: sessionId,
      activeSessionId: _sessionId,
    );
    if (_terminalBufferLoading && mounted) {
      setState(() => _setTerminalBufferLoading(false));
    }
    CoduxLog.info(
      '[codux-flutter-terminal] terminal.buffer received session=${sessionId ?? ''}',
    );
  }

  void _clearTerminal() {
    _terminalRenderer.clear(sessionId: _sessionId);
  }

  void _writeTerminalData(String data, {required bool replayingBuffer}) {
    _terminalRenderer.write(data, replayingBuffer: replayingBuffer);
  }

  void _renderTerminalSnapshot(String data, {required String sessionId}) {
    if (mounted) {
      setState(
        () => _setTerminalBufferLoading(
          true,
          phase: RemoteTerminalBufferPhase.rendering,
        ),
      );
    } else {
      _setTerminalBufferLoading(
        true,
        phase: RemoteTerminalBufferPhase.rendering,
      );
    }
    final render = _terminalRenderer.replace(data, replayingBuffer: true);
    unawaited(
      render.whenComplete(() {
        if (!mounted || _sessionId != sessionId) return;
        _markTerminalBufferReceived(sessionId);
      }),
    );
  }

  void _replayCurrentTerminalToNative({
    required CoduxNativeTerminalController controller,
    required String reason,
  }) {
    if (_nativeTerminalController != controller) return;
    final sessionId = _sessionId;
    if (sessionId == null) return;
    final cached = _terminalOutputController.cachedOutput(sessionId);
    _terminalRenderer.replayCached(
      cached ?? '',
      sessionId: sessionId,
      reason: reason,
    );
  }

  bool _restoreNativeTerminalController(
    CoduxNativeTerminalController controller,
  ) {
    final sessionId = _sessionId;
    final cached = sessionId == null
        ? null
        : _terminalOutputController.cachedOutput(sessionId);
    return _terminalRenderer.restoreControllerWithCached(cached);
  }

  void _sendTerminalResize(int cols, int rows) {
    final id = _sessionId;
    if (id == null) return;
    final terminal = _currentTerminal();
    if (!_canResizeTerminal(terminal)) return;
    final resize = _terminalViewportController.resize(
      cols: cols,
      rows: rows,
      keyboardVisible: _keyboardVisible,
    );
    if (resize == null) return;
    _sendTerminalEnvelope(
      RelayEnvelope(
        type: 'terminal.viewport.resize',
        sessionId: id,
        payload: {'cols': resize.cols, 'rows': resize.rows},
      ),
    );
  }

  void _flushPendingTerminalResize({bool force = false}) {
    final id = _sessionId;
    if (id == null) return;
    final terminal = _currentTerminal();
    if (!_canResizeTerminal(terminal)) return;
    final resize = _terminalViewportController.flushPending(force: force);
    if (resize == null) return;
    _sendTerminalEnvelope(
      RelayEnvelope(
        type: 'terminal.viewport.resize',
        sessionId: id,
        payload: {'cols': resize.cols, 'rows': resize.rows},
      ),
    );
  }

  void _claimTerminalViewport({String? sessionId}) {
    final id = sessionId ?? _sessionId;
    if (id == null || id.trim().isEmpty) return;
    final terminal = _currentTerminal();
    if (terminal == null || !_canResizeTerminal(terminal)) return;
    _sendTerminalEnvelope(
      RelayEnvelope(type: 'terminal.viewport.claim', sessionId: id),
      terminal: terminal,
    );
  }

  void _releaseTerminalViewport({String? sessionId}) {
    final id = sessionId ?? _sessionId;
    if (id == null || id.trim().isEmpty) return;
    final terminal = _currentTerminal();
    if (terminal == null || !_canResizeTerminal(terminal)) return;
    _sendTerminalEnvelope(
      RelayEnvelope(type: 'terminal.viewport.release', sessionId: id),
      terminal: terminal,
    );
  }

  void _queueTerminalTyping(String data) {
    if (data.isEmpty) return;
    _terminalInputBatcher.add(data);
  }

  void _sendTerminalKey(String data) {
    if (data.isEmpty) return;
    _terminalInputBatcher.flush();
    _sendInputNow(data, source: 'key');
  }

  void _insertTerminalText(String text) {
    if (text.isEmpty) return;
    _terminalInputBatcher.flush();
    _sendInputNow(terminalPastePayload(text), source: 'insert');
  }

  void _sendInputNow(String data, {required String source}) {
    if (data.isEmpty) return;
    var id = _sessionId;
    if (id == null) {
      CoduxLog.debug(
        '[codux-flutter-input] no session, ensure terminal before input',
      );
      _ensureTerminalForSelectedProject();
      id = _sessionId;
    }
    if (id == null) {
      setState(() => _status = _t('terminal.createOrSelectFirst'));
      return;
    }
    _terminalInputSender.send(sessionId: id, data: data, source: source);
  }

  void _sendTerminalOutputAck(
    String sessionId,
    int outputSeq,
    int? bufferLength,
  ) {
    final payload = <String, Object>{'outputSeq': outputSeq};
    if (bufferLength != null) {
      payload['bufferLength'] = bufferLength;
    }
    _sendTerminalEnvelope(
      RelayEnvelope(
        type: 'terminal.output.ack',
        sessionId: sessionId,
        payload: payload,
      ),
    );
  }

  void _ackTerminalOutputIfNeeded(
    String sessionId,
    int? outputSeq,
    int? bufferLength,
  ) {
    if (outputSeq == null) return;
    _sendTerminalOutputAck(sessionId, outputSeq, bufferLength);
  }

  void _createTerminal([String? projectId, String layoutKind = 'split']) {
    final target =
        projectId ??
        _selectedProjectId ??
        (_projects.isNotEmpty ? _projects.first.id : null);
    if (target == null) {
      setState(() => _status = _t('project.noAvailable'));
      return;
    }
    if (_creatingTerminalProjectId == target) return;
    _remoteRuntime.setTerminalCreatingProject(target);
    setState(_syncRuntimeViewState);
    _clearTerminal();
    _send(
      RelayEnvelope(
        type: 'terminal.create',
        payload: {'projectId': target, 'command': '', 'layoutKind': layoutKind},
      ),
    );
  }

  bool _isAccessibleTerminal(TerminalInfo terminal) {
    return RemoteRuntimeStore.isAccessibleTerminal(terminal);
  }

  TerminalInfo? _currentTerminal() {
    return _remoteRuntime.activeTerminal();
  }

  RemoteTerminalScope? _terminalScopeForSession(
    String sessionId, {
    TerminalInfo? terminal,
  }) {
    return _remoteRuntime.terminalScopeForSession(
      sessionId,
      terminal: terminal,
    );
  }

  RelayEnvelope? _scopeTerminalEnvelope(
    RelayEnvelope message, {
    TerminalInfo? terminal,
  }) {
    final sessionId = message.sessionId?.trim();
    if (sessionId == null || sessionId.isEmpty) return message;
    final scope = _terminalScopeForSession(sessionId, terminal: terminal);
    if (scope == null) {
      CoduxLog.warn(
        '[codux-flutter-terminal] drop ${message.type} reason=missing-scope session=$sessionId',
      );
      return null;
    }
    return scopedTerminalEnvelope(message, scope);
  }

  bool _canResizeTerminal(TerminalInfo? terminal) {
    return terminal != null && _isAccessibleTerminal(terminal);
  }

  List<TerminalInfo> _currentProjectTerminals() {
    return _remoteRuntime.currentProjectTerminals();
  }

  void _selectTerminal(TerminalInfo terminal) {
    if (!_isAccessibleTerminal(terminal)) return;
    _terminalInputBatcher.flush();
    setState(() => _workspaceMode = 'terminal');
    final plan = _remoteRuntime.selectTerminal(terminal);
    _applyRuntimePlan(plan, reason: 'select-terminal');
    _focusTerminalViewSoon();
  }

  void _createCurrentProjectTerminal() {
    final projectId = _selectedProjectId;
    if (projectId == null) {
      _showToast(_t('project.selectFirst'));
      return;
    }
    setState(() => _workspaceMode = 'terminal');
    _createTerminal(projectId);
  }

  void _createCurrentProjectTabTerminal() {
    final projectId = _selectedProjectId;
    if (projectId == null) {
      _showToast(_t('project.selectFirst'));
      return;
    }
    setState(() => _workspaceMode = 'terminal');
    _createTerminal(projectId, 'tab');
  }

  void _closeCurrentTerminal() {
    final terminal = _currentTerminal();
    if (terminal == null || !_isAccessibleTerminal(terminal)) return;
    _closeTerminal(terminal);
  }

  void _closeTerminal(TerminalInfo terminal) {
    if (!_isAccessibleTerminal(terminal)) return;
    final plan = _remoteRuntime.removeTerminal(terminal.id);
    _applyRuntimePlan(plan, reason: 'close-terminal');
    _sendTerminalEnvelope(
      RelayEnvelope(type: 'terminal.close', sessionId: terminal.id),
      terminal: terminal,
    );
  }

  Future<void> _openTerminalSwitcher() async {
    if (_showTerminalSwitcher) return;
    _requestWorktreeList(loading: _worktrees.isEmpty);
    await _pushCupertinoPage(() {
      _showTerminalSwitcher = true;
    });
  }

  void _closeTerminalSwitcher() {
    _popCupertinoPage(() {
      _showTerminalSwitcher = false;
    });
  }

  void _selectTerminalFromSwitcher(TerminalInfo terminal) {
    _selectTerminal(terminal);
    _closeTerminalSwitcher();
  }

  void _selectWorktree(RemoteWorktreeInfo worktree) {
    final project = _selectedProject;
    if (project == null) {
      _showToast(_t('project.selectFirst'));
      return;
    }
    setState(() => _worktreeListLoading = true);
    _send(_worktreeController.selectEnvelope(project, worktree));
  }

  Future<void> _createWorktree() async {
    final project = _selectedProject;
    if (project == null || project.path == null || project.path!.isEmpty) {
      _showToast(_t('project.selectPathFirst'));
      return;
    }
    final branchOptions = _worktreeCreatorBranchOptions();
    final request = await showDialog<WorktreeCreateDraft>(
      context: context,
      builder: (ctx) => WorktreeCreateDialog(
        title: _t('worktree.new'),
        baseBranchLabel: _t('worktree.baseBranch'),
        nameLabel: _t('worktree.name'),
        cancelLabel: _t('app.cancel'),
        createLabel: _t('common.create'),
        branchOptions: branchOptions,
        initialBaseBranch: _worktreeCreatorDefaultBaseBranch(branchOptions),
        initialName: defaultWorktreeName(),
      ),
    );
    if (request == null) return;
    if (request.baseBranch.isEmpty) {
      _showToast(_t('worktree.baseBranchRequired'));
      return;
    }
    if (request.name.isEmpty) {
      _showToast(_t('worktree.nameRequired'));
      return;
    }
    setState(() => _worktreeListLoading = true);
    _send(
      _worktreeController.createEnvelope(
        project: project,
        baseBranch: request.baseBranch,
        name: request.name,
      ),
    );
  }

  List<String> _worktreeCreatorBranchOptions() {
    return worktreeBranchOptions(
      defaultBaseBranch: _defaultWorktreeBaseBranch,
      baseBranches: _worktreeBaseBranches,
      worktrees: _worktrees,
    );
  }

  String _worktreeCreatorDefaultBaseBranch(List<String> options) {
    return defaultWorktreeBaseBranch(
      preferred: _defaultWorktreeBaseBranch,
      options: options,
    );
  }

  Future<void> _mergeWorktree(RemoteWorktreeInfo worktree) async {
    final confirmed = await _confirmWorktreeAction(
      title: _t('worktree.merge'),
      message: _t(
        'worktree.mergeConfirm',
        params: {'name': _worktreeTitle(worktree)},
      ),
      destructive: false,
    );
    if (!confirmed) return;
    _sendWorktreeOperation('worktree.merge', worktree);
  }

  Future<void> _deleteWorktree(RemoteWorktreeInfo worktree) async {
    final confirmed = await _confirmWorktreeAction(
      title: _t('worktree.delete'),
      message: _t(
        'worktree.deleteConfirm',
        params: {'name': _worktreeTitle(worktree)},
      ),
      destructive: true,
    );
    if (!confirmed) return;
    _sendWorktreeOperation('worktree.delete', worktree);
  }

  void _sendWorktreeOperation(String type, RemoteWorktreeInfo worktree) {
    final project = _selectedProject;
    if (project == null || project.path == null || project.path!.isEmpty) {
      _showToast(_t('project.selectPathFirst'));
      return;
    }
    setState(() => _worktreeListLoading = true);
    final envelope = type == 'worktree.delete'
        ? _worktreeController.deleteEnvelope(project, worktree)
        : _worktreeController.mergeEnvelope(project, worktree);
    _send(envelope);
  }

  Future<bool> _confirmWorktreeAction({
    required String title,
    required String message,
    required bool destructive,
  }) async {
    return await showDialog<bool>(
          context: context,
          builder: (ctx) => WorktreeActionDialog(
            title: title,
            message: message,
            cancelLabel: _t('app.cancel'),
            destructive: destructive,
          ),
        ) ??
        false;
  }

  String _worktreeTitle(RemoteWorktreeInfo worktree) {
    return worktreeTitle(worktree);
  }

  Future<void> _refreshDeviceList() async {
    final device = _activeDevice;
    if (device == null) return;
    if (!_isConnected) {
      _connect(device);
      await Future<void>.delayed(const Duration(milliseconds: 350));
      return;
    }
    _sendTransportPing();
    _sendHostInfoRequest(force: true);
    _requestProjectList(resetRetry: true);
    _requestTerminalList(resetRetry: true);
    await Future<void>.delayed(const Duration(milliseconds: 350));
  }

  void _refreshLists() {
    _sendTransportPing();
    _sendHostInfoRequest(force: true);
    _requestProjectList(resetRetry: true);
    _requestTerminalList(resetRetry: true);
    _requestGitStatus();
  }

  void _rebuildCurrentTerminal() {
    final projectId = _selectedProjectId;
    if (projectId == null) {
      _showToast(_t('project.selectFirst'));
      return;
    }
    String? closingSessionId;
    TerminalInfo? closingTerminal;
    final current = _currentTerminal();
    final projectTerminals = _terminals
        .where(
          (terminal) =>
              terminal.projectId == projectId &&
              _isAccessibleTerminal(terminal),
        )
        .toList();
    if (current != null &&
        current.projectId == projectId &&
        _isAccessibleTerminal(current)) {
      closingSessionId = current.id;
      closingTerminal = current;
    } else if (projectTerminals.isNotEmpty) {
      closingTerminal = projectTerminals.first;
      closingSessionId = closingTerminal.id;
    }
    final shouldCreateReplacement = projectTerminals.length > 1;
    if (closingSessionId != null) {
      final plan = _remoteRuntime.removeTerminal(closingSessionId);
      _applyRuntimePlan(plan, reason: 'rebuild-terminal');
      _sendTerminalEnvelope(
        RelayEnvelope(type: 'terminal.close', sessionId: closingSessionId),
        terminal: closingTerminal,
      );
    } else {
      _clearTerminal();
    }
    if (shouldCreateReplacement) {
      _createTerminal(projectId);
    }
    _showToast(_t('terminal.rebuilding'));
  }

  void _ensureTerminalForSelectedProject() {
    final plan = _remoteRuntime.ensureTerminalForSelectedProject(
      terminalVisible: _showTerminal && _workspaceMode == 'terminal',
      terminalListLoaded: _terminalListLoaded,
    );
    _applyRuntimePlan(plan, reason: 'missing-terminal');
  }

  void _requestProjectEdit() {
    final project = _selectedProject;
    if (project == null) {
      _showSnack(_t('project.selectFirst'));
      return;
    }
    final draft = _projectController.editDraft(project);
    setState(() {
      _applyProjectFormDraft(draft);
      _showProjectForm = true;
    });
  }

  void _requestProjectAdd() {
    final draft = _projectController.addDraft();
    setState(() {
      _applyProjectFormDraft(draft);
      _showProjectForm = true;
    });
  }

  void _chooseProjectFormPath() {
    _filePickerMode = 'projectForm';
    final current = _projectPathController.text.trim();
    _openRemoteFilePicker(current.isEmpty ? null : current);
  }

  void _saveProjectForm() {
    final plan = _projectController.savePlan(
      mode: _projectFormMode,
      path: _projectPathController.text,
      name: _projectNameController.text,
      selectedProject: _selectedProject,
    );
    if (!plan.valid) {
      _showToast(_t('project.selectPathFirst'));
      return;
    }
    _send(plan.envelope!);
    setState(() => _showProjectForm = false);
    _showToast(_t('project.saveSubmitted'));
  }

  void _openRemoteFilePicker([String? path]) {
    _filePickerTimeoutTimer?.cancel();
    setState(() {
      _showFilePicker = true;
      _filePickerLoading = true;
      _filePickerPath = path ?? _filePickerPath;
    });
    _filePickerTimeoutTimer = Timer(const Duration(seconds: 8), () {
      if (!mounted || !_filePickerLoading) return;
      setState(() => _filePickerLoading = false);
      _showToast(_t('remote.dirTimeout'));
    });
    _send(_projectController.filePickerListEnvelope(path));
  }

  void _selectRemoteProjectFolder(RemoteFileEntry entry) {
    if (_filePickerMode == 'projectForm') {
      final selection = _projectController.selectFolder(
        entry: entry,
        currentName: _projectNameController.text,
      );
      setState(() {
        _projectPathController.text = selection.path;
        _projectNameController.text = selection.name;
        _showFilePicker = false;
      });
      return;
    }
    setState(() => _showFilePicker = false);
  }

  void _applyProjectFormDraft(ProjectFormDraft draft) {
    _projectFormMode = draft.mode;
    _projectNameController.text = draft.name;
    _projectPathController.text = draft.path;
  }

  void _requestProjectRemove() {
    final project = _selectedProject;
    if (project == null) {
      _showSnack(_t('project.selectFirst'));
      return;
    }
    _send(_projectController.removeEnvelope(project));
    _showToast(_t('project.removeRequested'));
  }

  void _requestAIStats() {
    final project = _selectedProject;
    if (project == null) {
      _showToast(_t('project.selectFirst'));
      return;
    }
    setState(() {
      _workspaceMode = 'stats';
      _aiStatsLoading = true;
    });
    _send(_projectController.aiStatsEnvelope(project));
  }

  void _requestGitStatus() {
    final project = _selectedProject;
    if (!_remoteProtocolReady || project == null) return;
    _send(_projectController.gitStatusEnvelope(project));
  }

  void _syncTerminalToSelectedProject({bool requestListIfMissing = true}) {
    final plan = _remoteRuntime.ensureTerminalForSelectedProject(
      terminalVisible: _showTerminal && _workspaceMode == 'terminal',
      terminalListLoaded: requestListIfMissing && _terminalListLoaded,
    );
    _applyRuntimePlan(plan, reason: 'missing-terminal');
  }

  void _showTerminalMode() {
    setState(() => _workspaceMode = 'terminal');
    _syncTerminalToSelectedProject();
    _requestGitStatus();
    _focusTerminalViewSoon();
  }

  void _showFilesMode() {
    final project = _selectedProject;
    if (project == null) {
      _showToast(_t('project.selectFirst'));
      return;
    }
    final targetPath = _projectFileController.pathForProject(
      project,
      currentPath: _projectFilesPath,
    );
    setState(() {
      _workspaceMode = 'files';
    });
    _requestGitStatus();
    _requestProjectFiles(targetPath);
  }

  void _requestProjectFiles([String? path]) {
    final project = _selectedProject;
    final target = path ?? project?.path;
    if (target == null || target.isEmpty) {
      _showToast(_t('project.currentNoDir'));
      return;
    }
    setState(() {
      _projectFilesLoading = true;
      _projectFilesPath = target;
      if (project != null) {
        _projectFileController.remember(projectId: project.id, path: target);
      }
    });
    _send(_projectFileController.listEnvelope(target));
  }

  Future<void> _copyProjectFilePath(RemoteFileEntry entry) async {
    final message = AppPreferences.of(context).t('file.pathCopied');
    await Clipboard.setData(ClipboardData(text: entry.path));
    _showToast(message);
  }

  Future<void> _renameProjectFile(RemoteFileEntry entry) async {
    final prefs = AppPreferences.of(context);
    final nextName = await showDialog<String>(
      context: context,
      builder: (ctx) => FileRenameDialog(
        title: prefs.t('file.renameTitle'),
        label: prefs.t('file.renameLabel'),
        cancelLabel: prefs.t('file.cancel'),
        saveLabel: prefs.t('file.save'),
        initialName: entry.name,
      ),
    );
    if (nextName == null) return;
    final plan = _projectFileController.renamePlan(entry, nextName);
    if (plan == null) return;
    if (!plan.valid) {
      _showToast(prefs.t('file.nameInvalid'));
      return;
    }
    _send(plan.envelope!);
  }

  Future<void> _deleteProjectFile(RemoteFileEntry entry) async {
    final prefs = AppPreferences.of(context);
    final confirmed = await showDialog<bool>(
      context: context,
      builder: (ctx) => FileDeleteDialog(
        title: prefs.t('file.deleteTitle'),
        message: prefs.t('file.deleteConfirm', params: {'name': entry.name}),
        cancelLabel: prefs.t('file.cancel'),
        deleteLabel: prefs.t('file.menuDelete'),
      ),
    );
    if (confirmed != true) return;
    _send(_projectFileController.deleteEnvelope(entry));
  }

  void _openFileLocation(String path) {
    if (_showFilePicker) {
      _openRemoteFilePicker(path);
      return;
    }
    _requestProjectFiles(path);
  }

  void _requestFileRead(RemoteFileEntry entry) {
    if (entry.isDirectory) return;
    final fileState = _projectFileController.beginReadState(entry);
    setState(() {
      _applyFileEditorState(fileState);
    });
    _send(_projectFileController.readEnvelope(entry));
  }

  void _applyFileListState(RemoteFileListState state) {
    if (state.isProjectFiles) {
      setState(() {
        _projectFilesPath = state.path;
        _projectFilesParent = state.parent;
        _projectFileEntries = state.entries;
        _projectFilesLoading = false;
        final projectId = _selectedProjectId;
        if (projectId != null && state.path.isNotEmpty) {
          _projectFileController.remember(
            projectId: projectId,
            path: state.path,
          );
        }
      });
      return;
    }
    setState(() {
      _filePickerPath = state.path;
      _filePickerParent = state.parent;
      _filePickerEntries = state.entries;
      _filePickerLoading = false;
      _filePickerTimeoutTimer?.cancel();
      _showFilePicker = true;
    });
  }

  void _applyFileEditorState(RemoteFileEditorState state) {
    _editingFilePath = state.path;
    _fileEditorController.text = state.content;
    _fileEditorController.highlightEnabled = state.highlightEnabled;
    _fileEditorLoading = state.loading;
    _fileEditorSaving = state.saving;
    _fileEditorEditing = state.editing;
    _fileEditorEditable = state.editable;
  }

  void _saveEditingFile() {
    final path = _editingFilePath;
    if (path == null || _fileEditorSaving || !_fileEditorEditing) return;
    setState(() => _fileEditorSaving = true);
    _send(
      _projectFileController.writeEnvelope(
        path: path,
        content: _fileEditorController.text,
      ),
    );
  }

  void _focusTerminalSoon() {
    Future<void>.delayed(const Duration(milliseconds: 80), () {
      if (!mounted) return;
      _nativeTerminalController?.focusKeyboard();
    });
  }

  void _toggleTerminalKeyboard() {
    if (_keyboardVisible) {
      _nativeTerminalController?.hideKeyboard();
      return;
    }
    _focusTerminalSoon();
  }

  void _focusTerminalViewSoon() {
    Future<void>.delayed(const Duration(milliseconds: 80), () {
      if (!mounted) return;
      _nativeTerminalController?.requestResize();
    });
  }

  Future<void> _removeDevice(StoredDevice device) async {
    final result = _deviceController.remove(
      devices: _devices,
      activeDevice: _activeDevice,
      device: device,
    );
    if (result.removedActive) {
      _shouldReconnect = false;
      _transportConnected = false;
      unawaited(_closeActiveTransport());
      _clearLatencyProbe();
    }
    await _saveDevices(result.state.devices);
    if (result.state.devices.isEmpty) {
      setState(() => _showTerminal = false);
    }
  }

  void _openDeviceTerminal(StoredDevice device) {
    if (device.deviceId != _activeDevice?.deviceId || !_isConnected) return;
    unawaited(
      _pushCupertinoPage(() {
        _showTerminal = true;
        _workspaceMode = 'terminal';
        _setTerminalBufferLoading(false);
      }).then((_) {
        if (!mounted) return;
        _ensureTerminalForSelectedProject();
      }),
    );
    if (!_projectListLoaded) {
      _requestProjectList(resetRetry: true);
    }
    if (!_terminalListLoaded) {
      _requestTerminalList(resetRetry: true);
    }
    _ensureTerminalForSelectedProject();
    _focusTerminalViewSoon();
  }

  Future<void> _editDevice(StoredDevice device) async {
    final next = await showDialog<StoredDevice>(
      context: context,
      builder: (ctx) => DeviceEditDialog(
        device: device,
        title: _t('device.editTitle'),
        nameLabel: _t('device.nameLabel'),
        cancelLabel: _t('app.cancel'),
        saveLabel: _t('common.save'),
      ),
    );
    if (next == null) return;
    final nextState = _deviceController.replace(
      devices: _devices,
      device: next,
      activeDevice: _activeDevice,
    );
    await _saveDevices(nextState.devices);
    if (_activeDevice?.deviceId == next.deviceId) {
      _connect(next, true);
    }
  }

  void _onProjectSelected(ProjectInfo project) {
    final projectChanged = _selectedProjectId != project.id;
    final resetTerminal = projectChanged && _workspaceMode == 'terminal';
    setState(() {
      _currentAIStats = null;
      _projectFileEntries = [];
      _projectFilesPath = project.path ?? '';
      _projectFilesParent = null;
      if (projectChanged) {
        _projectFileController.forget(project.id);
        _worktrees = [];
        _worktreeBaseBranches = [];
        _defaultWorktreeBaseBranch = null;
        _selectedWorktreeId = null;
      }
    });
    final plan = _remoteRuntime.userSelectProject(
      project: project,
      terminalVisible: resetTerminal,
    );
    _applyRuntimePlan(plan, reason: 'user-select');
    _requestWorktreeList(loading: _showTerminalSwitcher);
    if (_workspaceMode == 'stats') {
      _requestAIStats();
      return;
    }
    if (_workspaceMode == 'files') {
      _requestProjectFiles(project.path);
      return;
    }
    if (resetTerminal) {
      return;
    }
    final current = _terminals.any(
      (item) =>
          item.id == _sessionId &&
          item.projectId == project.id &&
          _isAccessibleTerminal(item),
    );
    if (!current) {
      _ensureTerminalForSelectedProject();
    }
  }

  Future<void> _pasteToTerminal() async {
    final data = await Clipboard.getData(Clipboard.kTextPlain);
    if (data?.text?.isNotEmpty == true) {
      _insertTerminalText(data!.text!);
    }
  }

  Future<void> _copyTerminalSelection() async {
    final prefs = AppPreferences.of(context);
    final copied = await _nativeTerminalController?.copySelection() ?? false;
    _showSnack(
      copied ? prefs.t('toolbar.copyDone') : prefs.t('toolbar.copyEmpty'),
    );
  }

  Future<void> _startVoiceInput() async {
    if (_showVoiceOverlay) return;
    setState(() => _showVoiceOverlay = true);
  }

  Future<void> _chooseUploadForTerminal() async {
    if (_terminalUploadLoading) return;
    if (!_canUploadOverCurrentPath) {
      _showSnack(_t('upload.directRequired'));
      setState(() => _status = _t('upload.directRequired'));
      return;
    }
    final prefs = AppPreferences.of(context);
    final source = await showModalBottomSheet<TerminalUploadSource>(
      context: context,
      backgroundColor: AppColors.bgElevated,
      barrierColor: AppColors.backdrop,
      shape: const RoundedRectangleBorder(
        borderRadius: BorderRadius.vertical(top: Radius.circular(AppRadius.lg)),
      ),
      builder: (context) => TerminalUploadSourceSheet(
        fileLabel: prefs.t('upload.chooseFile'),
        imageLabel: prefs.t('upload.chooseImage'),
      ),
    );
    if (source == null || !mounted) return;
    await _uploadPickedFileToTerminal(source);
  }

  Future<void> _uploadPickedFileToTerminal(TerminalUploadSource source) async {
    if (_terminalUploadLoading) return;
    if (!_canUploadOverCurrentPath) {
      _showSnack(_t('upload.directRequired'));
      setState(() => _status = _t('upload.directRequired'));
      return;
    }
    final id = _sessionId;
    if (id == null) {
      setState(() => _status = _t('terminal.createOrSelectFirst'));
      return;
    }
    final result = await FilePicker.pickFiles(
      type: source == TerminalUploadSource.image
          ? FileType.image
          : FileType.any,
      allowMultiple: false,
      withData: true,
    );
    final files = result?.files;
    final picked = files == null || files.isEmpty ? null : files.single;
    if (picked == null) return;
    if (picked.size > 20 * 1024 * 1024) {
      _showSnack(_t('upload.fileTooLarge'));
      return;
    }
    final bytes =
        picked.bytes ??
        (picked.path == null ? null : await File(picked.path!).readAsBytes());
    if (bytes == null) {
      _showSnack(_t('upload.fileReadFailed'));
      return;
    }
    if (bytes.isEmpty) return;
    if (!_canUploadOverCurrentPath) {
      _showSnack(_t('upload.directRequired'));
      setState(() => _status = _t('upload.directRequired'));
      return;
    }
    _terminalUploadCompletion?.completeError(
      StateError('Terminal upload superseded'),
    );
    final uploadCompletion = Completer<void>();
    _terminalUploadCompletion = uploadCompletion;
    final uploadingMessage = _t(terminalUploadUploadingKey(source));
    setState(() {
      _terminalUploadLoading = true;
      _terminalUploadStatus = uploadingMessage;
      _status = _terminalUploadStatus;
    });
    try {
      await _terminalUploadSender.uploadFile(
        sessionId: id,
        name: picked.name,
        mime: terminalUploadMime(
          picked.name,
          image: source == TerminalUploadSource.image,
        ),
        bytes: bytes,
        kind: terminalUploadKind(source),
        onProgress: (progress) {
          if (!mounted) return;
          final message = '$uploadingMessage ${progress.percent}%';
          setState(() {
            _terminalUploadStatus = message;
            _status = message;
          });
        },
      );
      if (!mounted) return;
      final insertingMessage = _t(terminalUploadInsertingKey(source));
      setState(() {
        _terminalUploadStatus = insertingMessage;
        _status = insertingMessage;
      });
      await uploadCompletion.future.timeout(const Duration(seconds: 30));
    } catch (error) {
      CoduxLog.warn('[codux-flutter-upload] upload failed: $error');
      if (!mounted) return;
      if (_terminalUploadCompletion == uploadCompletion) {
        _terminalUploadCompletion = null;
      }
      setState(() {
        _terminalUploadLoading = false;
        _terminalUploadStatus = '';
        _status = '${_t('remote.error')}: $error';
      });
    }
  }

  bool get _canUploadOverCurrentPath =>
      _isConnected && _connectionPath == 'direct';

  Future<void> _checkUpdate() async {
    setState(() {
      _status = _t('update.checking');
      _blockingLoadingMessage = _t('update.loading');
    });
    try {
      final result = await _updateCheckService.check();
      if (!result.available) {
        final toastKey = result.toastKey;
        if (toastKey != null && toastKey.isNotEmpty) {
          _showToast(_t(toastKey, params: result.toastParams));
        }
        return;
      }
      if (!mounted) return;
      showDialog<void>(
        context: context,
        builder: (ctx) => UpdateAvailableDialog(
          title: _t(
            'update.foundTitle',
            params: {'version': result.version ?? ''},
          ),
          body: _t(
            result.isIos ? 'update.foundBodyAppStore' : 'update.foundBody',
            params: {'version': result.currentVersion},
          ),
          laterLabel: _t('common.later'),
          actionLabel: result.isIos
              ? _t('common.openAppStore')
              : _t('common.openGithub'),
          onOpen: () {
            if (result.url.isNotEmpty) _openUrl(result.url);
          },
        ),
      );
    } catch (error) {
      _showToast(_t('update.failed', params: {'reason': '$error'}));
    } finally {
      if (mounted) setState(() => _blockingLoadingMessage = null);
    }
  }

  Future<void> _showAboutDialogNow() async {
    final info = await PackageInfo.fromPlatform();
    if (!mounted) return;
    showDialog<void>(
      context: context,
      builder: (ctx) => CoduxAboutDialog(
        title: _t('app.about'),
        body: _t('app.aboutText'),
        versionText: 'v${info.version}+${info.buildNumber}',
        closeLabel: _t('app.close'),
        onOpenGithub: () => _openUrl('https://github.com/duxweb/codux-flutter'),
      ),
    );
  }

  Future<void> _openUrl(String value) async {
    final uri = Uri.parse(value);
    if (!await launchUrl(uri, mode: LaunchMode.externalApplication)) {
      await launchUrl(uri);
    }
  }

  void _showSnack(String message) => _showToast(message);

  void _showToast(String message) {
    if (!mounted) return;
    _toastTimer?.cancel();
    setState(() => _toastMessage = message);
    _toastTimer = Timer(const Duration(seconds: 2), () {
      if (mounted) setState(() => _toastMessage = null);
    });
  }

  void _showLogViewer() {
    showDialog<void>(
      context: context,
      builder: (ctx) => DebugLogDialog(
        title: _t('app.debugLogs'),
        emptyLabel: _t('logs.empty'),
        clearLabel: _t('logs.clear'),
        copyLabel: _t('logs.copy'),
        exportLabel: _t('logs.export'),
        closeLabel: _t('app.close'),
        onCopy: (text) async {
          await Clipboard.setData(ClipboardData(text: text));
          if (mounted) _showToast(_t('logs.copied'));
        },
        onExport: _exportLogs,
      ),
    );
  }

  Future<void> _exportLogs(String text) async {
    try {
      await _logExportService.export(text, shareText: _t('logs.shareText'));
      if (mounted) _showToast(_t('logs.exported'));
    } catch (error) {
      if (mounted) _showToast('${_t('logs.exportFailed')}: $error');
    }
  }

  void _confirmRemoveDevice(StoredDevice device) {
    showDialog<bool>(
      context: context,
      builder: (ctx) => DeviceRemoveDialog(
        title: _t('app.removeDevice'),
        message: _t(
          'app.removeDeviceConfirm',
          params: {'name': device.hostName ?? device.name},
        ),
        cancelLabel: _t('app.cancel'),
        removeLabel: _t('app.remove'),
      ),
    ).then((confirmed) {
      if (confirmed == true) _removeDevice(device);
    });
  }

  void _handleBackNavigation() {
    if (_editingFilePath != null) {
      setState(() {
        _editingFilePath = null;
        _fileEditorLoading = false;
        _fileEditorSaving = false;
        _fileEditorEditing = false;
      });
      return;
    }
    if (_showScanner) {
      setState(() => _showScanner = false);
      return;
    }
    if (_showFilePicker) {
      _filePickerTimeoutTimer?.cancel();
      setState(() => _showFilePicker = false);
      return;
    }
    if (_showProjectForm) {
      setState(() => _showProjectForm = false);
      return;
    }
    if (_showSettings) {
      _popCupertinoPage(() {
        _showSettings = false;
      });
      return;
    }
    if (_showTerminalSwitcher) {
      _popCupertinoPage(() {
        _showTerminalSwitcher = false;
      });
      return;
    }
    if (_pendingPairing != null) {
      _cancelPairing();
      return;
    }
    if (_showTerminal) {
      _popCupertinoPage(() {
        _showTerminal = false;
        _workspaceMode = 'terminal';
      });
      return;
    }
    _disconnectTransport(status: _t('app.disconnected'), closeTerminal: true);
    SystemNavigator.pop();
  }

  void _handleWorkspaceEdgeDragStart(DragStartDetails details) {
    if (!Platform.isIOS ||
        (!_showTerminal && !_showSettings && !_showTerminalSwitcher)) {
      return;
    }
    final edgeWidth = MediaQuery.viewPaddingOf(context).left + 24.0;
    final startX = details.localPosition.dx;
    if (startX > edgeWidth) {
      _edgeBackDragStartX = null;
      return;
    }
    _edgeBackDragStartX = startX;
    _edgeBackDragDeltaX = 0;
    _edgeBackDragDeltaY = 0;
    _edgeBackController.stop();
  }

  void _handleWorkspaceEdgeDragUpdate(DragUpdateDetails details) {
    if (_edgeBackDragStartX == null) return;
    _edgeBackDragDeltaX += details.delta.dx;
    _edgeBackDragDeltaY += details.delta.dy;
    final width = MediaQuery.sizeOf(context).width;
    if (width <= 0) return;
    _edgeBackController.value = (_edgeBackDragDeltaX / width).clamp(0.0, 1.0);
  }

  void _handleWorkspaceEdgeDragEnd(DragEndDetails details) {
    if (_edgeBackDragStartX == null) return;
    final dragX = _edgeBackDragDeltaX;
    final dragY = _edgeBackDragDeltaY.abs();
    final velocityX = details.velocity.pixelsPerSecond.dx;
    _edgeBackDragStartX = null;
    _edgeBackDragDeltaX = 0;
    _edgeBackDragDeltaY = 0;
    final width = MediaQuery.sizeOf(context).width;
    final progress = width <= 0 ? 0.0 : (dragX / width).clamp(0.0, 1.0);
    final shouldComplete =
        dragX > 72 &&
        dragX > dragY * 1.4 &&
        (velocityX > 260 || progress > 0.34);
    if (shouldComplete) {
      unawaited(_completeCupertinoPageBack());
    } else {
      unawaited(
        _edgeBackController.animateBack(
          0,
          duration: const Duration(milliseconds: 180),
          curve: Curves.easeOutCubic,
        ),
      );
    }
  }

  Future<void> _completeCupertinoPageBack() async {
    await _edgeBackController.animateTo(
      1,
      duration: const Duration(milliseconds: 180),
      curve: Curves.easeOutCubic,
    );
    if (!mounted) return;
    setState(() {
      if (_showSettings) {
        _showSettings = false;
      } else if (_showTerminalSwitcher) {
        _showTerminalSwitcher = false;
      } else {
        _showTerminal = false;
      }
      _workspaceMode = 'terminal';
    });
    _edgeBackController.value = 0;
  }

  Future<void> _pushCupertinoPage(VoidCallback updateState) async {
    _edgeBackController.value = 1;
    setState(updateState);
    await _edgeBackController.animateBack(
      0,
      duration: const Duration(milliseconds: 260),
      curve: Curves.easeOutCubic,
    );
  }

  Future<void> _popCupertinoPage(VoidCallback updateState) async {
    if (!Platform.isIOS) {
      setState(updateState);
      return;
    }
    await _edgeBackController.animateTo(
      1,
      duration: const Duration(milliseconds: 220),
      curve: Curves.easeOutCubic,
    );
    if (!mounted) return;
    setState(updateState);
    _edgeBackController.value = 0;
  }

  void _cancelWorkspaceEdgeBack() {
    _edgeBackDragStartX = null;
    _edgeBackDragDeltaX = 0;
    _edgeBackDragDeltaY = 0;
    unawaited(
      _edgeBackController.animateBack(
        0,
        duration: const Duration(milliseconds: 180),
        curve: Curves.easeOutCubic,
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    final media = MediaQuery.of(context);
    final topInset = media.viewPadding.top;
    final bottomInset = media.viewPadding.bottom;
    final leftInset = media.viewPadding.left;
    _keyboardVisible = media.viewInsets.bottom > bottomInset + 8.0;

    final deviceHome = _buildDeviceHome(topInset, bottomInset);
    final settingsPage = _buildSettingsPage(topInset, bottomInset);
    final switcherPage = _buildTerminalSwitcherPage(topInset, bottomInset);
    final workspacePage = _buildWorkspace(topInset, bottomInset);

    return CoduxHomeShell(
      metrics: CoduxHomeShellMetrics(
        topInset: topInset,
        bottomInset: bottomInset,
        leftInset: leftInset,
        edgeBackAnimation: _edgeBackController,
      ),
      pages: CoduxHomeShellPages(
        deviceHome: deviceHome,
        settingsPage: settingsPage,
        switcherPage: switcherPage,
        workspacePage: workspacePage,
      ),
      state: CoduxHomeShellState(
        showSettings: _showSettings,
        showTerminal: _showTerminal,
        showTerminalSwitcher: _showTerminalSwitcher,
      ),
      overlays: CoduxHomeOverlayState(
        showScanner: _showScanner,
        pendingPairing: _pendingPairing,
        pairingInFlight: _pairingInFlight,
        pairingError: _pairingError,
        showProjectForm: _showProjectForm,
        projectFormTitle: _projectFormMode == ProjectFormMode.edit
            ? _t('project.edit')
            : _t('project.add'),
        projectNameController: _projectNameController,
        projectPathController: _projectPathController,
        showFilePicker: _showFilePicker,
        filePickerTitle: _t('project.pathLabel'),
        filePickerPath: _filePickerPath,
        filePickerParent: _filePickerParent,
        filePickerEntries: _filePickerEntries,
        filePickerLoading: _filePickerLoading,
        showVoiceOverlay: _showVoiceOverlay,
        voiceService: _voiceService,
        editingFilePath: _editingFilePath,
        fileEditorController: _fileEditorController,
        fileEditorLoading: _fileEditorLoading,
        fileEditorSaving: _fileEditorSaving,
        fileEditorEditing: _fileEditorEditing,
        fileEditorEditable: _fileEditorEditable,
        blockingLoadingMessage: _blockingLoadingMessage,
        toastMessage: _toastMessage,
      ),
      actions: CoduxHomeShellActions(
        onBack: _handleBackNavigation,
        onEdgeDragStart: _handleWorkspaceEdgeDragStart,
        onEdgeDragUpdate: _handleWorkspaceEdgeDragUpdate,
        onEdgeDragEnd: _handleWorkspaceEdgeDragEnd,
        onEdgeDragCancel: _cancelWorkspaceEdgeBack,
        onScannerDetected: _handleScannedPayload,
        onCloseScanner: () => setState(() => _showScanner = false),
        onCancelPairing: _cancelPairing,
        onConfirmPairing: _confirmPairing,
        onCloseProjectForm: () => setState(() => _showProjectForm = false),
        onChooseProjectPath: _chooseProjectFormPath,
        onSaveProjectForm: _saveProjectForm,
        onCloseFilePicker: () {
          _filePickerTimeoutTimer?.cancel();
          setState(() => _showFilePicker = false);
        },
        onOpenFilePickerPath: _openRemoteFilePicker,
        onSelectFilePickerEntry: _selectRemoteProjectFolder,
        onOpenFilePickerHome: () => _openRemoteFilePicker(),
        onOpenFilePickerRoot: () => _openRemoteFilePicker('/'),
        onOpenFilePickerVolumes: () => _openRemoteFilePicker('/Volumes'),
        onCloseVoice: () => setState(() => _showVoiceOverlay = false),
        onSendVoiceText: (text) {
          _insertTerminalText(text);
          setState(() => _showVoiceOverlay = false);
        },
        onCloseFileEditor: () => setState(() => _editingFilePath = null),
        onEditFile: () => setState(() => _fileEditorEditing = true),
        onSaveFile: _saveEditingFile,
      ),
    );
  }

  Widget _buildDeviceHome(double topInset, double bottomInset) {
    return DeviceHomeScreen(
      devices: _devices,
      activeDeviceId: _activeDevice?.deviceId,
      ready: _isDeviceListConnected,
      status: _deviceListStatusText,
      latencyMs: _isConnected ? _latencyMs : null,
      topInset: topInset,
      bottomInset: bottomInset,
      onOpen: _openDeviceTerminal,
      onConnect: (device) => _connect(device),
      onAdd: () => setState(() => _showScanner = true),
      onEdit: _editDevice,
      onDelete: _confirmRemoveDevice,
      onRefresh: _refreshDeviceList,
      onSettings: () => _pushCupertinoPage(() {
        _showSettings = true;
      }),
      onLogs: _showLogViewer,
      onCheckUpdate: _checkUpdate,
      onAbout: _showAboutDialogNow,
    );
  }

  Widget _buildSettingsPage(double topInset, double bottomInset) {
    final prefs = AppPreferences.of(context);
    return SettingsScreen(
      nameController: _settingsNameController,
      detectedName: _detectedDeviceName,
      topInset: topInset,
      bottomInset: bottomInset,
      currentAccent: prefs.accent,
      currentLocale: prefs.locale,
      currentLogLevel: _settings.logLevel,
      onChangeAccent: (next) {
        widget.onChangeAccent(next);
        setState(() => _settings = _settings.copyWith(accentId: next.id));
      },
      onChangeLocale: (next) {
        widget.onChangeLocale(next);
        setState(() => _settings = _settings.copyWith(localeId: next.id));
      },
      onChangeLogLevel: (next) {
        CoduxLog.setLevelName(next);
        _nativeTerminalController?.setLogLevel(CoduxLog.nativeLevelName);
        setState(() => _settings = _settings.copyWith(logLevel: next));
      },
      onUseDetectedName: () =>
          setState(() => _settingsNameController.text = _detectedDeviceName),
      onSave: _saveSettings,
      onBack: () => _popCupertinoPage(() {
        _showSettings = false;
      }),
    );
  }

  Widget _buildTerminalSwitcherPage(double topInset, double bottomInset) {
    return TerminalSwitcherScreen(
      topInset: topInset,
      bottomInset: bottomInset,
      terminals: _currentProjectTerminals(),
      worktrees: _worktrees,
      activeTerminalId: _sessionId,
      selectedWorktreeId: _selectedWorktreeId,
      loadingWorktrees: _worktreeListLoading,
      onBack: _closeTerminalSwitcher,
      onSelectTerminal: _selectTerminalFromSwitcher,
      onCreateSplit: _createCurrentProjectTerminal,
      onCreateTab: _createCurrentProjectTabTerminal,
      onCloseTerminal: _closeTerminal,
      onSelectWorktree: _selectWorktree,
      onCreateWorktree: _createWorktree,
      onMergeWorktree: _mergeWorktree,
      onDeleteWorktree: _deleteWorktree,
      onRefreshWorktrees: () => _requestWorktreeList(loading: true),
    );
  }

  Widget _buildWorkspace(double topInset, double bottomInset) {
    final terminalBody = RemoteTerminalPane(
      connected: _isConnected,
      showTerminal: _hasShownTerminal,
      hasDevice: _activeDevice != null,
      status: _status,
      workspaceMode: _workspaceMode,
      projectListLoaded: _projectListLoaded,
      projectCount: _projects.length,
      terminalUploadLoading: _terminalUploadLoading,
      terminalUploadStatus: _terminalUploadStatus,
      terminalBufferLoading: _terminalBufferLoading,
      sessionId: _sessionId,
      pendingBufferSessionId: _terminalBufferRetry.pendingSessionId,
      connectionStatusText: _connectionStatusText,
      terminalHistoryLoadingText: _terminalHistoryLoadingText(),
      maskOpacity: _maskOpacity,
      keyboardVisible: _keyboardVisible,
      terminalCursorBottom: _terminalCursorBottom,
      onConnect: () => _connect(),
      onControllerCreated: (controller) {
        final port = CoduxNativeTerminalPort(controller);
        _nativeTerminalController = controller;
        _nativeTerminalPort = port;
        _terminalRenderer.attach(port);
        controller.setLogLevel(CoduxLog.nativeLevelName);
        _restoreNativeTerminalController(controller);
        _terminalReady = false;
        _terminalBufferRetry.resetLastBuffered();
        controller.requestResize();
        WidgetsBinding.instance.addPostFrameCallback((_) {
          if (!mounted) return;
          _replayCurrentTerminalToNative(
            controller: controller,
            reason: 'controller-created',
          );
        });
      },
      onControllerDisposed: (controller) {
        if (_nativeTerminalController == controller) {
          final port = _nativeTerminalPort;
          if (port != null) {
            _terminalRenderer.detach(port);
          }
          _nativeTerminalPort = null;
          _nativeTerminalController = null;
          _terminalReady = false;
        }
      },
      onInput: _queueTerminalTyping,
      onResize: (cols, rows) {
        final firstResize = !_terminalReady;
        _terminalReady = true;
        _sendTerminalResize(cols, rows);
        if (firstResize) {
          WidgetsBinding.instance.addPostFrameCallback((_) {
            if (!mounted) return;
            _requestBufferForCurrentSession(force: true, preferFull: true);
            final controller = _nativeTerminalController;
            if (controller != null) {
              _replayCurrentTerminalToNative(
                controller: controller,
                reason: 'first-resize',
              );
            }
          });
        }
      },
      onMetricsCursorBottom: (cursorBottom) {
        if (_terminalCursorBottom == cursorBottom) return;
        setState(() {
          _terminalCursorBottom = cursorBottom;
        });
      },
      onSendKey: _sendTerminalKey,
      onToggleKeyboard: _toggleTerminalKeyboard,
      onPaste: _pasteToTerminal,
      onCopy: _copyTerminalSelection,
      onUpload: _chooseUploadForTerminal,
      onVoiceInput: _startVoiceInput,
    );

    return RemoteWorkspaceView(
      topInset: topInset,
      workspaceMode: _workspaceMode,
      connected: _isConnected,
      latencyMs: _latencyMs,
      projects: _projects,
      selectedProjectId: _selectedProjectId,
      projectListLoaded: _projectListLoaded,
      terminals: _currentProjectTerminals(),
      activeTerminalId: _sessionId,
      hasCurrentTerminal: _currentTerminal() != null,
      aiStats: _currentAIStats,
      aiStatsLoading: _aiStatsLoading,
      projectFilesPath: _projectFilesPath,
      projectFilesParent: _projectFilesParent,
      projectFileEntries: _projectFileEntries,
      projectFilesLoading: _projectFilesLoading,
      terminalBody: terminalBody,
      onShowTerminal: _showTerminalMode,
      onShowStats: _requestAIStats,
      onShowFiles: _showFilesMode,
      onBack: () => setState(() {
        _showTerminal = false;
        _workspaceMode = 'terminal';
      }),
      onEditProject: _requestProjectEdit,
      onAddProject: _requestProjectAdd,
      onRemoveProject: _requestProjectRemove,
      onSelectProject: _onProjectSelected,
      onSelectTerminal: _selectTerminal,
      onRefreshLists: _refreshLists,
      onCreateTerminal: _createCurrentProjectTerminal,
      onCloseCurrentTerminal: _closeCurrentTerminal,
      onRebuildTerminal: _rebuildCurrentTerminal,
      onOpenTerminalSwitcher: _openTerminalSwitcher,
      onRequestProjectFiles: _requestProjectFiles,
      onOpenProjectFile: _requestFileRead,
      onOpenProjectHome: () => _openFileLocation(_selectedProject?.path ?? ''),
      onOpenProjectRoot: () => _openFileLocation('/'),
      onOpenProjectVolumes: () => _openFileLocation('/Volumes'),
      onRenameProjectFile: _renameProjectFile,
      onCopyProjectFilePath: _copyProjectFilePath,
      onDeleteProjectFile: _deleteProjectFile,
    );
  }
}
