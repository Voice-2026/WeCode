package com.duxweb.codux.terminal

import android.content.Context
import io.flutter.embedding.engine.FlutterEngine
import io.flutter.plugin.common.BinaryMessenger
import io.flutter.plugin.common.EventChannel
import io.flutter.plugin.common.MethodCall
import io.flutter.plugin.common.MethodChannel
import io.flutter.plugin.platform.PlatformView
import io.flutter.plugin.platform.PlatformViewFactory
import io.flutter.plugin.common.StandardMessageCodec

object CoduxNativeTerminalPlugin {
    private const val VIEW_TYPE = "codux/native_terminal"
    private const val METHOD_CHANNEL = "codux/native_terminal/methods"
    private const val EVENT_CHANNEL = "codux/native_terminal/events"

    private val views = mutableMapOf<Int, CoduxRemoteTerminalView>()
    private var eventSink: EventChannel.EventSink? = null

    fun register(flutterEngine: FlutterEngine) {
        val messenger = flutterEngine.dartExecutor.binaryMessenger
        flutterEngine
            .platformViewsController
            .registry
            .registerViewFactory(VIEW_TYPE, NativeTerminalViewFactory(messenger))

        MethodChannel(messenger, METHOD_CHANNEL).setMethodCallHandler { call, result ->
            handleMethod(call, result)
        }
        EventChannel(messenger, EVENT_CHANNEL).setStreamHandler(
            object : EventChannel.StreamHandler {
                override fun onListen(arguments: Any?, events: EventChannel.EventSink?) {
                    eventSink = events
                }

                override fun onCancel(arguments: Any?) {
                    eventSink = null
                }
            },
        )
    }

    private fun handleMethod(call: MethodCall, result: MethodChannel.Result) {
        val id = call.argument<Int>("id")
        val view = id?.let { views[it] }
        if (view == null) {
            result.success(false)
            return
        }
        when (call.method) {
            "feed" -> {
                view.feed(call.argument<String>("data").orEmpty())
                result.success(true)
            }
            "replace" -> {
                view.replace(call.argument<String>("data").orEmpty())
                result.success(true)
            }
            "reset" -> {
                view.reset()
                result.success(true)
            }
            "setFontSize" -> {
                view.setTerminalFontSize(call.argument<Double>("fontSize") ?: 14.0)
                result.success(true)
            }
            "sendKey" -> {
                view.sendKey(call.argument<String>("key").orEmpty())
                result.success(true)
            }
            "focus" -> {
                view.requestFocus()
                result.success(true)
            }
            "showKeyboard" -> {
                view.showKeyboard()
                result.success(true)
            }
            "hideKeyboard" -> {
                view.hideKeyboard()
                result.success(true)
            }
            else -> result.notImplemented()
        }
    }

    private class NativeTerminalViewFactory(
        private val messenger: BinaryMessenger,
    ) : PlatformViewFactory(StandardMessageCodec.INSTANCE) {
        override fun create(context: Context, viewId: Int, args: Any?): PlatformView {
            val nativeView = CoduxRemoteTerminalView(
                context,
                object : CoduxRemoteTerminalView.Callback {
                    override fun onInput(data: String) {
                        emit(mapOf("id" to viewId, "type" to "input", "data" to data))
                    }

                    override fun onSelectionChanged(text: String?) {
                        emit(
                            mapOf(
                                "id" to viewId,
                                "type" to "selection",
                                "data" to text.orEmpty(),
                            ),
                        )
                    }

                    override fun onResize(cols: Int, rows: Int) {
                        emit(
                            mapOf(
                                "id" to viewId,
                                "type" to "resize",
                                "cols" to cols,
                                "rows" to rows,
                            ),
                        )
                    }
                },
            )
            views[viewId] = nativeView
            val params = args as? Map<*, *>
            val fontSize = (params?.get("fontSize") as? Number)?.toDouble()
            if (fontSize != null) nativeView.setTerminalFontSize(fontSize)
            return object : PlatformView {
                override fun getView() = nativeView
                override fun dispose() {
                    views.remove(viewId)
                }
            }
        }
    }

    private fun emit(event: Map<String, Any>) {
        eventSink?.success(event)
    }
}
