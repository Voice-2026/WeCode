import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:mobile_scanner/mobile_scanner.dart';
import '../i18n.dart';
import '../services/remote_protocol.dart';
import '../theme/app_theme.dart';

class ScannerScreen extends StatefulWidget {
  const ScannerScreen({
    super.key,
    required this.bottomInset,
    required this.onDetected,
    required this.onClose,
  });

  final double bottomInset;
  final ValueChanged<String> onDetected;
  final VoidCallback onClose;

  @override
  State<ScannerScreen> createState() => _ScannerScreenState();
}

class _ScannerScreenState extends State<ScannerScreen>
    with WidgetsBindingObserver {
  late final MobileScannerController _controller;
  final _pairingCodeController = TextEditingController();
  final _customServerController = TextEditingController();
  bool _startPending = false;
  bool _handledPayload = false;
  bool _showManualConnect = false;

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addObserver(this);
    _controller = MobileScannerController(
      autoStart: false,
      formats: const [BarcodeFormat.qrCode],
    );
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (mounted) _startScanner();
    });
  }

  Future<void> _startScanner() async {
    if (_startPending || _handledPayload || _controller.value.isRunning) {
      return;
    }
    _startPending = true;
    await _controller.start();
    _startPending = false;
  }

  Future<void> _stopScanner() async {
    if (_controller.value.isRunning) {
      await _controller.stop();
    }
  }

  void _handleDetected(String? value) {
    final payload = value?.trim();
    if (payload == null || payload.isEmpty || _handledPayload) return;
    _handledPayload = true;
    _stopScanner();
    widget.onDetected(payload);
  }

  void _openManualConnect() {
    setState(() => _showManualConnect = true);
  }

  void _submitManualPayload(String server) {
    final code = _pairingCodeController.text.trim();
    if (server.trim().isEmpty || code.isEmpty) return;
    _handleDetected(
      Uri(
        scheme: 'codux',
        host: 'manual-pair',
        queryParameters: {'server': server.trim(), 'code': code},
      ).toString(),
    );
  }

  @override
  void didChangeAppLifecycleState(AppLifecycleState state) {
    if (!mounted || _handledPayload) return;
    switch (state) {
      case AppLifecycleState.resumed:
        _startScanner();
      case AppLifecycleState.inactive:
      case AppLifecycleState.hidden:
      case AppLifecycleState.paused:
      case AppLifecycleState.detached:
        _stopScanner();
    }
  }

  @override
  void dispose() {
    WidgetsBinding.instance.removeObserver(this);
    _pairingCodeController.dispose();
    _customServerController.dispose();
    _controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final accent = Theme.of(context).colorScheme.secondary;
    final prefs = AppPreferences.of(context);
    return Positioned.fill(
      child: ColoredBox(
        color: Colors.black,
        child: Stack(
          children: [
            MobileScanner(
              controller: _controller,
              useAppLifecycleState: false,
              onDetect: (capture) {
                final value = capture.barcodes.firstOrNull?.rawValue;
                _handleDetected(value);
              },
            ),
            Center(
              child: Container(
                width: 240,
                height: 240,
                decoration: BoxDecoration(
                  border: Border.all(color: accent, width: 2),
                  borderRadius: BorderRadius.circular(AppRadius.lg),
                ),
              ),
            ),
            Positioned(
              left: 18,
              right: 18,
              bottom: 36 + widget.bottomInset,
              child: Column(
                children: [
                  Text(
                    prefs.t('pair.scanTitle'),
                    style: const TextStyle(
                      color: Colors.white,
                      fontSize: AppTextSize.title,
                      fontWeight: FontWeight.w700,
                    ),
                  ),
                  const SizedBox(height: AppSpacing.s),
                  Text(
                    prefs.t('pair.scanHint'),
                    style: const TextStyle(
                      color: Color(0xFFCBD5E1),
                      fontSize: 14,
                    ),
                  ),
                  const SizedBox(height: AppSpacing.l),
                  Wrap(
                    alignment: WrapAlignment.center,
                    spacing: AppSpacing.s,
                    runSpacing: AppSpacing.s,
                    children: [
                      _ScannerAction(
                        label: prefs.t('pair.manualConnect'),
                        onTap: _openManualConnect,
                      ),
                      _ScannerAction(
                        label: prefs.t('pair.close'),
                        onTap: () {
                          _stopScanner();
                          widget.onClose();
                        },
                      ),
                    ],
                  ),
                ],
              ),
            ),
            if (_showManualConnect)
              _ManualConnectOverlay(
                codeController: _pairingCodeController,
                customServerController: _customServerController,
                onSubmit: _submitManualPayload,
                onCancel: () => setState(() => _showManualConnect = false),
              ),
          ],
        ),
      ),
    );
  }
}

class _ScannerAction extends StatelessWidget {
  const _ScannerAction({required this.label, required this.onTap});
  final String label;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) => Material(
    color: Colors.black54,
    borderRadius: BorderRadius.circular(AppRadius.sm),
    child: InkWell(
      borderRadius: BorderRadius.circular(AppRadius.sm),
      onTap: onTap,
      child: Padding(
        padding: const EdgeInsets.symmetric(
          horizontal: AppSpacing.l,
          vertical: AppSpacing.m,
        ),
        child: Text(
          label,
          style: const TextStyle(
            color: Colors.white,
            fontWeight: FontWeight.w700,
          ),
        ),
      ),
    ),
  );
}

class _ManualConnectOverlay extends StatefulWidget {
  const _ManualConnectOverlay({
    required this.codeController,
    required this.customServerController,
    required this.onSubmit,
    required this.onCancel,
  });

  final TextEditingController codeController;
  final TextEditingController customServerController;
  final ValueChanged<String> onSubmit;
  final VoidCallback onCancel;

  @override
  State<_ManualConnectOverlay> createState() => _ManualConnectOverlayState();
}

class _ManualConnectOverlayState extends State<_ManualConnectOverlay> {
  String _preset = 'global';

  String get _server => remoteTransportRelayUrlForPreset(
    preset: _preset,
    customUrl: widget.customServerController.text,
  );

  @override
  void initState() {
    super.initState();
    widget.customServerController.addListener(_onInputChanged);
    widget.codeController.addListener(_onInputChanged);
  }

  @override
  void dispose() {
    widget.customServerController.removeListener(_onInputChanged);
    widget.codeController.removeListener(_onInputChanged);
    super.dispose();
  }

  void _onInputChanged() {
    if (mounted) setState(() {});
  }

  bool get _canSubmit {
    final code = widget.codeController.text.replaceAll(RegExp(r'\D'), '');
    return code.length == 6 && _server.trim().isNotEmpty;
  }

  void _submit() {
    if (!_canSubmit) return;
    widget.onSubmit(_server);
  }

  @override
  Widget build(BuildContext context) {
    final prefs = AppPreferences.of(context);
    final accent = Theme.of(context).colorScheme.secondary;
    return Positioned.fill(
      child: ColoredBox(
        color: AppColors.backdrop,
        child: Center(
          child: Container(
            width: 340,
            margin: const EdgeInsets.symmetric(horizontal: AppSpacing.l),
            padding: const EdgeInsets.all(AppSpacing.l),
            decoration: BoxDecoration(
              color: AppColors.bgSurface,
              borderRadius: BorderRadius.circular(AppRadius.lg),
              border: Border.all(color: AppColors.border, width: 0.5),
            ),
            child: Column(
              mainAxisSize: MainAxisSize.min,
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                Text(
                  prefs.t('pair.manualConnect'),
                  style: const TextStyle(
                    color: AppColors.textPrimary,
                    fontSize: AppTextSize.body,
                    fontWeight: FontWeight.w700,
                  ),
                ),
                const SizedBox(height: AppSpacing.m),
                SegmentedButton<String>(
                  segments: [
                    ButtonSegment(
                      value: 'global',
                      label: Text(prefs.t('pair.serverGlobal')),
                    ),
                    ButtonSegment(
                      value: 'china',
                      label: Text(prefs.t('pair.serverChina')),
                    ),
                    ButtonSegment(
                      value: 'custom',
                      label: Text(prefs.t('pair.serverCustom')),
                    ),
                  ],
                  selected: {_preset},
                  onSelectionChanged: (value) =>
                      setState(() => _preset = value.first),
                  showSelectedIcon: false,
                ),
                if (_preset == 'custom') ...[
                  const SizedBox(height: AppSpacing.m),
                  TextField(
                    controller: widget.customServerController,
                    autofocus: true,
                    keyboardType: TextInputType.url,
                    textInputAction: TextInputAction.next,
                    style: const TextStyle(
                      color: AppColors.textPrimary,
                      fontSize: AppTextSize.body,
                    ),
                    decoration: InputDecoration(
                      filled: true,
                      fillColor: AppColors.bgElevated,
                      hintText: prefs.t('pair.serverCustomHint'),
                      hintStyle: const TextStyle(color: AppColors.textSubtle),
                      border: OutlineInputBorder(
                        borderRadius: BorderRadius.circular(AppRadius.sm),
                        borderSide: BorderSide.none,
                      ),
                    ),
                  ),
                ],
                const SizedBox(height: AppSpacing.m),
                TextField(
                  controller: widget.codeController,
                  autofocus: _preset != 'custom',
                  keyboardType: TextInputType.number,
                  textInputAction: TextInputAction.done,
                  inputFormatters: [FilteringTextInputFormatter.digitsOnly],
                  maxLength: 6,
                  onSubmitted: (_) => _submit(),
                  style: const TextStyle(
                    color: AppColors.textPrimary,
                    fontSize: 24,
                    fontWeight: FontWeight.w700,
                    letterSpacing: 0,
                  ),
                  decoration: InputDecoration(
                    counterText: '',
                    filled: true,
                    fillColor: AppColors.bgElevated,
                    hintText: prefs.t('pair.codeHint'),
                    hintStyle: const TextStyle(color: AppColors.textSubtle),
                    border: OutlineInputBorder(
                      borderRadius: BorderRadius.circular(AppRadius.sm),
                      borderSide: BorderSide.none,
                    ),
                  ),
                ),
                const SizedBox(height: AppSpacing.s),
                Text(
                  prefs.t('pair.manualHelp'),
                  style: const TextStyle(
                    color: AppColors.textMuted,
                    fontSize: AppTextSize.small,
                  ),
                ),
                const SizedBox(height: AppSpacing.m),
                Row(
                  children: [
                    Expanded(
                      child: OutlinedButton(
                        onPressed: widget.onCancel,
                        child: Text(
                          prefs.t('app.cancel'),
                          maxLines: 1,
                          overflow: TextOverflow.ellipsis,
                        ),
                      ),
                    ),
                    const SizedBox(width: AppSpacing.s),
                    Expanded(
                      child: FilledButton(
                        onPressed: _canSubmit ? _submit : null,
                        style: FilledButton.styleFrom(
                          backgroundColor: accent,
                          foregroundColor: AppColors.bgBase,
                        ),
                        child: Text(
                          prefs.t('pair.submit'),
                          maxLines: 1,
                          overflow: TextOverflow.ellipsis,
                        ),
                      ),
                    ),
                  ],
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}
