import 'package:flutter/foundation.dart';

/// Cheap repaint notifier for the self-drawn terminal. The renderer reads the
/// Rust cell snapshot directly from [RemoteTerminalOutputController] (which is
/// not a [Listenable]), so a live output frame just calls [tick] to rebuild
/// that subtree instead of the whole page.
class TerminalRepaintSignal extends ChangeNotifier {
  void tick() => notifyListeners();
}
