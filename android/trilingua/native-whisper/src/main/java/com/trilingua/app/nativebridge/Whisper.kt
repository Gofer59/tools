package com.trilingua.app.nativebridge

class Whisper private constructor(private var ctx: Long) : AutoCloseable {

    external fun nativeTranscribe(ctx: Long, samples: ShortArray, languageTag: String, nThreads: Int): String

    fun transcribe(samples: ShortArray, languageTag: String, nThreads: Int = 4): String {
        check(ctx != 0L) { "Whisper context closed" }
        return nativeTranscribe(ctx, samples, languageTag, nThreads)
    }

    override fun close() {
        if (ctx != 0L) {
            nativeFree(ctx)
            ctx = 0L
        }
    }

    private external fun nativeFree(ctx: Long)

    companion object {
        init { System.loadLibrary("whisper_jni") }

        @JvmStatic
        external fun nativeInit(modelPath: String): Long

        fun open(modelPath: String): Whisper {
            val c = nativeInit(modelPath)
            require(c != 0L) { "whisper init failed for $modelPath" }
            return Whisper(c)
        }
    }
}
