package com.codux.mobile.terminal;

import android.util.Log;

public final class CoduxTerminalLog {
    public static final int DEBUG = 10;
    public static final int INFO = 20;
    public static final int WARN = 30;
    public static final int ERROR = 40;
    public static final int OFF = 100;

    private static volatile int level = WARN;

    private CoduxTerminalLog() {}

    public static void setLevel(String value) {
        if (value == null) {
            level = WARN;
            return;
        }
        switch (value.trim().toLowerCase()) {
            case "debug":
                level = DEBUG;
                break;
            case "info":
                level = INFO;
                break;
            case "warn":
            case "warning":
                level = WARN;
                break;
            case "error":
                level = ERROR;
                break;
            case "off":
            case "none":
                level = OFF;
                break;
            default:
                level = WARN;
                break;
        }
    }

    public static void d(String tag, String message) {
        if (enabled(DEBUG)) Log.d(tag, message);
    }

    public static void i(String tag, String message) {
        if (enabled(INFO)) Log.i(tag, message);
    }

    public static void w(String tag, String message) {
        if (enabled(WARN)) Log.w(tag, message);
    }

    public static void e(String tag, String message) {
        if (enabled(ERROR)) Log.e(tag, message);
    }

    public static void e(String tag, String message, Throwable error) {
        if (enabled(ERROR)) Log.e(tag, message, error);
    }

    private static boolean enabled(int messageLevel) {
        return level != OFF && messageLevel >= level;
    }
}
