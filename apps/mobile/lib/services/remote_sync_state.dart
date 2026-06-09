class RemoteSyncState {
  bool projectListLoaded = false;
  bool terminalListLoaded = false;
  bool projectListPending = false;
  bool terminalListPending = false;
  int projectListRetryAttempt = 0;
  int terminalListRetryAttempt = 0;

  void beginConnectionGeneration() {
    projectListLoaded = false;
    terminalListLoaded = false;
    projectListPending = false;
    terminalListPending = false;
    projectListRetryAttempt = 0;
    terminalListRetryAttempt = 0;
  }

  void markProjectListReceived() {
    projectListLoaded = true;
    projectListPending = false;
    projectListRetryAttempt = 0;
  }

  void markTerminalListReceived() {
    terminalListLoaded = true;
    terminalListPending = false;
    terminalListRetryAttempt = 0;
  }

  bool shouldRequestProjectList({bool force = false}) {
    return force || (!projectListLoaded && !projectListPending);
  }

  bool shouldRequestTerminalList({bool force = false}) {
    return force || (!terminalListLoaded && !terminalListPending);
  }

  void markProjectListRequested() {
    projectListPending = true;
  }

  void markTerminalListRequested() {
    terminalListPending = true;
  }

  void resetProjectListRetry() {
    projectListRetryAttempt = 0;
    projectListPending = false;
  }

  void resetTerminalListRetry() {
    terminalListRetryAttempt = 0;
    terminalListPending = false;
  }

  bool canRetryProjectList(int maxAttempts) {
    return !projectListLoaded && projectListRetryAttempt < maxAttempts;
  }

  bool canRetryTerminalList(int maxAttempts) {
    return !terminalListLoaded && terminalListRetryAttempt < maxAttempts;
  }

  int nextProjectListRetryAttempt() {
    projectListRetryAttempt += 1;
    projectListPending = false;
    return projectListRetryAttempt;
  }

  int nextTerminalListRetryAttempt() {
    terminalListRetryAttempt += 1;
    terminalListPending = false;
    return terminalListRetryAttempt;
  }
}
