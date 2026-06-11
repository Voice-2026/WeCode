import 'package:codux_flutter/services/remote_sync_state.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('connection generation resets loaded flags and retry attempts', () {
    final state = RemoteSyncState()
      ..markProjectListReceived()
      ..markTerminalListReceived()
      ..nextProjectListRetryAttempt()
      ..nextTerminalListRetryAttempt();

    state.beginConnectionGeneration();

    expect(state.projectListLoaded, isFalse);
    expect(state.terminalListLoaded, isFalse);
    expect(state.projectListPending, isFalse);
    expect(state.terminalListPending, isFalse);
    expect(state.projectListRetryAttempt, 0);
    expect(state.terminalListRetryAttempt, 0);
    expect(state.shouldRequestProjectList(), isTrue);
    expect(state.shouldRequestTerminalList(), isTrue);
  });

  test('loaded lists are not requested again unless forced', () {
    final state = RemoteSyncState()
      ..markProjectListReceived()
      ..markTerminalListReceived();

    expect(state.shouldRequestProjectList(), isFalse);
    expect(state.shouldRequestTerminalList(), isFalse);
    expect(state.shouldRequestProjectList(force: true), isTrue);
    expect(state.shouldRequestTerminalList(force: true), isTrue);
  });

  test('pending list requests are deduplicated until a response arrives', () {
    final state = RemoteSyncState();

    expect(state.shouldRequestProjectList(), isTrue);
    state.markProjectListRequested();
    expect(state.projectListPending, isTrue);
    expect(state.shouldRequestProjectList(), isFalse);

    expect(state.shouldRequestTerminalList(), isTrue);
    state.markTerminalListRequested();
    expect(state.terminalListPending, isTrue);
    expect(state.shouldRequestTerminalList(), isFalse);

    state.markProjectListReceived();
    state.markTerminalListReceived();

    expect(state.projectListPending, isFalse);
    expect(state.terminalListPending, isFalse);
    expect(state.shouldRequestProjectList(), isFalse);
    expect(state.shouldRequestTerminalList(), isFalse);
    expect(state.shouldRequestProjectList(force: true), isTrue);
    expect(state.shouldRequestTerminalList(force: true), isTrue);
  });

  test(
    'retry and reset clear pending requests without marking lists loaded',
    () {
      final state = RemoteSyncState()
        ..markProjectListRequested()
        ..markTerminalListRequested();

      expect(state.shouldRequestProjectList(), isFalse);
      expect(state.shouldRequestTerminalList(), isFalse);

      expect(state.nextProjectListRetryAttempt(), 1);
      expect(state.nextTerminalListRetryAttempt(), 1);

      expect(state.projectListPending, isFalse);
      expect(state.terminalListPending, isFalse);
      expect(state.shouldRequestProjectList(), isTrue);
      expect(state.shouldRequestTerminalList(), isTrue);

      state
        ..markProjectListRequested()
        ..markTerminalListRequested()
        ..resetProjectListRetry()
        ..resetTerminalListRetry();

      expect(state.projectListRetryAttempt, 0);
      expect(state.terminalListRetryAttempt, 0);
      expect(state.projectListPending, isFalse);
      expect(state.terminalListPending, isFalse);
    },
  );
}
