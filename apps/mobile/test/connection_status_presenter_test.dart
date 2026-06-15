import 'package:codux_flutter/services/connection_status_presenter.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  const presenter = ConnectionStatusPresenter();

  test('shows not connected for idle disconnected state', () {
    expect(
      presenter.connectionStatusKey(_snapshot(connected: false)),
      'app.notConnected',
    );
  });

  test('keeps custom disconnected status as raw status marker', () {
    expect(
      presenter.connectionStatusKey(
        _snapshot(connected: false, status: 'Upgrade required'),
      ),
      '',
    );
    expect(
      presenter.deviceListStatusKey(
        _snapshot(connected: false, status: 'Pair again'),
      ),
      '',
    );
  });

  test('shows syncing until the first project list is loaded', () {
    expect(
      presenter.connectionStatusKey(
        _snapshot(connected: true, hostResponsive: true, hasProjects: false),
      ),
      'app.syncing',
    );
  });

  test('maps transport paths to status keys', () {
    expect(presenter.transportStatusKey('direct'), 'connection.direct');
    expect(presenter.transportStatusKey('relay'), 'connection.relay');
    expect(presenter.transportStatusKey('mixed'), 'connection.relay');
    expect(presenter.transportStatusKey('unknown'), 'app.connecting');
  });
}

ConnectionStatusSnapshot _snapshot({
  required bool connected,
  bool hostResponsive = false,
  String connectionPath = 'relay',
  bool projectListLoaded = false,
  bool hasProjects = false,
  bool recovering = false,
  bool hasActiveDevice = false,
  bool backgroundConnect = false,
  String status = '',
  String connectedText = 'Connected',
}) {
  return ConnectionStatusSnapshot(
    connected: connected,
    hostResponsive: hostResponsive,
    connectionPath: connectionPath,
    projectListLoaded: projectListLoaded,
    hasProjects: hasProjects,
    recovering: recovering,
    hasActiveDevice: hasActiveDevice,
    backgroundConnect: backgroundConnect,
    status: status,
    connectedText: connectedText,
  );
}
