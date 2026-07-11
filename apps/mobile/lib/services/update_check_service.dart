import 'dart:convert';
import 'dart:io';

import 'package:http/http.dart' as http;
import 'package:package_info_plus/package_info_plus.dart';

class UpdateCheckService {
  const UpdateCheckService({
    this.githubRepository = 'duxweb/wecode-flutter',
    this.requestTimeout = const Duration(seconds: 10),
    http.Client? httpClient,
  }) : _httpClient = httpClient;

  final String githubRepository;
  final Duration requestTimeout;
  final http.Client? _httpClient;

  Future<UpdateCheckResult> check() async {
    final info = await PackageInfo.fromPlatform();
    final outcome = Platform.isIOS
        ? await checkAppStore(info)
        : await checkGithub(info);
    return outcome.copyWith(
      currentVersion: info.version,
      currentBuildNumber: info.buildNumber,
      isIos: Platform.isIOS,
    );
  }

  Future<UpdateCheckResult> checkGithub(PackageInfo info) async {
    final uri = Uri.parse(
      'https://api.github.com/repos/$githubRepository/releases/latest',
    );
    final response = await _get(
      uri,
      headers: const {
        'Accept': 'application/vnd.github+json',
        'X-GitHub-Api-Version': '2022-11-28',
      },
    );
    if (response.statusCode == 404) {
      return UpdateCheckResult.toast(
        key: 'update.noRelease',
        currentVersion: info.version,
        currentBuildNumber: info.buildNumber,
      );
    }
    if (response.statusCode < 200 || response.statusCode >= 300) {
      return UpdateCheckResult.toast(
        key: 'update.httpFailed',
        params: {'status': '${response.statusCode}'},
        currentVersion: info.version,
        currentBuildNumber: info.buildNumber,
      );
    }
    final json = jsonDecode(response.body) as Map<String, dynamic>;
    final tag = '${json['tag_name'] ?? ''}'.trim();
    final url = '${json['html_url'] ?? ''}'.trim();
    return _versionOutcome(
      remoteVersion: tag,
      url: url,
      currentVersion: info.version,
      currentBuildNumber: info.buildNumber,
    );
  }

  Future<UpdateCheckResult> checkAppStore(PackageInfo info) async {
    final uri = Uri.https('itunes.apple.com', '/lookup', {
      'bundleId': info.packageName,
    });
    final response = await _get(uri);
    if (response.statusCode < 200 || response.statusCode >= 300) {
      return UpdateCheckResult.toast(
        key: 'update.httpFailed',
        params: {'status': '${response.statusCode}'},
        currentVersion: info.version,
        currentBuildNumber: info.buildNumber,
        isIos: true,
      );
    }
    final json = jsonDecode(response.body) as Map<String, dynamic>;
    final results = json['results'] as List<dynamic>? ?? const [];
    if (results.isEmpty) {
      return UpdateCheckResult.toast(
        key: 'update.appStorePending',
        currentVersion: info.version,
        currentBuildNumber: info.buildNumber,
        isIos: true,
      );
    }
    final first = results.first as Map<String, dynamic>;
    final version = '${first['version'] ?? ''}'.trim();
    final url = '${first['trackViewUrl'] ?? ''}'.trim();
    return _versionOutcome(
      remoteVersion: version,
      url: url,
      currentVersion: info.version,
      currentBuildNumber: info.buildNumber,
      isIos: true,
    );
  }

  Future<http.Response> _get(Uri uri, {Map<String, String>? headers}) {
    final client = _httpClient;
    final request = client == null
        ? http.get(uri, headers: headers)
        : client.get(uri, headers: headers);
    return request.timeout(requestTimeout);
  }

  UpdateCheckResult _versionOutcome({
    required String remoteVersion,
    required String url,
    required String currentVersion,
    required String currentBuildNumber,
    bool isIos = false,
  }) {
    if (remoteVersion.isEmpty) {
      return UpdateCheckResult.toast(
        key: 'update.noVersion',
        currentVersion: currentVersion,
        currentBuildNumber: currentBuildNumber,
        isIos: isIos,
      );
    }
    if (compareVersions(remoteVersion, currentVersion) <= 0) {
      return UpdateCheckResult.toast(
        key: 'update.latest',
        params: {'version': currentVersion},
        currentVersion: currentVersion,
        currentBuildNumber: currentBuildNumber,
        isIos: isIos,
      );
    }
    return UpdateCheckResult.available(
      version: remoteVersion,
      url: url,
      currentVersion: currentVersion,
      currentBuildNumber: currentBuildNumber,
      isIos: isIos,
    );
  }
}

class UpdateCheckResult {
  const UpdateCheckResult._({
    required this.available,
    required this.currentVersion,
    required this.currentBuildNumber,
    required this.isIos,
    this.version,
    this.url = '',
    this.toastKey,
    this.toastParams = const {},
  });

  factory UpdateCheckResult.available({
    required String version,
    required String url,
    required String currentVersion,
    required String currentBuildNumber,
    bool isIos = false,
  }) {
    return UpdateCheckResult._(
      available: true,
      version: version,
      url: url,
      currentVersion: currentVersion,
      currentBuildNumber: currentBuildNumber,
      isIos: isIos,
    );
  }

  factory UpdateCheckResult.toast({
    required String key,
    required String currentVersion,
    required String currentBuildNumber,
    Map<String, String> params = const {},
    bool isIos = false,
  }) {
    return UpdateCheckResult._(
      available: false,
      toastKey: key,
      toastParams: params,
      currentVersion: currentVersion,
      currentBuildNumber: currentBuildNumber,
      isIos: isIos,
    );
  }

  final bool available;
  final String currentVersion;
  final String currentBuildNumber;
  final bool isIos;
  final String? version;
  final String url;
  final String? toastKey;
  final Map<String, String> toastParams;

  UpdateCheckResult copyWith({
    String? currentVersion,
    String? currentBuildNumber,
    bool? isIos,
  }) {
    return UpdateCheckResult._(
      available: available,
      version: version,
      url: url,
      toastKey: toastKey,
      toastParams: toastParams,
      currentVersion: currentVersion ?? this.currentVersion,
      currentBuildNumber: currentBuildNumber ?? this.currentBuildNumber,
      isIos: isIos ?? this.isIos,
    );
  }
}

int compareVersions(String left, String right) {
  List<int> parse(String value) => value
      .replaceFirst(RegExp(r'^[vV]'), '')
      .split(RegExp(r'[^0-9]+'))
      .where((part) => part.isNotEmpty)
      .take(3)
      .map(int.parse)
      .toList();
  final a = parse(left);
  final b = parse(right);
  for (var index = 0; index < 3; index += 1) {
    final av = index < a.length ? a[index] : 0;
    final bv = index < b.length ? b[index] : 0;
    if (av != bv) return av.compareTo(bv);
  }
  return 0;
}
