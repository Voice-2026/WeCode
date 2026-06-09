import 'dart:async';
import 'dart:convert';

import 'package:codux_flutter/services/update_check_service.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:package_info_plus/package_info_plus.dart';

void main() {
  test('compares semantic versions with v prefixes', () {
    expect(compareVersions('v1.7.2', '1.7.1'), greaterThan(0));
    expect(compareVersions('1.7.1', '1.7.1+35'), 0);
    expect(compareVersions('1.7.0-beta.1', '1.7.0'), 0);
    expect(compareVersions('1.6.9', '1.7.0'), lessThan(0));
  });

  test('github check returns available release', () async {
    final service = UpdateCheckService(
      httpClient: _FakeHttpClient(
        (request) => http.Response(
          jsonEncode({
            'tag_name': 'v1.7.2',
            'html_url':
                'https://github.com/duxweb/codux-flutter/releases/tag/v1.7.2',
          }),
          200,
        ),
      ),
    );

    final result = await service.checkGithub(_packageInfo(version: '1.7.1'));

    expect(result.available, isTrue);
    expect(result.version, 'v1.7.2');
    expect(result.url, contains('v1.7.2'));
  });

  test(
    'github check reports latest when remote version is not newer',
    () async {
      final service = UpdateCheckService(
        httpClient: _FakeHttpClient(
          (request) => http.Response(
            jsonEncode({
              'tag_name': 'v1.7.1',
              'html_url': 'https://example.com',
            }),
            200,
          ),
        ),
      );

      final result = await service.checkGithub(_packageInfo(version: '1.7.1'));

      expect(result.available, isFalse);
      expect(result.toastKey, 'update.latest');
      expect(result.toastParams, {'version': '1.7.1'});
    },
  );

  test('github check maps http errors to toast keys', () async {
    final service = UpdateCheckService(
      httpClient: _FakeHttpClient((request) => http.Response('', 404)),
    );

    final result = await service.checkGithub(_packageInfo());

    expect(result.available, isFalse);
    expect(result.toastKey, 'update.noRelease');
  });

  test('app store check reports pending when lookup is empty', () async {
    final service = UpdateCheckService(
      httpClient: _FakeHttpClient(
        (request) => http.Response(jsonEncode({'results': []}), 200),
      ),
    );

    final result = await service.checkAppStore(_packageInfo());

    expect(result.available, isFalse);
    expect(result.toastKey, 'update.appStorePending');
    expect(result.isIos, isTrue);
  });
}

PackageInfo _packageInfo({String version = '1.7.1'}) {
  return PackageInfo(
    appName: 'Codux',
    packageName: 'plus.dux.codux',
    version: version,
    buildNumber: '35',
    buildSignature: '',
    installerStore: null,
  );
}

final class _FakeHttpClient extends http.BaseClient {
  _FakeHttpClient(this.handler);

  final FutureOr<http.Response> Function(http.BaseRequest request) handler;

  @override
  Future<http.StreamedResponse> send(http.BaseRequest request) async {
    final response = await handler(request);
    return http.StreamedResponse(
      Stream<List<int>>.value(response.bodyBytes),
      response.statusCode,
      request: request,
      headers: response.headers,
      reasonPhrase: response.reasonPhrase,
    );
  }
}
