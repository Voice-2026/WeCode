import 'package:codux_flutter/services/remote_path_utils.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('returns last path component for unix and windows paths', () {
    expect(remoteLastPathComponent('/Volumes/Web/codux'), 'codux');
    expect(remoteLastPathComponent(r'C:\work\codux'), 'codux');
    expect(remoteLastPathComponent('/'), 'Project');
  });

  test('returns parent path without crossing root', () {
    expect(remoteParentPathOf('/Volumes/Web/codux'), '/Volumes/Web');
    expect(remoteParentPathOf('/codux'), '/');
    expect(remoteParentPathOf('codux'), '/');
  });
}
