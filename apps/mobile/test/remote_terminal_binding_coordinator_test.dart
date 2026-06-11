import 'package:codux_flutter/models/remote_models.dart';
import 'package:codux_flutter/services/remote_capabilities.dart';
import 'package:codux_flutter/services/remote_protocol.dart';
import 'package:codux_flutter/services/remote_runtime_store.dart';
import 'package:codux_flutter/services/remote_terminal_binding_coordinator.dart';
import 'package:codux_flutter/services/remote_terminal_output_controller.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('subscribes session baseline through coordinator', () {
    final sent = <RelayEnvelope>[];
    final coordinator = _coordinator(sent: sent);

    final requested = coordinator.subscribeSessionBaseline(
      sessionId: 'term-1',
      reason: 'test',
      capability: const TerminalBufferCapability(chunking: true),
    );

    expect(requested, isTrue);
    expect(sent, hasLength(1));
    expect(sent.single.type, RemoteMessageType.resourceSubscribe);
    expect(sent.single.sessionId, 'term-1');
    expect(sent.single.payload, containsPair('baseline', true));
    expect(
      sent.single.payload,
      containsPair('requestId', 'req-session-term-1-1'),
    );
  });

  test('does not request baseline again when visible session has cache', () {
    final output = RemoteTerminalOutputController();
    final sent = <RelayEnvelope>[];
    final coordinator = _coordinator(output: output, sent: sent);
    output.accept(
      RelayEnvelope(
        type: RemoteMessageType.terminalBuffer,
        sessionId: 'term-1',
        payload: {
          'buffer': true,
          'data': 'ready',
          'offset': 0,
          'bufferLength': 5,
          'truncated': false,
          'outputSeq': 1,
        },
      ),
      activeSessionId: 'term-1',
    );

    coordinator.resubscribeVisibleTerminal(
      transportConnected: true,
      protocolReady: true,
      activeSessionId: 'term-1',
      selectedProjectId: 'project-1',
      capability: TerminalBufferCapability.fallback,
      reason: 'resume',
      ensureBoundBaseline: (_, _) {},
    );

    expect(sent, hasLength(1));
    expect(sent.single.payload, isNot(containsPair('baseline', true)));
  });

  test(
    'empty bound session marks project baseline stale and subscribes project',
    () {
      final sent = <RelayEnvelope>[];
      final coordinator = _coordinator(
        sent: sent,
        terminals: {'term-1': _terminal('term-1', 'project-1')},
      );

      coordinator.ensureBoundTerminalHasBaseline(
        sessionId: 'term-1',
        baselineRequested: false,
        reason: 'bind',
        capability: TerminalBufferCapability.fallback,
      );

      expect(sent, hasLength(1));
      expect(sent.single.type, RemoteMessageType.resourceSubscribe);
      expect(sent.single.payload, containsPair('projectId', 'project-1'));
      expect(sent.single.payload, containsPair('baseline', true));
    },
  );

  test(
    'bind session subscribes project without baseline before session baseline',
    () {
      final sent = <RelayEnvelope>[];
      final output = RemoteTerminalOutputController();
      final coordinator = _coordinator(
        output: output,
        sent: sent,
        terminals: {'term-1': _terminal('term-1', 'project-1')},
      );

      final result = coordinator.bindSession(
        plan: const RemoteRuntimePlan(
          bindSessionId: 'term-1',
          bindFullBuffer: true,
        ),
        bindSessionId: 'term-1',
        reason: 'select',
        selectedProjectId: 'project-1',
        capability: TerminalBufferCapability.fallback,
        restored: false,
      );

      expect(result.baselineRequested, isTrue);
      expect(sent, hasLength(2));
      expect(sent.first.payload, isNot(containsPair('baseline', true)));
      expect(sent.last.sessionId, 'term-1');
      expect(sent.last.payload, containsPair('baseline', true));
    },
  );
}

RemoteTerminalBindingCoordinator _coordinator({
  RemoteTerminalOutputController? output,
  List<RelayEnvelope>? sent,
  Map<String, TerminalInfo> terminals = const {},
}) {
  var counter = 0;
  final messages = sent ?? <RelayEnvelope>[];
  return RemoteTerminalBindingCoordinator(
    outputController: output ?? RemoteTerminalOutputController(),
    send: (envelope) {
      messages.add(envelope);
      return true;
    },
    terminalById: terminals.lookup,
    nextRequestId: (scope) => 'req-$scope-${++counter}',
  );
}

TerminalInfo _terminal(String id, String projectId) {
  return TerminalInfo(id: id, title: id, projectId: projectId);
}

extension on Map<String, TerminalInfo> {
  TerminalInfo? lookup(String id) => this[id];
}
