package com.trilingua.app.mt

import com.trilingua.app.model.Direction
import com.trilingua.app.model.Language
import com.trilingua.app.model.TrilinguaError
import com.trilingua.app.nativebridge.Ct2Translator as NativeCt2
import com.trilingua.app.pipeline.PipelineException
import com.trilingua.app.util.Logger
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import java.io.File
import java.util.concurrent.ConcurrentHashMap
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock

/**
 * High-level Kotlin wrapper around the JNI Ct2Translator.
 * Holds one handle per language pair; lazy-initializes on first use.
 */
class Ct2Translator(private val mtModelsRoot: String) : Translator {

    init {
        // Warn at construction time if any supported direction is missing its model directory.
        // Models may not yet be extracted on first boot; this is a non-fatal advisory log.
        val missing = Direction.supported.filter { dir ->
            !MtModelRegistry.modelDir(mtModelsRoot, dir).exists()
        }
        if (missing.isNotEmpty()) {
            Logger.e("Ct2Translator: missing model dirs: ${missing.joinToString { it.id }}")
        }
    }

    // Cache: direction id -> native handle (lazy loaded)
    private val handles = ConcurrentHashMap<String, NativeCt2>()
    // Mutex guards handle creation to prevent races on ConcurrentHashMap.getOrPut
    private val openMutex = Mutex()

    private suspend fun getHandle(direction: Direction): NativeCt2 {
        handles[direction.id]?.let { return it }
        return openMutex.withLock {
            // Double-check inside the lock
            handles[direction.id] ?: run {
                val dir = MtModelRegistry.modelDir(mtModelsRoot, direction)
                if (!dir.exists() || !dir.isDirectory) {
                    throw PipelineException(TrilinguaError.ModelMissing("mt:${direction.id}"))
                }
                val cfgFile = File(dir, "config.json")
                if (!cfgFile.exists()) {
                    throw PipelineException(TrilinguaError.ModelMissing("mt:${direction.id}:config.json"))
                }
                val srcSpm = MtModelRegistry.spmPath(dir, "source")
                val tgtSpm = MtModelRegistry.spmPath(dir, "target")
                if (!File(srcSpm).exists()) {
                    throw PipelineException(TrilinguaError.ModelMissing("mt:${direction.id}:source.spm"))
                }
                if (!File(tgtSpm).exists()) {
                    throw PipelineException(TrilinguaError.ModelMissing("mt:${direction.id}:target.spm"))
                }
                val openStart = System.currentTimeMillis()
                Logger.i("Ct2Translator: opening model at ${dir.absolutePath} srcSpm=$srcSpm tgtSpm=$tgtSpm")
                val handle = NativeCt2.open(
                    modelDir = dir.absolutePath,
                    srcSpmPath = srcSpm,
                    tgtSpmPath = tgtSpm,
                    numThreads = 2
                )
                android.util.Log.d("Trilingua", "[${System.currentTimeMillis()}] Ct2Translator: open complete durationMs=${System.currentTimeMillis()-openStart}")
                handles[direction.id] = handle
                handle
            }
        }
    }

    override suspend fun translate(text: String, from: Language, to: Language): String {
        return withContext(Dispatchers.Default) {
            val direction = Direction(from, to)
            // Guard: Direction.init already rejects from==to, and Direction.supported covers all
            // valid constructor-arguments — so this branch is normally unreachable. Keep it as a
            // forward-compatibility safety net if Direction.supported is ever narrowed.
            if (!Direction.supported.contains(direction)) {
                throw PipelineException(TrilinguaError.UnsupportedPair(direction))
            }
            val handle = getHandle(direction)
            val transStart = System.currentTimeMillis()
            android.util.Log.d("Trilingua", "[${System.currentTimeMillis()}] Ct2Translator: translate ${from.tag}->${to.tag} inputLen=${text.length}")
            val result = handle.translate(text = text)
            android.util.Log.d("Trilingua", "[${System.currentTimeMillis()}] Ct2Translator: translate done outputLen=${result.length} durationMs=${System.currentTimeMillis()-transStart}")
            result
        }
    }

    override fun close() {
        for ((id, h) in handles) {
            Logger.i("Ct2Translator: closing handle $id")
            h.close()
        }
        handles.clear()
    }
}
