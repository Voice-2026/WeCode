import Flutter
import CoreText
import SwiftTerm
import UIKit

final class CoduxNativeTerminalPlugin: NSObject {
  private static let terminalFontAsset = "assets/fonts/MapleMono-NF-CN-Regular.ttf"
  private static let viewType = "codux/native_terminal"
  private static let methodChannel = "codux/native_terminal/methods"
  private static let eventChannel = "codux/native_terminal/events"
  private static var views: [Int64: WeakCoduxNativeTerminalView] = [:]
  private static var eventSink: FlutterEventSink?

  static func register(with registrar: FlutterPluginRegistrar) {
    CoduxTerminalFont.assetPath = Bundle.main.path(
      forResource: registrar.lookupKey(forAsset: terminalFontAsset),
      ofType: nil
    )

    let factory = CoduxNativeTerminalFactory(messenger: registrar.messenger())
    registrar.register(factory, withId: viewType)

    let methods = FlutterMethodChannel(name: methodChannel, binaryMessenger: registrar.messenger())
    methods.setMethodCallHandler { call, result in
      handleMethod(call, result: result)
    }

    let events = FlutterEventChannel(name: eventChannel, binaryMessenger: registrar.messenger())
    events.setStreamHandler(CoduxNativeTerminalEvents())
  }

  fileprivate static func addView(_ view: CoduxNativeTerminalView, id: Int64) {
    views[id] = WeakCoduxNativeTerminalView(view)
  }

  fileprivate static func removeView(id: Int64) {
    views.removeValue(forKey: id)
  }

  fileprivate static func emit(_ event: [String: Any]) {
    eventSink?(event)
  }

  private static func handleMethod(_ call: FlutterMethodCall, result: FlutterResult) {
    guard
      let args = call.arguments as? [String: Any],
      let id = args["id"] as? Int64,
      let view = views[id]?.value
    else {
      if let id = (call.arguments as? [String: Any])?["id"] as? Int64 {
        views[id] = nil
      }
      result(false)
      return
    }

    switch call.method {
    case "feed":
      view.feed(args["data"] as? String ?? "")
      result(true)
    case "replace":
      view.replace(args["data"] as? String ?? "")
      result(true)
    case "reset":
      view.reset()
      result(true)
    case "setFontSize":
      let size = args["fontSize"] as? Double ?? 14
      view.setFontSize(CGFloat(size))
      result(true)
    case "sendKey":
      view.sendKey(args["key"] as? String ?? "")
      result(true)
    case "focus":
      view.focus()
      result(true)
    case "showKeyboard":
      view.showKeyboard()
      result(true)
    case "hideKeyboard":
      view.hideKeyboard()
      result(true)
    default:
      result(FlutterMethodNotImplemented)
    }
  }

  private final class CoduxNativeTerminalEvents: NSObject, FlutterStreamHandler {
    func onListen(withArguments arguments: Any?, eventSink events: @escaping FlutterEventSink) -> FlutterError? {
      CoduxNativeTerminalPlugin.eventSink = events
      return nil
    }

    func onCancel(withArguments arguments: Any?) -> FlutterError? {
      CoduxNativeTerminalPlugin.eventSink = nil
      return nil
    }
  }
}

private final class CoduxNativeTerminalFactory: NSObject, FlutterPlatformViewFactory {
  private let messenger: FlutterBinaryMessenger

  init(messenger: FlutterBinaryMessenger) {
    self.messenger = messenger
  }

  func createArgsCodec() -> FlutterMessageCodec & NSObjectProtocol {
    FlutterStandardMessageCodec.sharedInstance()
  }

  func create(
    withFrame frame: CGRect,
    viewIdentifier viewId: Int64,
    arguments args: Any?
  ) -> FlutterPlatformView {
    let params = args as? [String: Any]
    let fontSize = params?["fontSize"] as? Double ?? 14
    let view = CoduxNativeTerminalView(id: viewId, frame: frame, fontSize: CGFloat(fontSize))
    CoduxNativeTerminalPlugin.addView(view, id: viewId)
    return view
  }
}

private final class CoduxNativeTerminalView: NSObject, FlutterPlatformView, TerminalViewDelegate {
  private let id: Int64
  private let container: CoduxNativeTerminalContainerView
  private var terminalView: CoduxSwiftTermTerminalView
  private var fontSize: CGFloat
  private var pendingReplayText: String?
#if canImport(MetalKit)
  private var metalEnabled = false
#endif
  private let terminalBackgroundColor = CoduxTerminalTheme.background
  private let terminalForegroundColor = CoduxTerminalTheme.foreground

  init(id: Int64, frame: CGRect, fontSize: CGFloat) {
    self.id = id
    self.container = CoduxNativeTerminalContainerView(frame: frame)
    self.fontSize = fontSize
    self.terminalView = CoduxSwiftTermTerminalView(frame: CGRect(origin: .zero, size: frame.size))
    super.init()
    container.onLayout = { [weak self] in
      self?.layoutTerminalView()
    }
    installTerminalView()
  }

  deinit {
    CoduxNativeTerminalPlugin.removeView(id: id)
  }

  func view() -> UIView {
    container
  }

  func feed(_ text: String) {
    guard !text.isEmpty else { return }
    if pendingReplayText != nil || !isLayoutReady {
      pendingReplayText = (pendingReplayText ?? "") + text
      return
    }
    terminalView.feed(text: text)
  }

  func replace(_ text: String) {
    pendingReplayText = text
    applyPendingReplayIfReady()
  }

  func reset() {
    replace("")
  }

  func setFontSize(_ size: CGFloat) {
    fontSize = size
    terminalView.font = CoduxTerminalFont.font(size: size)
    terminalView.setNeedsDisplay()
  }

  private func installTerminalView() {
    configureTerminalView(terminalView)
    terminalView.frame = container.bounds
    container.addSubview(terminalView)
    setFontSize(fontSize)
  }

  private func configureTerminalView(_ view: CoduxSwiftTermTerminalView) {
    container.backgroundColor = terminalBackgroundColor
    container.clipsToBounds = true
    view.backgroundColor = terminalBackgroundColor
    view.nativeBackgroundColor = terminalBackgroundColor
    view.nativeForegroundColor = terminalForegroundColor
    view.caretColor = terminalForegroundColor
    view.installColors(CoduxTerminalTheme.palette)
    view.notifyUpdateChanges = true
    view.terminalDelegate = self
    view.autoresizingMask = [.flexibleWidth, .flexibleHeight]
    view.font = CoduxTerminalFont.font(size: fontSize)
    view.inputAccessoryView = nil
    view.inputView = nil
    view.keyboardAppearance = .dark
    view.suppressSystemKeyboard()
  }

  private func layoutTerminalView() {
    terminalView.frame = container.bounds
    terminalView.setNeedsLayout()
    terminalView.layoutIfNeeded()
#if canImport(MetalKit)
    enableMetalIfReady()
#endif
    applyPendingReplayIfReady()
  }

  private var isLayoutReady: Bool {
    container.bounds.width > 1 && container.bounds.height > 1
  }

  private func applyPendingReplayIfReady() {
    guard isLayoutReady, let text = pendingReplayText else { return }
    pendingReplayText = nil
    applyReplay(text)
  }

  private func applyReplay(_ text: String) {
    let wasFirstResponder = terminalView.isFirstResponder
    terminalView.frame = container.bounds
    terminalView.setNeedsLayout()
    terminalView.layoutIfNeeded()
    terminalView.getTerminal().resetToInitialState()
    applyTerminalTheme()
    if !text.isEmpty {
      terminalView.feed(text: text)
    } else {
      terminalView.setNeedsDisplay(terminalView.bounds)
    }
    scrollReplayToBottom()
    if wasFirstResponder {
      terminalView.becomeFirstResponder()
    }
  }

  private func applyTerminalTheme() {
    terminalView.backgroundColor = terminalBackgroundColor
    terminalView.nativeBackgroundColor = terminalBackgroundColor
    terminalView.nativeForegroundColor = terminalForegroundColor
    terminalView.caretColor = terminalForegroundColor
    terminalView.installColors(CoduxTerminalTheme.palette)
  }

  private func scrollReplayToBottom() {
    terminalView.layoutIfNeeded()
    terminalView.scrollToBottom(notifyAccessibility: false)
  }

#if canImport(MetalKit)
  private func enableMetalIfReady() {
    guard !metalEnabled, terminalView.window != nil else { return }
    do {
      try terminalView.setUseMetal(true)
      metalEnabled = true
    } catch {
      metalEnabled = true
    }
  }
#endif

  func sendKey(_ key: String) {
    let value: String?
    switch key {
    case "enter": value = "\r"
    case "backspace": value = "\u{7f}"
    case "escape": value = "\u{1b}"
    case "tab": value = "\t"
    case "arrowLeft": value = "\u{1b}[D"
    case "arrowRight": value = "\u{1b}[C"
    case "arrowUp": value = "\u{1b}[A"
    case "arrowDown": value = "\u{1b}[B"
    default: value = nil
    }
    if let value {
      CoduxNativeTerminalPlugin.emit(["id": id, "type": "input", "data": value])
    }
  }

  func focus() {
    terminalView.setNeedsDisplay()
  }

  func showKeyboard() {
    terminalView.showSystemKeyboard()
  }

  func hideKeyboard() {
    terminalView.hideSystemKeyboard()
  }

  func send(source: TerminalView, data: ArraySlice<UInt8>) {
    let text = String(decoding: data, as: UTF8.self)
    CoduxNativeTerminalPlugin.emit(["id": id, "type": "input", "data": text])
  }

  func sizeChanged(source: TerminalView, newCols: Int, newRows: Int) {
    CoduxNativeTerminalPlugin.emit([
      "id": id,
      "type": "resize",
      "cols": newCols,
      "rows": newRows,
    ])
  }

  func setTerminalTitle(source: TerminalView, title: String) {}
  func hostCurrentDirectoryUpdate(source: TerminalView, directory: String?) {}
  func scrolled(source: TerminalView, position: Double) {}
  func requestOpenLink(source: TerminalView, link: String, params: [String: String]) {}
  func bell(source: TerminalView) {}
  func clipboardCopy(source: TerminalView, content: Data) {}
  func rangeChanged(source: TerminalView, startY: Int, endY: Int) {
    CoduxNativeTerminalPlugin.emit([
      "id": id,
      "type": "selection",
      "data": source.getSelection() ?? "",
    ])
  }
  func clipboardRead(source: TerminalView) -> Data? { nil }
  func iTermContent(source: TerminalView, content: ArraySlice<UInt8>) {}
}

private final class WeakCoduxNativeTerminalView {
  weak var value: CoduxNativeTerminalView?

  init(_ value: CoduxNativeTerminalView) {
    self.value = value
  }
}

private final class CoduxNativeTerminalContainerView: UIView {
  var onLayout: (() -> Void)?

  override func layoutSubviews() {
    super.layoutSubviews()
    onLayout?()
  }
}

private final class CoduxSwiftTermTerminalView: TerminalView {
  private let suppressedInputView = CoduxSuppressedKeyboardView()

  func showSystemKeyboard() {
    if isFirstResponder {
      _ = resignFirstResponder()
    }
    inputView = nil
    inputAccessoryView = nil
    reloadInputViews()
    _ = becomeFirstResponder()
  }

  func hideSystemKeyboard() {
    _ = resignFirstResponder()
    suppressSystemKeyboard()
  }

  func suppressSystemKeyboard() {
    inputView = suppressedInputView
    inputAccessoryView = nil
    reloadInputViews()
  }
}

private final class CoduxSuppressedKeyboardView: UIView {
  init() {
    super.init(frame: .zero)
    backgroundColor = .clear
  }

  required init?(coder: NSCoder) {
    nil
  }

  override var intrinsicContentSize: CGSize {
    CGSize(width: UIView.noIntrinsicMetric, height: 0)
  }

  override func sizeThatFits(_ size: CGSize) -> CGSize {
    CGSize(width: size.width, height: 0)
  }
}

private enum CoduxTerminalFont {
  static var assetPath: String?
  private static var registeredAssetPath: String?

  static func font(size: CGFloat) -> UIFont {
    registerIfNeeded()
    let base = UIFont(name: postScriptName(), size: size)
      ?? UIFont.monospacedSystemFont(ofSize: size, weight: .regular)
    let descriptor = base.fontDescriptor.addingAttributes([
      .featureSettings: [
        [
          UIFontDescriptor.FeatureKey.featureIdentifier: kLigaturesType,
          UIFontDescriptor.FeatureKey.typeIdentifier: kCommonLigaturesOffSelector,
        ],
      ],
    ])
    return UIFont(descriptor: descriptor, size: size)
  }

  private static func registerIfNeeded() {
    guard let path = assetPath, registeredAssetPath != path else { return }
    registeredAssetPath = path
    CTFontManagerRegisterFontsForURL(URL(fileURLWithPath: path) as CFURL, .process, nil)
  }

  private static func postScriptName() -> String {
    return "MapleMono-NF-CN-Regular"
  }
}

private enum CoduxTerminalTheme {
  static let background = UIColor(red: 13 / 255, green: 17 / 255, blue: 23 / 255, alpha: 1)
  static let foreground = UIColor.white

  static let palette: [Color] = [
    color(0x00, 0x00, 0x00),
    color(0xcd, 0x00, 0x00),
    color(0x00, 0xcd, 0x00),
    color(0xcd, 0xcd, 0x00),
    color(0x64, 0x95, 0xed),
    color(0xcd, 0x00, 0xcd),
    color(0x00, 0xcd, 0xcd),
    color(0xe5, 0xe5, 0xe5),
    color(0x7f, 0x7f, 0x7f),
    color(0xff, 0x00, 0x00),
    color(0x00, 0xff, 0x00),
    color(0xff, 0xff, 0x00),
    color(0x5c, 0x5c, 0xff),
    color(0xff, 0x00, 0xff),
    color(0x00, 0xff, 0xff),
    color(0xff, 0xff, 0xff),
  ]

  private static func color(_ red: UInt16, _ green: UInt16, _ blue: UInt16) -> Color {
    Color(red: red * 257, green: green * 257, blue: blue * 257)
  }
}
