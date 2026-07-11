import 'package:wecode_flutter/services/terminal_upload_metadata.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('maps upload source to protocol kind and i18n keys', () {
    expect(terminalUploadKind(TerminalUploadSource.file), 'file');
    expect(terminalUploadKind(TerminalUploadSource.image), 'image');
    expect(
      terminalUploadUploadingKey(TerminalUploadSource.image),
      'upload.imageUploading',
    );
    expect(
      terminalUploadInsertingKey(TerminalUploadSource.file),
      'upload.fileInserting',
    );
  });

  test('detects common mime types', () {
    expect(terminalUploadMime('photo.jpg', image: true), 'image/jpeg');
    expect(terminalUploadMime('photo.jpeg', image: true), 'image/jpeg');
    expect(terminalUploadMime('icon.png', image: true), 'image/png');
    expect(terminalUploadMime('README.md', image: false), 'text/plain');
    expect(terminalUploadMime('data.json', image: false), 'application/json');
    expect(
      terminalUploadMime('unknown.bin', image: false),
      'application/octet-stream',
    );
    expect(terminalUploadMime('unknown', image: true), 'image/*');
  });
}
