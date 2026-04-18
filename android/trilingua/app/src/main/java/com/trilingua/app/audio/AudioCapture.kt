package com.trilingua.app.audio

import android.Manifest
import android.media.AudioFormat
import android.media.AudioRecord
import android.media.MediaRecorder
import android.util.Log
import androidx.annotation.RequiresPermission
import kotlinx.coroutines.*

/**
 * Captures mono 16-bit PCM audio from the device microphone.
 *
 * Thread-safety: [start] and [stop]/[abort] must be called from a single logical thread
 * (e.g. the pipeline coroutine). The internal capture loop runs on [parentScope].
 *
 * [parentScope] is used as the parent for the recording coroutine so that it is
 * cancelled automatically when the application scope is cancelled.
 */
class AudioCapture(private val parentScope: CoroutineScope) {
    private var recorder: AudioRecord? = null
    // Accumulate raw ShortArray chunks to avoid per-sample boxing
    private val chunks = mutableListOf<ShortArray>()
    private var recordingJob: Job? = null
    private var startedAtMs: Long = 0L
    @Volatile private var hitMaxDuration: Boolean = false
    @Volatile private var readErrorCode: Int = 0

    /** Maximum consecutive zero-reads before aborting (nominal ~500 ms at 5 ms sleep; actual wall time depends on mic driver read latency). */
    private val MAX_ZERO_READS = 100

    @RequiresPermission(Manifest.permission.RECORD_AUDIO)
    fun start() {
        if (recorder != null) return
        val minBuf = AudioRecord.getMinBufferSize(
            AudioConfig.SAMPLE_RATE_HZ,
            AudioFormat.CHANNEL_IN_MONO,
            AudioFormat.ENCODING_PCM_16BIT
        ).coerceAtLeast(AudioConfig.SAMPLE_RATE_HZ * 2)
        val ar = try {
            AudioRecord(
                MediaRecorder.AudioSource.VOICE_RECOGNITION,
                AudioConfig.SAMPLE_RATE_HZ,
                AudioFormat.CHANNEL_IN_MONO,
                AudioFormat.ENCODING_PCM_16BIT,
                minBuf
            )
        } catch (e: Exception) {
            // Fallback to MIC source on OEMs that reject VOICE_RECOGNITION
            AudioRecord(
                MediaRecorder.AudioSource.MIC,
                AudioConfig.SAMPLE_RATE_HZ,
                AudioFormat.CHANNEL_IN_MONO,
                AudioFormat.ENCODING_PCM_16BIT,
                minBuf
            )
        }
        if (ar.state != AudioRecord.STATE_INITIALIZED) {
            ar.release()
            throw SecurityException("AudioRecord not initialized; check RECORD_AUDIO or AudioSource fallback")
        }
        Log.d("Trilingua", "[${System.currentTimeMillis()}] AudioCapture.start: minBuf=$minBuf audioSource=${ar.audioSource} state=${ar.state}")
        synchronized(chunks) { chunks.clear() }
        hitMaxDuration = false
        readErrorCode = 0
        ar.startRecording()
        if (ar.recordingState != AudioRecord.RECORDSTATE_RECORDING) {
            ar.release()
            throw SecurityException("AudioRecord failed to start recording; recordingState=${ar.recordingState}")
        }
        recorder = ar
        startedAtMs = System.currentTimeMillis()
        Log.d("Trilingua", "[${System.currentTimeMillis()}] AudioCapture.start: recording started, recordingState=${ar.recordingState}")
        recordingJob = parentScope.launch(Dispatchers.IO) {
            val scratch = ShortArray(minBuf / 2)
            val loggedErrorCodes = mutableSetOf<Int>()
            var consecutiveZeros = 0
            while (isActive) {
                val n = ar.read(scratch, 0, scratch.size)
                if (n > 0) {
                    consecutiveZeros = 0
                    val chunk = scratch.copyOf(n)
                    synchronized(chunks) { chunks.add(chunk) }
                } else if (n < 0) {
                    if (loggedErrorCodes.add(n)) {
                        Log.w("Trilingua", "[${System.currentTimeMillis()}] AudioCapture: read returned n=$n, breaking capture loop")
                    }
                    readErrorCode = n
                    break
                } else {
                    // n == 0: no data available yet. Yield to avoid burning a core on
                    // misbehaving OEM mic drivers that return 0 instead of blocking.
                    consecutiveZeros++
                    if (consecutiveZeros >= MAX_ZERO_READS) {
                        Log.w("Trilingua", "[${System.currentTimeMillis()}] AudioCapture: $MAX_ZERO_READS consecutive zero-reads, aborting (denoise path stuck?)")
                        readErrorCode = -42 // custom: zero-read timeout
                        break
                    }
                    kotlinx.coroutines.delay(5)
                }
                if (System.currentTimeMillis() - startedAtMs > AudioConfig.MAX_DURATION_MS) {
                    hitMaxDuration = true
                    break
                }
            }
        }
    }

    suspend fun stop(): ShortArray {
        recordingJob?.cancelAndJoin()
        // recorder.stop() throws IllegalStateException if never successfully started
        recorder?.runCatching { stop() }; recorder?.release(); recorder = null
        return synchronized(chunks) {
            val totalSize = chunks.sumOf { it.size }
            val durationMs = if (startedAtMs > 0L) System.currentTimeMillis() - startedAtMs else 0L
            Log.d("Trilingua", "[${System.currentTimeMillis()}] AudioCapture.stop: totalSamples=$totalSize durationMs=$durationMs wasTruncated=$hitMaxDuration readErrorCode=$readErrorCode")
            val result = ShortArray(totalSize)
            var offset = 0
            for (chunk in chunks) {
                chunk.copyInto(result, offset)
                offset += chunk.size
            }
            result
        }
    }

    /** Returns true if the last recording was capped by MAX_DURATION_MS. */
    fun wasTruncated(): Boolean = hitMaxDuration

    fun abort() {
        recordingJob?.cancel()
        recorder?.runCatching { stop(); release() }
        recorder = null
        synchronized(chunks) { chunks.clear() }
        hitMaxDuration = false
        readErrorCode = 0
    }

    /** Returns the last AudioRecord.read() error code (< 0) or 0 if no error. */
    fun getReadErrorCode(): Int = readErrorCode

    companion object {
        const val MIN_SAMPLES = (AudioConfig.SAMPLE_RATE_HZ * AudioConfig.MIN_DURATION_MS / 1000).toInt()
    }
}
