import 'package:codux_flutter/models/remote_models.dart';
import 'package:codux_flutter/services/storage_service.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shared_preferences/shared_preferences.dart';

void main() {
  setUp(() {
    SharedPreferences.setMockInitialValues({});
  });

  test('project list cache is scoped by host without server url', () async {
    final storage = StorageService();
    const device = StoredDevice(
      server: 'legacy-a',
      hostId: 'host-1',
      deviceId: 'device-1',
      token: 'token',
      name: 'Phone',
      transports: [
        RemoteTransportCandidate(
          kind: RemoteTransportKind.websocketRelay,
          url: 'legacy-a',
        ),
      ],
    );
    const sameHostWithDifferentLegacyServer = StoredDevice(
      server: 'legacy-b',
      hostId: 'host-1',
      deviceId: 'device-1',
      token: 'token',
      name: 'Phone',
      transports: [
        RemoteTransportCandidate(
          kind: RemoteTransportKind.websocketRelay,
          url: 'legacy-b',
        ),
      ],
    );

    await storage.saveCachedProjects(device, const [
      ProjectInfo(id: 'project-1', name: 'Codux', path: '/Volumes/Web/codux'),
    ]);

    final cached = await storage.loadCachedProjects(
      sameHostWithDifferentLegacyServer,
    );

    expect(cached, hasLength(1));
    expect(cached.single.id, 'project-1');
    expect(cached.single.name, 'Codux');
    expect(cached.single.path, '/Volumes/Web/codux');
  });
}
