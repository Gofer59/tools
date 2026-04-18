package com.trilingua.app.util

import android.util.Log

object Logger {
    private const val TAG = "Trilingua"

    fun i(msg: String) = Log.i(TAG, msg)
    fun d(msg: String) = Log.d(TAG, msg)
    fun w(msg: String) = Log.w(TAG, msg)
    fun e(msg: String) = Log.e(TAG, msg)
    fun e(msg: String, t: Throwable) = Log.e(TAG, msg, t)
}
