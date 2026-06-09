import Flutter
import GhosttyTerminal
import ObjectiveC.runtime
import UIKit

public final class CoduxNativeTerminalPlugin: NSObject, FlutterPlugin {
    public static func register(with registrar: FlutterPluginRegistrar) {
        let factory = CoduxTerminalPlatformViewFactory(messenger: registrar.messenger())
        registrar.register(factory, withId: "codux_native_terminal/terminal_view")
    }
}

private final class CoduxTerminalPlatformViewFactory: NSObject, FlutterPlatformViewFactory {
    private let messenger: FlutterBinaryMessenger

    init(messenger: FlutterBinaryMessenger) {
        self.messenger = messenger
        super.init()
    }

    func create(
        withFrame frame: CGRect,
        viewIdentifier viewId: Int64,
        arguments args: Any?
    ) -> FlutterPlatformView {
        CoduxTerminalPlatformView(frame: frame, viewId: viewId, messenger: messenger)
    }

    func createArgsCodec() -> FlutterMessageCodec & NSObjectProtocol {
        FlutterStandardMessageCodec.sharedInstance()
    }
}

@MainActor
private final class CoduxTerminalDelegate:
    TerminalSurfaceGridResizeDelegate,
    TerminalSurfaceLifecycleDelegate,
    TerminalSurfaceFocusDelegate,
    TerminalSurfaceBellDelegate,
    TerminalSurfaceTitleDelegate,
    TerminalSurfaceCloseDelegate,
    TerminalSurfaceTextSelectionRequestDelegate
{
    weak var owner: CoduxGhosttyTerminalHostView?

    init(owner: CoduxGhosttyTerminalHostView) {
        self.owner = owner
    }

    func terminalDidAttachSurface(_ surface: TerminalSurface) {
        owner?.surface = surface
        owner?.emitCurrentResize()
    }

    func terminalDidDetachSurface() {
        owner?.surface = nil
    }

    func terminalDidResize(_ size: TerminalGridMetrics) {
        owner?.emitResize(size)
        owner?.emitMetrics(size)
    }

    func terminalDidChangeFocus(_ focused: Bool) {}

    func terminalDidRingBell() {}

    func terminalDidChangeTitle(_ title: String) {}

    func terminalDidClose(processAlive: Bool) {}

    func terminalDidRequestTextSelection(_ request: TerminalTextSelectionRequest) {}
}

@MainActor
private final class CoduxGhosttyTerminalHostView: UIView {
    private let terminalView = TerminalView(frame: .zero)
    private let eventSinkProvider: () -> FlutterEventSink?
    private let session: InMemoryTerminalSession
    private let terminalController: TerminalController
    private var delegateProxy: CoduxTerminalDelegate?
    private var lastResize: (cols: Int, rows: Int)?
    private var lastMetrics: TerminalGridMetrics?
    private let terminalResponseParser = CoduxTerminalResponseParser()

    weak var surface: TerminalSurface?
    var scrollEnabled = true {
        didSet { updateScrollGestureState() }
    }

    init(frame: CGRect, eventSinkProvider: @escaping () -> FlutterEventSink?) {
        self.eventSinkProvider = eventSinkProvider

        let sessionBox = CoduxSessionBox()
        session = InMemoryTerminalSession(
            write: { data in
                Task { @MainActor in
                    sessionBox.owner?.emitInput(data)
                }
            },
            resize: { viewport in
                Task { @MainActor in
                    sessionBox.owner?.emitResize(cols: Int(viewport.columns), rows: Int(viewport.rows))
                }
            }
        )

        let coduxTheme = TerminalConfiguration()
            .background("0D1117")
            .foreground("E6EDF3")
            .selectionBackground("30363D")
            .cursorColor("00B8D9")
            .palette(0, color: "0D1117")
            .palette(1, color: "FF7B72")
            .palette(2, color: "3FB950")
            .palette(3, color: "D29922")
            .palette(4, color: "58A6FF")
            .palette(5, color: "BC8CFF")
            .palette(6, color: "39C5CF")
            .palette(7, color: "B1BAC4")
            .palette(8, color: "6E7681")
            .palette(9, color: "FFA198")
            .palette(10, color: "56D364")
            .palette(11, color: "E3B341")
            .palette(12, color: "79C0FF")
            .palette(13, color: "D2A8FF")
            .palette(14, color: "56D4DD")
            .palette(15, color: "F0F6FC")

        terminalController = TerminalController(
            theme: TerminalTheme(light: coduxTheme, dark: coduxTheme)
        ) { builder in
            builder.withFontSize(10)
            builder.withWindowPaddingX(8)
            builder.withWindowPaddingY(6)
        }

        super.init(frame: frame)

        sessionBox.owner = self
        overrideUserInterfaceStyle = .dark
        terminalController.setColorScheme(.dark)
        backgroundColor = UIColor(red: 0.051, green: 0.067, blue: 0.09, alpha: 1)
        isOpaque = true

        terminalView.translatesAutoresizingMaskIntoConstraints = false
        configureTerminalResponder(terminalView)
        terminalView.backgroundColor = .clear
        terminalView.isOpaque = false
        terminalView.controller = terminalController
        terminalView.configuration = TerminalSurfaceOptions(backend: .inMemory(session), fontSize: 10)
        addSubview(terminalView)
        NSLayoutConstraint.activate([
            terminalView.leadingAnchor.constraint(equalTo: leadingAnchor),
            terminalView.trailingAnchor.constraint(equalTo: trailingAnchor),
            terminalView.topAnchor.constraint(equalTo: topAnchor),
            terminalView.bottomAnchor.constraint(equalTo: bottomAnchor),
        ])

        let proxy = CoduxTerminalDelegate(owner: self)
        delegateProxy = proxy
        terminalView.delegate = proxy
        updateScrollGestureState()
    }

    @available(*, unavailable)
    required init?(coder: NSCoder) {
        fatalError("init(coder:) has not been implemented")
    }

    func write(_ text: String) {
        session.receive(text)
    }

    func replace(_ text: String) {
        terminalResponseParser.reset()
        if text.isEmpty {
            session.receive("\u{001B}[2J\u{001B}[3J\u{001B}[H")
        } else {
            session.receive("\u{001B}[2J\u{001B}[3J\u{001B}[H" + text)
        }
    }

    func clear() {
        terminalResponseParser.reset()
        session.receive("\u{001B}[2J\u{001B}[3J\u{001B}[H")
    }

    func focusKeyboard() {
        allowNextTerminalKeyboardFocus(terminalView)
        defer { disallowTerminalKeyboardFocus(terminalView) }
        terminalView.becomeFirstResponder()
    }

    func hideKeyboard() {
        disallowTerminalKeyboardFocus(terminalView)
        terminalView.resignFirstResponder()
    }

    func requestResize() {
        terminalView.setNeedsLayout()
        terminalView.layoutIfNeeded()
        emitCurrentResize()
    }

    func copySelectionToPasteboard() -> Bool {
        terminalView.copy(nil)
        return true
    }

    func emitInput(_ data: Data) {
        guard !data.isEmpty else { return }
        let text = String(decoding: data, as: UTF8.self)
        let input = terminalResponseParser.consume(text)
        if !input.isEmpty {
            emit(["type": "input", "data": input])
        }
    }

    func emitResize(_ size: TerminalGridMetrics) {
        emitResize(cols: Int(size.columns), rows: Int(size.rows))
        lastMetrics = size
    }

    func emitResize(cols: Int, rows: Int) {
        guard cols > 0, rows > 0 else { return }
        if let lastResize, lastResize.cols == cols, lastResize.rows == rows {
            return
        }
        lastResize = (cols, rows)
        emit(["type": "resize", "cols": cols, "rows": rows])
    }

    func emitMetrics(_ size: TerminalGridMetrics) {
        lastMetrics = size
        let cursorBottom = Int(size.rows) * Int(size.cellHeightPixels)
        emit([
            "type": "metrics",
            "rows": Int(size.rows),
            "cursorRow": Int(size.rows),
            "cursorBottomPx": cursorBottom,
            "historyRows": 0,
            "topRow": 0,
        ])
    }

    func emitCurrentResize() {
        if let lastMetrics {
            emitResize(lastMetrics)
            emitMetrics(lastMetrics)
        }
    }

    private func updateScrollGestureState() {
        for recognizer in terminalView.gestureRecognizers ?? [] {
            if recognizer is UIPanGestureRecognizer {
                recognizer.isEnabled = scrollEnabled
            }
        }
    }

    private func emit(_ event: [String: Any]) {
        eventSinkProvider()?(event)
    }
}

private final class CoduxTerminalResponseParser {
    private enum State {
        case ground
        case escape
        case csi
        case osc
        case oscEscape
        case string
        case stringEscape
    }

    private var state = State.ground
    private var sequence = ""
    private let maxSequenceLength = 4096

    func consume(_ data: String) -> String {
        var output = ""
        for scalar in data.unicodeScalars {
            let value = scalar.value
            switch state {
            case .ground:
                if value == 0x1B {
                    start(scalar)
                    state = .escape
                } else {
                    output.unicodeScalars.append(scalar)
                }
            case .escape:
                append(scalar)
                if value == 0x5B {
                    state = .csi
                } else if value == 0x5D {
                    state = .osc
                } else if value == 0x50 || value == 0x5E || value == 0x5F {
                    state = .string
                } else {
                    output += sequence
                    reset()
                }
            case .csi:
                append(scalar)
                if isFinalByte(value) {
                    output += sequence
                    reset()
                } else if sequence.count > maxSequenceLength {
                    reset()
                }
            case .osc:
                append(scalar)
                if value == 0x07 {
                    reset()
                } else if value == 0x1B {
                    state = .oscEscape
                } else if sequence.count > maxSequenceLength {
                    reset()
                }
            case .oscEscape:
                append(scalar)
                if value == 0x5C {
                    reset()
                } else {
                    state = .osc
                }
            case .string:
                append(scalar)
                if value == 0x1B {
                    state = .stringEscape
                } else if sequence.count > maxSequenceLength {
                    reset()
                }
            case .stringEscape:
                append(scalar)
                if value == 0x5C {
                    reset()
                } else {
                    state = .string
                }
            }
        }
        return output
    }

    func reset() {
        sequence.removeAll(keepingCapacity: true)
        state = .ground
    }

    private func start(_ scalar: UnicodeScalar) {
        sequence.removeAll(keepingCapacity: true)
        sequence.unicodeScalars.append(scalar)
    }

    private func append(_ scalar: UnicodeScalar) {
        sequence.unicodeScalars.append(scalar)
    }

    private func isFinalByte(_ value: UInt32) -> Bool {
        value >= 0x40 && value <= 0x7E
    }
}

private final class CoduxSessionBox: @unchecked Sendable {
    @MainActor weak var owner: CoduxGhosttyTerminalHostView?
}

private final class CoduxTerminalPlatformView: NSObject, FlutterPlatformView, FlutterStreamHandler {
    private let methods: FlutterMethodChannel
    private let events: FlutterEventChannel
    private var eventSink: FlutterEventSink?
    private let terminalView: UIView

    init(frame: CGRect, viewId: Int64, messenger: FlutterBinaryMessenger) {
        methods = FlutterMethodChannel(
            name: "codux_native_terminal/terminal_view_\(viewId)/methods",
            binaryMessenger: messenger
        )
        events = FlutterEventChannel(
            name: "codux_native_terminal/terminal_view_\(viewId)/events",
            binaryMessenger: messenger
        )

        let sinkBox = CoduxEventSinkBox()
        let view = MainActor.assumeIsolated {
            CoduxGhosttyTerminalHostView(frame: frame) {
                sinkBox.sink
            }
        }
        terminalView = view
        super.init()
        events.setStreamHandler(self)
        methods.setMethodCallHandler { [weak self] call, result in
            self?.handle(call: call, result: result)
        }
        sinkBox.sink = { [weak self] event in
            self?.eventSink?(event)
        }
    }

    func view() -> UIView {
        terminalView
    }

    func onListen(withArguments arguments: Any?, eventSink events: @escaping FlutterEventSink) -> FlutterError? {
        Task { @MainActor in
            eventSink = events
            if let terminal = terminalView as? CoduxGhosttyTerminalHostView {
                terminal.emitCurrentResize()
            }
        }
        return nil
    }

    func onCancel(withArguments arguments: Any?) -> FlutterError? {
        Task { @MainActor in
            eventSink = nil
        }
        return nil
    }

    private func handle(call: FlutterMethodCall, result: @escaping FlutterResult) {
        Task { @MainActor in
            guard let terminal = terminalView as? CoduxGhosttyTerminalHostView else {
                result(nil)
                return
            }

            switch call.method {
            case "write":
                let args = call.arguments as? [String: Any]
                let data = args?["data"] as? String ?? ""
                terminal.write(data)
                result(nil)
            case "replace":
                let args = call.arguments as? [String: Any]
                let data = args?["data"] as? String ?? ""
                terminal.replace(data)
                result(nil)
            case "clear":
                terminal.clear()
                result(nil)
            case "focusKeyboard":
                terminal.focusKeyboard()
                result(nil)
            case "hideKeyboard":
                terminal.hideKeyboard()
                result(nil)
            case "setScrollEnabled":
                let args = call.arguments as? [String: Any]
                terminal.scrollEnabled = (args?["enabled"] as? Bool) != false
                result(nil)
            case "copySelection":
                result(terminal.copySelectionToPasteboard())
            case "resize":
                terminal.requestResize()
                result(nil)
            case "setLogLevel":
                result(nil)
            default:
                result(FlutterError(code: "not_implemented", message: "Method not implemented", details: call.method))
            }
        }
    }

    deinit {
        methods.setMethodCallHandler(nil)
        events.setStreamHandler(nil)
    }
}

private final class CoduxEventSinkBox {
    var sink: FlutterEventSink?
}

private var coduxKeyboardFocusAllowedKey: UInt8 = 0

private func configureTerminalResponder(_ view: UIView) {
    let inputAccessorySelector = #selector(getter: UIResponder.inputAccessoryView)
    let canBecomeSelector = #selector(getter: UIResponder.canBecomeFirstResponder)
    let className = "\(NSStringFromClass(type(of: view)))_CoduxResponder"
    let subclass: AnyClass
    if let existing = NSClassFromString(className) {
        subclass = existing
    } else {
        guard let created = objc_allocateClassPair(type(of: view), className, 0) else {
            return
        }
        let inputAccessoryBlock: @convention(block) (AnyObject) -> UIView? = { _ in nil }
        class_addMethod(
            created,
            inputAccessorySelector,
            imp_implementationWithBlock(inputAccessoryBlock),
            "@@:"
        )
        let canBecomeBlock: @convention(block) (AnyObject) -> Bool = { object in
            let allowed = objc_getAssociatedObject(object, &coduxKeyboardFocusAllowedKey) as? Bool
            return allowed == true
        }
        class_addMethod(
            created,
            canBecomeSelector,
            imp_implementationWithBlock(canBecomeBlock),
            "B@:"
        )
        objc_registerClassPair(created)
        subclass = created
    }
    object_setClass(view, subclass)
    disallowTerminalKeyboardFocus(view)
}

private func allowNextTerminalKeyboardFocus(_ view: UIView) {
    objc_setAssociatedObject(view, &coduxKeyboardFocusAllowedKey, true, .OBJC_ASSOCIATION_RETAIN_NONATOMIC)
}

private func disallowTerminalKeyboardFocus(_ view: UIView) {
    objc_setAssociatedObject(view, &coduxKeyboardFocusAllowedKey, false, .OBJC_ASSOCIATION_RETAIN_NONATOMIC)
}
