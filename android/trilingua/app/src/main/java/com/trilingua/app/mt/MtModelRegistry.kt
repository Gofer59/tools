package com.trilingua.app.mt

import com.trilingua.app.model.Direction
import java.io.File

/**
 * Maps a Direction to the on-disk directory containing the converted CTranslate2 model.
 * Models are expected at: <modelsRoot>/opus-mt-<from>-<to>/
 */
object MtModelRegistry {
    fun modelDir(modelsRoot: String, direction: Direction): File {
        val dirName = "opus-mt-${direction.from.tag}-${direction.to.tag}"
        return File(modelsRoot, dirName)
    }

    fun spmPath(modelDir: File, side: String): String {
        // Helsinki OPUS-MT standard: source.spm / target.spm
        val named = File(modelDir, "$side.spm")
        if (named.exists()) return named.absolutePath
        // Generic fallback — log a warning; spm.model is not expected in bundled assets
        android.util.Log.w("Trilingua", "MtModelRegistry: ${named.absolutePath} not found, falling back to spm.model (unexpected)")
        return File(modelDir, "spm.model").absolutePath
    }
}
