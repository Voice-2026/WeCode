package com.duxweb.codux

import android.os.Build
import android.os.Bundle
import android.view.ViewGroup
import com.duxweb.codux.terminal.CoduxNativeTerminalPlugin
import io.flutter.embedding.android.FlutterActivity
import io.flutter.embedding.engine.FlutterEngine

class MainActivity : FlutterActivity() {
    override fun configureFlutterEngine(flutterEngine: FlutterEngine) {
        super.configureFlutterEngine(flutterEngine)
        CoduxNativeTerminalPlugin.register(flutterEngine)
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        disableDefaultFocusHighlight(window.decorView)
    }

    private fun disableDefaultFocusHighlight(view: android.view.View) {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            view.defaultFocusHighlightEnabled = false
        }
        if (view is ViewGroup) {
            for (index in 0 until view.childCount) {
                disableDefaultFocusHighlight(view.getChildAt(index))
            }
        }
    }
}
