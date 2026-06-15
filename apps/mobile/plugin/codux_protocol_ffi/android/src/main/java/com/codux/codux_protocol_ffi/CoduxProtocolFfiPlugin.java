package com.codux.codux_protocol_ffi;

import android.content.Context;
import android.util.Log;
import androidx.annotation.NonNull;
import io.flutter.embedding.engine.plugins.FlutterPlugin;

public final class CoduxProtocolFfiPlugin implements FlutterPlugin {
  private static final String TAG = "CoduxProtocolFfiPlugin";

  static {
    System.loadLibrary("codux_protocol_ffi");
  }

  private static native boolean initAndroidContext(Context context);

  @Override
  public void onAttachedToEngine(@NonNull FlutterPluginBinding binding) {
    Context context = binding.getApplicationContext();
    if (!initAndroidContext(context.getApplicationContext())) {
      Log.e(TAG, "failed to initialize Android context for Codux protocol FFI");
    }
  }

  @Override
  public void onDetachedFromEngine(@NonNull FlutterPluginBinding binding) {}
}
