package com.codux.native_terminal

import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context
import android.view.View
import com.codux.mobile.terminal.CoduxTerminalLog
import com.termux.view.RemoteTerminalView
import io.flutter.plugin.common.BinaryMessenger
import io.flutter.plugin.common.EventChannel
import io.flutter.plugin.common.MethodCall
import io.flutter.plugin.common.MethodChannel
import io.flutter.plugin.platform.PlatformView

class CoduxTerminalPlatformView(
    context: Context,
    messenger: BinaryMessenger,
    viewId: Int,
) : PlatformView, MethodChannel.MethodCallHandler, EventChannel.StreamHandler {
    companion object {
        const val viewType = "codux_native_terminal/terminal_view"
    }

    private val terminalView = RemoteTerminalView(context)
    private val methodChannel = MethodChannel(
        messenger,
        "codux_native_terminal/terminal_view_$viewId/methods",
    )
    private val eventChannel = EventChannel(
        messenger,
        "codux_native_terminal/terminal_view_$viewId/events",
    )
    private var events: EventChannel.EventSink? = null
    private val clipboard = context.getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager
    private var lastResizeEvent: Pair<Int, Int>? = null

    init {
        terminalView.setRemoteClient(object : RemoteTerminalView.RemoteClient {
            override fun onInput(data: String) {
                CoduxTerminalLog.d("CoduxNativeTerminal", "input bytes=${data.toByteArray().size} data=${debugData(data)}")
                events?.success(mapOf("type" to "input", "data" to data))
            }

            override fun onTerminalResponse(data: String) {
                CoduxTerminalLog.d("CoduxNativeTerminal", "response bytes=${data.toByteArray().size} data=${debugData(data)}")
                events?.success(mapOf("type" to "response", "data" to data))
            }

            override fun onResize(columns: Int, rows: Int) {
                emitResize(columns, rows)
            }
        })
        terminalView.setScreenMetricsListener { rows, cursorRow, cursorBottomPx, historyRows, topRow ->
            events?.success(
                mapOf(
                    "type" to "metrics",
                    "rows" to rows,
                    "cursorRow" to cursorRow,
                    "cursorBottomPx" to cursorBottomPx,
                    "historyRows" to historyRows,
                    "topRow" to topRow,
                ),
            )
        }
        methodChannel.setMethodCallHandler(this)
        eventChannel.setStreamHandler(this)
    }

    override fun getView(): View = terminalView

    override fun onMethodCall(call: MethodCall, result: MethodChannel.Result) {
        when (call.method) {
            "write" -> {
                val data = call.argument<String>("data").orEmpty()
                CoduxTerminalLog.d("CoduxNativeTerminal", "write bytes=${data.toByteArray().size} data=${debugData(data)}")
                terminalView.writeRemote(data)
                result.success(null)
            }
            "replace" -> {
                val data = call.argument<String>("data").orEmpty()
                CoduxTerminalLog.d("CoduxNativeTerminal", "replace bytes=${data.toByteArray().size}")
                terminalView.clearRemote()
                terminalView.writeRemote(data)
                result.success(null)
            }
            "clear" -> {
                terminalView.clearRemote()
                result.success(null)
            }
            "focusKeyboard" -> {
                terminalView.focusAndShowKeyboard()
                result.success(null)
            }
            "hideKeyboard" -> {
                terminalView.hideKeyboard()
                result.success(null)
            }
            "setScrollEnabled" -> {
                terminalView.setTouchScrollEnabled(call.argument<Boolean>("enabled") != false)
                result.success(null)
            }
            "copySelection" -> {
                val selectedText = terminalView.selectedText
                if (selectedText.isBlank()) {
                    result.success(false)
                } else {
                    clipboard.setPrimaryClip(ClipData.newPlainText("terminal-selection", selectedText))
                    terminalView.stopTextSelectionMode()
                    result.success(true)
                }
            }
            "resize" -> {
                terminalView.post {
                    terminalView.updateSize()
                    emitCurrentResize(force = true)
                }
                result.success(null)
            }
            "setLogLevel" -> {
                CoduxTerminalLog.setLevel(call.argument<String>("level").orEmpty())
                result.success(null)
            }
            else -> result.notImplemented()
        }
    }

    override fun onListen(arguments: Any?, eventSink: EventChannel.EventSink?) {
        events = eventSink
        CoduxTerminalLog.d("CoduxNativeTerminal", "events attached")
        terminalView.post {
            terminalView.updateSize()
            emitCurrentResize(force = true)
        }
    }

    override fun onCancel(arguments: Any?) {
        events = null
    }

    override fun dispose() {
        methodChannel.setMethodCallHandler(null)
        eventChannel.setStreamHandler(null)
        terminalView.setRemoteClient(null)
        events = null
    }

    private fun emitCurrentResize(force: Boolean = false) {
        val columns = terminalView.terminalColumns
        val rows = terminalView.terminalRows
        if (columns <= 0 || rows <= 0) return
        emitResize(columns, rows, force = force)
    }

    private fun emitResize(columns: Int, rows: Int, force: Boolean = false) {
        if (columns <= 0 || rows <= 0) return
        val next = Pair(columns, rows)
        if (!force && lastResizeEvent == next) return
        lastResizeEvent = next
        CoduxTerminalLog.d("CoduxNativeTerminal", "emit current resize cols=$columns rows=$rows")
        events?.success(
            mapOf(
                "type" to "resize",
                "cols" to columns,
                "rows" to rows,
            ),
        )
    }

    private fun debugData(data: String): String {
        val maxLength = 160
        val text = if (data.length > maxLength) data.take(maxLength) + "…" else data
        return text
            .replace("\u001B", "<ESC>")
            .replace("\r", "<CR>")
            .replace("\n", "<LF>")
            .replace("\t", "<TAB>")
    }
}
