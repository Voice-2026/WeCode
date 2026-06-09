import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:mobile_scanner/mobile_scanner.dart';
import '../i18n.dart';
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
  final _manualController = TextEditingController();
  bool _startPending = false;
  bool _handledPayload = false;
  bool _showManualInput = false;

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

  Future<void> _pastePayload() async {
    final data = await Clipboard.getData(Clipboard.kTextPlain);
    final text = data?.text?.trim() ?? '';
    if (!_showManualInput) {
      setState(() {
        _showManualInput = true;
        _manualController.text = text;
      });
      return;
    }
    if (text.isNotEmpty) {
      setState(() => _manualController.text = text);
    }
  }

  void _submitManualPayload() {
    _handleDetected(_manualController.text);
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
    _manualController.dispose();
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
                        label: prefs.t('pair.pastePayload'),
                        onTap: _pastePayload,
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
            if (_showManualInput)
              _ManualPayloadOverlay(
                controller: _manualController,
                onPaste: _pastePayload,
                onSubmit: _submitManualPayload,
                onCancel: () => setState(() => _showManualInput = false),
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

class _ManualPayloadOverlay extends StatelessWidget {
  const _ManualPayloadOverlay({
    required this.controller,
    required this.onPaste,
    required this.onSubmit,
    required this.onCancel,
  });

  final TextEditingController controller;
  final VoidCallback onPaste;
  final VoidCallback onSubmit;
  final VoidCallback onCancel;

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
                  prefs.t('pair.manualHint'),
                  style: const TextStyle(
                    color: AppColors.textPrimary,
                    fontSize: AppTextSize.body,
                    fontWeight: FontWeight.w700,
                  ),
                ),
                const SizedBox(height: AppSpacing.m),
                TextField(
                  controller: controller,
                  autofocus: true,
                  minLines: 4,
                  maxLines: 6,
                  style: const TextStyle(
                    color: AppColors.textPrimary,
                    fontSize: AppTextSize.small,
                  ),
                  decoration: InputDecoration(
                    filled: true,
                    fillColor: AppColors.bgElevated,
                    hintText: prefs.t('pair.manualHint'),
                    hintStyle: const TextStyle(color: AppColors.textSubtle),
                    border: OutlineInputBorder(
                      borderRadius: BorderRadius.circular(AppRadius.sm),
                      borderSide: BorderSide.none,
                    ),
                  ),
                ),
                const SizedBox(height: AppSpacing.m),
                Row(
                  children: [
                    Expanded(
                      child: OutlinedButton(
                        onPressed: onCancel,
                        child: Text(
                          prefs.t('app.cancel'),
                          maxLines: 1,
                          overflow: TextOverflow.ellipsis,
                        ),
                      ),
                    ),
                    const SizedBox(width: AppSpacing.s),
                    Expanded(
                      child: OutlinedButton(
                        onPressed: onPaste,
                        child: Text(
                          prefs.t('pair.paste'),
                          maxLines: 1,
                          overflow: TextOverflow.ellipsis,
                        ),
                      ),
                    ),
                    const SizedBox(width: AppSpacing.s),
                    Expanded(
                      child: FilledButton(
                        onPressed: onSubmit,
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
