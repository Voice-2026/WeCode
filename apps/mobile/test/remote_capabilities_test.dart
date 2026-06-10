import 'package:codux_flutter/services/remote_capabilities.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('parses terminal buffer capability from host info', () {
    final capability = TerminalBufferCapability.fromHostInfo({
      'protocolVersion': 'v3.1',
      'capabilities': {
        'terminalBuffer': {
          'chunking': true,
          'maxChars': 180000,
          'chunkChars': 32768,
        },
      },
    }, clientMaxChars: 200000);

    expect(capability.chunking, isTrue);
    expect(capability.maxChars, 180000);
    expect(capability.chunkChars, 32768);
    expect(capability.requestId, isFalse);
  });

  test('parses request id capability', () {
    final capability = TerminalBufferCapability.fromHostInfo({
      'protocolVersion': 'v3.1',
      'capabilities': {
        'terminalBuffer': {
          'chunking': true,
          'maxChars': 65536,
          'chunkChars': 16384,
          'requestId': true,
        },
      },
    });

    expect(capability.requestId, isTrue);
  });

  test('limits terminal buffer capability to mobile default', () {
    final capability = TerminalBufferCapability.fromHostInfo({
      'protocolVersion': 'v3.1',
      'capabilities': {
        'terminalBuffer': {
          'chunking': true,
          'maxChars': 180000,
          'chunkChars': 32768,
        },
      },
    });

    expect(capability.chunking, isTrue);
    expect(capability.maxChars, TerminalBufferCapability.mobileMaxChars);
    expect(capability.chunkChars, 32768);
  });

  test('clamps terminal buffer capability to mobile limits', () {
    final capability = TerminalBufferCapability.fromHostInfo({
      'capabilities': {
        'terminalBuffer': {
          'chunking': true,
          'maxChars': 999999,
          'chunkChars': 999999,
        },
      },
    });

    expect(capability.maxChars, TerminalBufferCapability.mobileMaxChars);
    expect(capability.chunkChars, 65536);
  });

  test('falls back when host info has no terminal capability', () {
    final capability = TerminalBufferCapability.fromHostInfo({
      'protocolVersion': 'v3.0',
    });

    expect(capability.chunking, isFalse);
    expect(capability.maxChars, TerminalBufferCapability.mobileMaxChars);
    expect(capability.chunkChars, 16384);
  });
}
