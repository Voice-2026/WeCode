package com.codux.native_terminal

import io.flutter.embedding.engine.plugins.FlutterPlugin
import io.flutter.plugin.common.BinaryMessenger
import io.flutter.plugin.common.StandardMessageCodec
import io.flutter.plugin.platform.PlatformView
import io.flutter.plugin.platform.PlatformViewFactory
import android.content.Context

class CoduxNativeTerminalPlugin : FlutterPlugin {
    override fun onAttachedToEngine(binding: FlutterPlugin.FlutterPluginBinding) {
        binding.platformViewRegistry.registerViewFactory(
            CoduxTerminalPlatformView.viewType,
            CoduxTerminalViewFactory(binding.binaryMessenger),
        )
    }

    override fun onDetachedFromEngine(binding: FlutterPlugin.FlutterPluginBinding) = Unit
}

private class CoduxTerminalViewFactory(
    private val messenger: BinaryMessenger,
) : PlatformViewFactory(StandardMessageCodec.INSTANCE) {
    override fun create(context: Context, viewId: Int, args: Any?): PlatformView {
        return CoduxTerminalPlatformView(context, messenger, viewId)
    }
}
