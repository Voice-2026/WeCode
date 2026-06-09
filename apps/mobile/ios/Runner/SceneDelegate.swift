import Flutter
import UIKit

class SceneDelegate: FlutterSceneDelegate {
  override func scene(
    _ scene: UIScene,
    willConnectTo session: UISceneSession,
    options connectionOptions: UIScene.ConnectionOptions
  ) {
    super.scene(scene, willConnectTo: session, options: connectionOptions)
    disableFocusEffects(in: window)
  }

  private func disableFocusEffects(in view: UIView?) {
    guard let view else { return }
    if #available(iOS 15.0, *) {
      view.focusEffect = nil
    }
    for subview in view.subviews {
      disableFocusEffects(in: subview)
    }
  }
}
