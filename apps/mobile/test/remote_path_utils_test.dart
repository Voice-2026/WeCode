import 'package:wecode_flutter/services/remote_path_utils.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('returns last path component for unix and windows paths', () {
    expect(remoteLastPathComponent('/Volumes/Web/wecode'), 'wecode');
    expect(remoteLastPathComponent(r'C:\work\wecode'), 'wecode');
    expect(remoteLastPathComponent('/'), 'Project');
  });

  test('returns parent path without crossing root', () {
    expect(remoteParentPathOf('/Volumes/Web/wecode'), '/Volumes/Web');
    expect(remoteParentPathOf('/wecode'), '/');
    expect(remoteParentPathOf('wecode'), '/');
  });
}
