import 'dart:async';
import 'dart:io';
import 'dart:math' as math;
import 'dart:typed_data';

import 'package:archive/archive_io.dart';
import 'package:http/http.dart' as http;
import 'package:path/path.dart' as p;
import 'package:path_provider/path_provider.dart';
import 'package:record/record.dart';
import 'package:sherpa_onnx/sherpa_onnx.dart' as sherpa;

typedef VoiceLog = void Function(String message);
typedef VoiceProgress = void Function(double progress);

abstract class VoiceRecognitionService {
  Stream<double> get amplitudes;
  Future<void> prepare({VoiceProgress? onProgress});
  Future<void> start({VoiceProgress? onProgress});
  Future<String> stopAndRecognize();
  Future<void> cancel();
}

class LocalVoiceRecognitionService implements VoiceRecognitionService {
  LocalVoiceRecognitionService({VoiceLog? onLog}) : _onLog = onLog;

  static const sampleRate = 16000;
  static const _modelName =
      'sherpa-onnx-sense-voice-zh-en-ja-ko-yue-int8-2024-07-17';
  static const _modelUrl =
      'https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/$_modelName.tar.bz2';

  final VoiceLog? _onLog;
  final _recorder = AudioRecorder();
  final _samples = <double>[];
  final _amplitudeController = StreamController<double>.broadcast();

  StreamSubscription<Uint8List>? _audioSubscription;
  StreamSubscription<Amplitude>? _amplitudeSubscription;
  Future<Directory>? _modelDirectoryFuture;
  sherpa.OfflineRecognizer? _recognizer;
  bool _recording = false;
  bool _bindingsInitialized = false;

  @override
  Stream<double> get amplitudes => _amplitudeController.stream;

  @override
  Future<void> prepare({VoiceProgress? onProgress}) async {
    await _ensureModel(onProgress: onProgress);
    await _ensureRecognizer();
  }

  @override
  Future<void> start({VoiceProgress? onProgress}) async {
    if (_recording) return;
    final hasPermission = await _recorder.hasPermission();
    if (!hasPermission) {
      throw const VoiceRecognitionException('microphonePermissionDenied');
    }
    await prepare(onProgress: onProgress);
    _samples.clear();
    final stream = await _recorder.startStream(
      const RecordConfig(
        encoder: AudioEncoder.pcm16bits,
        sampleRate: sampleRate,
        numChannels: 1,
        autoGain: true,
        noiseSuppress: true,
        streamBufferSize: 4096,
      ),
    );
    _recording = true;
    _audioSubscription = stream.listen(
      _appendPcm16,
      onError: (Object error) {
        _log('audio stream error=$error');
      },
    );
    _amplitudeSubscription = _recorder
        .onAmplitudeChanged(const Duration(milliseconds: 80))
        .listen((amplitude) {
          _amplitudeController.add(_normalizeAmplitude(amplitude.current));
        });
    _log('recording started');
  }

  @override
  Future<String> stopAndRecognize() async {
    if (!_recording) return '';
    _recording = false;
    await _recorder.stop();
    await _audioSubscription?.cancel();
    await _amplitudeSubscription?.cancel();
    _audioSubscription = null;
    _amplitudeSubscription = null;
    _amplitudeController.add(0);
    final samples = Float32List.fromList(_samples);
    _samples.clear();
    if (samples.length < sampleRate ~/ 5) return '';
    final recognizer = await _ensureRecognizer();
    final stream = recognizer.createStream();
    try {
      stream.acceptWaveform(samples: samples, sampleRate: sampleRate);
      recognizer.decode(stream);
      return recognizer.getResult(stream).text.trim();
    } finally {
      stream.free();
    }
  }

  @override
  Future<void> cancel() async {
    if (!_recording) return;
    _recording = false;
    await _recorder.cancel();
    await _audioSubscription?.cancel();
    await _amplitudeSubscription?.cancel();
    _audioSubscription = null;
    _amplitudeSubscription = null;
    _samples.clear();
    _amplitudeController.add(0);
    _log('recording cancelled');
  }

  Future<void> dispose() async {
    await cancel();
    await _recorder.dispose();
    await _amplitudeController.close();
    _recognizer?.free();
    _recognizer = null;
  }

  Future<sherpa.OfflineRecognizer> _ensureRecognizer() async {
    final existing = _recognizer;
    if (existing != null) return existing;
    final modelDir = await _ensureModel();
    if (!_bindingsInitialized) {
      sherpa.initBindings();
      _bindingsInitialized = true;
    }
    final recognizer = sherpa.OfflineRecognizer(
      sherpa.OfflineRecognizerConfig(
        model: sherpa.OfflineModelConfig(
          tokens: p.join(modelDir.path, 'tokens.txt'),
          senseVoice: sherpa.OfflineSenseVoiceModelConfig(
            model: p.join(modelDir.path, 'model.int8.onnx'),
            language: 'zh',
            useInverseTextNormalization: true,
          ),
          numThreads: 2,
          debug: false,
          provider: 'cpu',
        ),
      ),
    );
    _recognizer = recognizer;
    _log('recognizer ready model=${modelDir.path}');
    return recognizer;
  }

  Future<Directory> _ensureModel({VoiceProgress? onProgress}) {
    final existing = _modelDirectoryFuture;
    if (existing != null) return existing;
    final future = _downloadAndExtractModel(onProgress: onProgress);
    _modelDirectoryFuture = future.catchError((error) {
      _modelDirectoryFuture = null;
      throw error;
    });
    return _modelDirectoryFuture!;
  }

  Future<Directory> _downloadAndExtractModel({
    VoiceProgress? onProgress,
  }) async {
    final appDir = await getApplicationSupportDirectory();
    final modelRoot = Directory(p.join(appDir.path, 'voice_models'));
    final modelDir = Directory(p.join(modelRoot.path, _modelName));
    if (await _modelReady(modelDir)) {
      onProgress?.call(1);
      return modelDir;
    }

    await modelRoot.create(recursive: true);
    final archivePath = p.join(modelRoot.path, '$_modelName.tar.bz2');
    final archiveFile = File(archivePath);
    await _downloadModel(archiveFile, onProgress: onProgress);

    final stagingDir = Directory(p.join(modelRoot.path, '$_modelName.tmp'));
    if (await stagingDir.exists()) {
      await stagingDir.delete(recursive: true);
    }
    await stagingDir.create(recursive: true);
    _log('extracting model archive=$archivePath');
    await extractFileToDisk(archivePath, stagingDir.path);

    final extractedDir = Directory(p.join(stagingDir.path, _modelName));
    final readyDir = await extractedDir.exists() ? extractedDir : stagingDir;
    if (await modelDir.exists()) {
      await modelDir.delete(recursive: true);
    }
    await readyDir.rename(modelDir.path);
    if (await stagingDir.exists()) {
      await stagingDir.delete(recursive: true);
    }
    if (await archiveFile.exists()) {
      await archiveFile.delete();
    }
    if (!await _modelReady(modelDir)) {
      throw const VoiceRecognitionException('voiceModelInvalid');
    }
    onProgress?.call(1);
    return modelDir;
  }

  Future<void> _downloadModel(
    File archiveFile, {
    VoiceProgress? onProgress,
  }) async {
    _log('downloading model url=$_modelUrl');
    final request = http.Request('GET', Uri.parse(_modelUrl));
    final client = http.Client();
    try {
      final response = await client.send(request);
      if (response.statusCode < 200 || response.statusCode >= 300) {
        throw VoiceRecognitionException('voiceModelDownloadFailed');
      }
      final total = response.contentLength ?? 0;
      var received = 0;
      final sink = archiveFile.openWrite();
      try {
        await for (final chunk in response.stream) {
          received += chunk.length;
          sink.add(chunk);
          if (total > 0) {
            onProgress?.call((received / total).clamp(0, 0.92));
          }
        }
      } finally {
        await sink.close();
      }
      _log('downloaded model bytes=$received');
    } finally {
      client.close();
    }
  }

  Future<bool> _modelReady(Directory dir) async {
    final tokens = File(p.join(dir.path, 'tokens.txt'));
    final model = File(p.join(dir.path, 'model.int8.onnx'));
    return await tokens.exists() && await model.exists();
  }

  void _appendPcm16(Uint8List bytes) {
    final view = ByteData.sublistView(bytes);
    final sampleCount = bytes.length ~/ 2;
    for (var index = 0; index < sampleCount; index += 1) {
      final value = view.getInt16(index * 2, Endian.little);
      _samples.add((value / 32768.0).clamp(-1.0, 1.0));
    }
  }

  double _normalizeAmplitude(double currentDb) {
    if (currentDb <= -60) return 0;
    if (currentDb >= 0) return 1;
    return math.pow(10, currentDb / 35).toDouble().clamp(0, 1);
  }

  void _log(String message) => _onLog?.call(message);
}

class VoiceRecognitionException implements Exception {
  const VoiceRecognitionException(this.code);

  final String code;

  @override
  String toString() => code;
}
