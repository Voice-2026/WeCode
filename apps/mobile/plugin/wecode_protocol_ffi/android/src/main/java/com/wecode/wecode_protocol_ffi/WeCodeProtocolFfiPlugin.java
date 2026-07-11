package com.wecode.wecode_protocol_ffi;

import android.content.Context;
import android.util.Log;
import androidx.annotation.NonNull;
import io.flutter.embedding.engine.plugins.FlutterPlugin;

public final class WeCodeProtocolFfiPlugin implements FlutterPlugin {
  private static final String TAG = "WeCodeProtocolFfiPlugin";

  static {
    System.loadLibrary("wecode_protocol_ffi");
  }

  private static native boolean initAndroidContext(Context context);

  @Override
  public void onAttachedToEngine(@NonNull FlutterPluginBinding binding) {
    Context context = binding.getApplicationContext();
    if (!initAndroidContext(context.getApplicationContext())) {
      Log.e(TAG, "failed to initialize Android context for WeCode protocol FFI");
    }
  }

  @Override
  public void onDetachedFromEngine(@NonNull FlutterPluginBinding binding) {}
}
