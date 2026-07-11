import 'dart:io';

import 'package:path_provider/path_provider.dart';
import 'package:share_plus/share_plus.dart';

class LogExportService {
  const LogExportService();

  Future<void> export(String text, {required String shareText}) async {
    final dir = await getTemporaryDirectory();
    final stamp = DateTime.now().toLocal().toIso8601String().replaceAll(
      RegExp(r'[:.]'),
      '-',
    );
    final file = File('${dir.path}/wecode-mobile-$stamp.log');
    await file.writeAsString(text, flush: true);
    await SharePlus.instance.share(
      ShareParams(
        text: shareText,
        files: [XFile(file.path, mimeType: 'text/plain')],
        fileNameOverrides: [file.uri.pathSegments.last],
      ),
    );
  }
}
