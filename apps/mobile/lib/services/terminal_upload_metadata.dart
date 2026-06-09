enum TerminalUploadSource { file, image }

String terminalUploadKind(TerminalUploadSource source) =>
    source == TerminalUploadSource.image ? 'image' : 'file';

String terminalUploadUploadingKey(TerminalUploadSource source) =>
    source == TerminalUploadSource.image
    ? 'upload.imageUploading'
    : 'upload.fileUploading';

String terminalUploadInsertingKey(TerminalUploadSource source) =>
    source == TerminalUploadSource.image
    ? 'upload.imageInserting'
    : 'upload.fileInserting';

String terminalUploadMime(String name, {required bool image}) {
  final parts = name.split('.');
  final extension = parts.length > 1 ? parts.last.toLowerCase() : '';
  return switch (extension) {
    'jpg' || 'jpeg' => 'image/jpeg',
    'png' => 'image/png',
    'gif' => 'image/gif',
    'webp' => 'image/webp',
    'heic' => 'image/heic',
    'pdf' => 'application/pdf',
    'json' => 'application/json',
    'txt' || 'log' || 'md' => 'text/plain',
    'csv' => 'text/csv',
    'html' || 'htm' => 'text/html',
    'zip' => 'application/zip',
    _ => image ? 'image/*' : 'application/octet-stream',
  };
}
