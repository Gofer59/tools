package com.trilingua.app.util

import android.content.Context
import java.io.File

/**
 * Copies assets/{whisper,mt,tts} to filesDir on first run.
 * Integrity: after extracting each file, writes a companion <file>.sha256 sidecar.
 *   On subsequent boots, if the file exists but the sidecar SHA mismatches the on-disk
 *   content, the file is re-extracted. Sidecar SHA256 detects post-extraction disk corruption;
 *   does NOT validate the integrity of bundled APK assets (those are guaranteed by the APK
 *   signature). Sidecar mismatch triggers re-extraction from assets.
 * Version marker: filesDir/models.v3.ok written when all files pass.
 *   v3: bumped from v2 to force re-extraction after CT2 config.json + vocabulary were added.
 */
class AssetExtractor(private val context: Context) {

    private val filesDir: File get() = context.filesDir
    private val markerFile: File get() = File(filesDir, "models.v5.ok")

    fun whisperModelPath(): String =
        File(filesDir, "whisper/ggml-base-q5_1.bin").absolutePath

    fun mtModelsRoot(): String =
        File(filesDir, "mt").absolutePath

    fun ttsVoicesRoot(): String =
        File(filesDir, "tts").absolutePath

    /** Returns true if extraction succeeded or was already done. */
    suspend fun ensureExtracted(onProgress: (Float) -> Unit): Boolean {
        // Remove stale v1 and v2 markers if present so they don't linger
        val staleMarkers = listOf("models.v1.ok", "models.v2.ok", "models.v3.ok", "models.v4.ok")
        val hasStaleMarker = staleMarkers.any { File(filesDir, it).exists() }
        staleMarkers.forEach { File(filesDir, it).let { f -> if (f.exists()) f.delete() } }
        // When upgrading from an older layout (pre-v5), purge legacy files so new assets
        // extract cleanly. Do NOT touch tts/ or whisper/ when already on v5 — they are valid.
        if (hasStaleMarker) {
            File(filesDir, "whisper/ggml-small-q5_1.bin").let { if (it.exists()) it.delete() }
            File(filesDir, "tts").deleteRecursively()
        }
        if (markerFile.exists()) {
            Logger.i("AssetExtractor: marker found, skipping extraction")
            return true
        }
        return try {
            val assetManager = context.assets
            val roots = listOf("whisper", "mt", "tts")

            // Count total files for progress
            val allFiles = mutableListOf<String>()
            for (root in roots) collectAssets(assetManager, root, allFiles)
            val total = allFiles.size.coerceAtLeast(1)

            Logger.i("AssetExtractor: extracting $total files")
            var done = 0

            for (path in allFiles) {
                val dest = File(filesDir, path)
                dest.parentFile?.mkdirs()

                val needsExtract = when {
                    !dest.exists() || dest.length() == 0L -> true
                    else -> !verifySha256Sidecar(dest)
                }

                if (needsExtract) {
                    assetManager.open(path).use { input ->
                        dest.outputStream().use { output ->
                            val buf = ByteArray(1024 * 1024)
                            var n: Int
                            while (input.read(buf).also { n = it } != -1) {
                                output.write(buf, 0, n)
                            }
                        }
                    }
                    writeSha256Sidecar(dest)
                    Logger.d("AssetExtractor: extracted $path (${dest.length()} bytes)")
                } else {
                    Logger.d("AssetExtractor: skipping $path (exists, SHA256 OK)")
                }
                done++
                onProgress(done.toFloat() / total)
            }

            markerFile.writeText("ok")
            Logger.i("AssetExtractor: extraction complete")
            true
        } catch (e: Exception) {
            Logger.e("AssetExtractor: extraction failed: ${e.message}", e)
            false
        }
    }

    /** Write a .sha256 sidecar file next to [dest] with the hex digest. */
    private fun writeSha256Sidecar(dest: File) {
        val sha = Sha256.ofFile(dest)
        File(dest.absolutePath + ".sha256").writeText(sha)
    }

    /**
     * Returns true if a .sha256 sidecar exists and the hash matches the current file contents.
     * Returns false if the sidecar is absent or the hash mismatches (triggers re-extraction).
     */
    private fun verifySha256Sidecar(dest: File): Boolean {
        val sidecar = File(dest.absolutePath + ".sha256")
        if (!sidecar.exists()) return false
        return try {
            val expected = sidecar.readText().trim()
            val actual = Sha256.ofFile(dest)
            if (expected == actual) true
            else {
                Logger.w("AssetExtractor: SHA256 mismatch for ${dest.name} — re-extracting")
                false
            }
        } catch (e: Exception) {
            Logger.w("AssetExtractor: SHA256 check failed for ${dest.name}: ${e.message}")
            false
        }
    }

    private fun collectAssets(assetManager: android.content.res.AssetManager, path: String, result: MutableList<String>) {
        val children = assetManager.list(path) ?: return
        if (children.isEmpty()) {
            // It's a file
            result.add(path)
        } else {
            for (child in children) {
                collectAssets(assetManager, "$path/$child", result)
            }
        }
    }
}
