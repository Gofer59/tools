package com.trilingua.app.nativebridge

class Ct2Translator private constructor(private var handle: Long) : AutoCloseable {

    /**
     * Translate [text] using the SPM processors already cached in the native handle.
     * SPM paths are no longer needed per call — they were loaded once in [open].
     */
    fun translate(
        text: String,
        beamSize: Int = 4,
        maxDecodingLength: Int = 256
    ): String {
        check(handle != 0L) { "CT2 translator closed" }
        return nativeTranslate(handle, text, beamSize, maxDecodingLength)
    }

    override fun close() {
        if (handle != 0L) { nativeClose(handle); handle = 0L }
    }

    private external fun nativeTranslate(
        handle: Long,
        text: String,
        beamSize: Int,
        maxDecodingLength: Int
    ): String

    private external fun nativeClose(handle: Long)

    companion object {
        init {
            System.loadLibrary("ctranslate2")
            System.loadLibrary("ct2_jni")
        }

        @JvmStatic
        external fun nativeOpen(
            modelDir: String,
            srcSpmPath: String,
            tgtSpmPath: String,
            numThreads: Int
        ): Long

        /**
         * Opens a CT2 translator handle, loading both SPM processors once.
         */
        fun open(modelDir: String, srcSpmPath: String, tgtSpmPath: String, numThreads: Int = 2): Ct2Translator {
            val h = nativeOpen(modelDir, srcSpmPath, tgtSpmPath, numThreads)
            require(h != 0L) { "CT2 open failed for $modelDir" }
            return Ct2Translator(h)
        }
    }
}
