class ConnectionStatusSnapshot {
  const ConnectionStatusSnapshot({
    required this.connected,
    required this.hostResponsive,
    required this.connectionPath,
    required this.projectListLoaded,
    required this.hasProjects,
    required this.recovering,
    required this.hasActiveDevice,
    required this.backgroundConnect,
    required this.status,
    required this.connectedText,
  });

  final bool connected;
  final bool hostResponsive;
  final String connectionPath;
  final bool projectListLoaded;
  final bool hasProjects;
  final bool recovering;
  final bool hasActiveDevice;
  final bool backgroundConnect;
  final String status;
  final String connectedText;
}

class ConnectionStatusPresenter {
  const ConnectionStatusPresenter();

  String connectionStatusKey(ConnectionStatusSnapshot state) {
    if (!state.connected) {
      if (state.recovering) return 'app.reconnecting';
      if (state.hasActiveDevice && state.backgroundConnect) {
        return 'app.connecting';
      }
      if (state.status.isEmpty || state.status == state.connectedText) {
        return 'app.notConnected';
      }
      return '';
    }
    if (!state.hostResponsive) return 'app.connecting';
    if (!state.projectListLoaded && !state.hasProjects) return 'app.syncing';
    return transportStatusKey(state.connectionPath);
  }

  String deviceListStatusKey(ConnectionStatusSnapshot state) {
    if (!state.connected) {
      if (state.recovering) return 'app.connecting';
      if (state.hasActiveDevice && state.backgroundConnect) {
        return 'app.connecting';
      }
      return 'app.notConnected';
    }
    if (!state.hostResponsive ||
        state.connectionPath == 'unknown' ||
        state.connectionPath == 'none') {
      return 'app.connecting';
    }
    return transportStatusKey(state.connectionPath);
  }

  String transportStatusKey(String path) {
    return switch (path) {
      'direct' => 'connection.direct',
      'mixed' => 'connection.relay',
      'relay' => 'connection.relay',
      _ => 'app.connecting',
    };
  }
}
