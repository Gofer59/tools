package com.trilingua.app.tts

import android.content.Context
import android.media.AudioAttributes
import android.media.AudioFormat
import android.media.AudioTrack
import com.k2fsa.sherpa.onnx.OfflineTts
import com.k2fsa.sherpa.onnx.OfflineTtsConfig
import com.k2fsa.sherpa.onnx.OfflineTtsModelConfig
import com.k2fsa.sherpa.onnx.OfflineTtsVitsModelConfig
import com.trilingua.app.model.Language
import com.trilingua.app.model.TtsSettings
import com.trilingua.app.model.TrilinguaError
import com.trilingua.app.pipeline.PipelineException
import com.trilingua.app.util.Logger
import com.trilingua.app.util.TtsSettingsStore
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.withContext
import java.io.File

/**
 * Piper TTS engine backed by sherpa-onnx.
 * Voice files are under: <voicesRoot>/<voiceDir>/model.onnx + model.onnx.json
 *
 * Thread-safety: [speak] is suspended and runs on [Dispatchers.Default]. [stop] may be
 * called from any thread. Engine instances are cached per language and rebuilt when the
 * user selects a different voice directory.
 */
class PiperTextToSpeechEngine(
    private val context: Context,
    private val voicesRoot: String,
    private val settingsStore: TtsSettingsStore
) : TextToSpeechEngine {

    private val _state = MutableStateFlow<TextToSpeechEngine.State>(TextToSpeechEngine.State.Initializing)
    override val state: StateFlow<TextToSpeechEngine.State> = _state.asStateFlow()

    // Cache per language — keyed by voice directory so a settings change invalidates the entry
    private data class CachedEngine(val voiceDir: String, val noiseScale: Float, val tts: OfflineTts)
    private val ttsCache = mutableMapOf<Language, CachedEngine>()

    private var currentTrack: AudioTrack? = null
    @Volatile private var stopRequested = false

    init {
        // Fail fast if any Language is added without a corresponding default voice entry.
        val missingVoices = Language.values().filter { it !in TtsSettings.DEFAULT_VOICES }
        require(missingVoices.isEmpty()) {
            "PiperTTS: DEFAULT_VOICES missing entries for: ${missingVoices.joinToString()}"
        }
        _state.value = TextToSpeechEngine.State.Ready
    }

    private fun resolveVoiceDir(language: Language, settings: TtsSettings): String =
        settings.voicePerLang[language] ?: TtsSettings.DEFAULT_VOICES[language]!!

    private fun getOrCreateEngine(language: Language, voiceDir: String, noiseScale: Float): OfflineTts? {
        // Return cached entry if voice dir + noiseScale are unchanged.
        // sherpa-onnx applies noiseScale at model-config time, not per-call, so a
        // user-requested noise change requires re-instantiating OfflineTts.
        ttsCache[language]?.let { cached ->
            if (cached.voiceDir == voiceDir && cached.noiseScale == noiseScale) return cached.tts
            cached.tts.release()
            ttsCache.remove(language)
        }

        val dir = File(voicesRoot, voiceDir)
        val modelPath = File(dir, "model.onnx")
        val configPath = File(dir, "model.onnx.json")
        val tokensPath = File(dir, "tokens.txt")
        if (!modelPath.exists() || !configPath.exists() || !tokensPath.exists()) {
            Logger.e("PiperTTS: voice missing at ${dir.absolutePath} (model/config/tokens)")
            _state.value = TextToSpeechEngine.State.VoiceMissing(language)
            return null
        }
        // espeak-ng-data lives in voicesRoot (shared across all Piper voices).
        // dataDir for sherpa-onnx must point AT the espeak-ng-data directory itself
        // (the dir containing `phontab`, `phondata`, language dicts, etc.).
        val dataDir = File(voicesRoot, "espeak-ng-data")
        // Piper models use VITS architecture in sherpa-onnx
        val vitsConfig = OfflineTtsVitsModelConfig(
            model = modelPath.absolutePath,
            tokens = tokensPath.absolutePath,
            dataDir = dataDir.absolutePath,
            noiseScale = noiseScale,
            noiseScaleW = 0.8f,
            lengthScale = 1.0f        // overridden per-call via speed arg
        )
        val modelConfig = OfflineTtsModelConfig(
            vits = vitsConfig,
            numThreads = 2,
            debug = false,
            provider = "cpu"
        )
        val ttsConfig = OfflineTtsConfig(model = modelConfig)
        val tts = OfflineTts(config = ttsConfig)
        ttsCache[language] = CachedEngine(voiceDir, noiseScale, tts)
        Logger.i("PiperTTS: loaded voice $voiceDir")
        return tts
    }

    override suspend fun speak(text: String, language: Language) {
        if (text.isBlank()) {
            Logger.i("PiperTTS: speak called with blank text for $language, returning early")
            return
        }
        // Read current settings before synthesis (non-blocking — collects latest value)
        val settings = settingsStore.settings.first()
        val voiceDir = resolveVoiceDir(language, settings)
        val lengthScale = 1.0f / settings.speed.coerceIn(0.1f, 4.0f)
        val noiseScale  = settings.noiseScale
        val pitch       = settings.pitch.coerceIn(0.5f, 2.0f)

        withContext(Dispatchers.Default) {
            _state.value = TextToSpeechEngine.State.Speaking
            stopRequested = false
            try {
                val tts = getOrCreateEngine(language, voiceDir, noiseScale)
                    ?: throw PipelineException(TrilinguaError.VoiceMissing(language))

                Logger.i("PiperTTS: synthesizing for $language textLen=${text.length} speed=${settings.speed} noiseScale=$noiseScale pitch=$pitch")
                val genStart = System.currentTimeMillis()
                // Note: sherpa-onnx OfflineTts.generate speed param maps to Piper's lengthScale internally.
                // We pass lengthScale-equivalent via the speed float (lengthScale = 1/speed in caller).
                val audio = tts.generate(text = text, sid = 0, speed = lengthScale)
                android.util.Log.d("Trilingua", "[${System.currentTimeMillis()}] PiperTTS: generate done samples=${audio.samples.size} sampleRate=${audio.sampleRate} durationMs=${System.currentTimeMillis()-genStart}")
                if (stopRequested) {
                    _state.value = TextToSpeechEngine.State.Ready
                    return@withContext
                }

                playPcm(audio.samples, audio.sampleRate, pitch)
                android.util.Log.d("Trilingua", "[${System.currentTimeMillis()}] PiperTTS: playPcm complete")
                _state.value = TextToSpeechEngine.State.Ready
            } catch (e: Exception) {
                Logger.e("PiperTTS: speak failed: ${e.message}")
                _state.value = TextToSpeechEngine.State.Error(e.message ?: "speak failed")
                throw e
            }
        }
    }

    private fun playPcm(samples: FloatArray, sampleRate: Int, pitch: Float = 1.0f) {
        val minBuf = AudioTrack.getMinBufferSize(
            sampleRate,
            AudioFormat.CHANNEL_OUT_MONO,
            AudioFormat.ENCODING_PCM_FLOAT
        )
        // N1: guard against invalid buffer size
        if (minBuf < 0) {
            Logger.e("PiperTTS: AudioTrack.getMinBufferSize returned $minBuf — invalid")
            throw PipelineException(TrilinguaError.NativeCrash("audio track buffer-size invalid: $minBuf"))
        }

        val track = AudioTrack.Builder()
            .setAudioAttributes(
                AudioAttributes.Builder()
                    .setUsage(AudioAttributes.USAGE_MEDIA)
                    .setContentType(AudioAttributes.CONTENT_TYPE_SPEECH)
                    .build()
            )
            .setAudioFormat(
                AudioFormat.Builder()
                    .setEncoding(AudioFormat.ENCODING_PCM_FLOAT)
                    .setSampleRate(sampleRate)
                    .setChannelMask(AudioFormat.CHANNEL_OUT_MONO)
                    .build()
            )
            .setBufferSizeInBytes(minBuf)
            .setTransferMode(AudioTrack.MODE_STREAM)
            .build()

        currentTrack = track
        // Apply pitch via PlaybackParams (API 23+, minSdk 26 — safe). Speed is already
        // controlled by Piper's lengthScale, so we only set pitch here.
        runCatching {
            track.playbackParams = android.media.PlaybackParams().apply {
                setPitch(pitch.coerceIn(0.5f, 2.0f))
            }
        }.onFailure { e ->
            Logger.e("PiperTTS: PlaybackParams.setPitch failed (${e.message}) — continuing at default pitch")
        }
        // N2: wrap write loop in try/finally to always release AudioTrack
        try {
            track.play()
            val chunkSize = 4096
            var offset = 0
            while (offset < samples.size && !stopRequested) {
                val end = minOf(offset + chunkSize, samples.size)
                track.write(samples, offset, end - offset, AudioTrack.WRITE_BLOCKING)
                offset = end
            }
        } finally {
            runCatching { track.stop() }
            runCatching { track.release() }
            currentTrack = null
        }
    }

    override fun stop() {
        stopRequested = true
        currentTrack?.stop()
        _state.value = TextToSpeechEngine.State.Ready
    }

    override fun close() {
        stop()
        for (cached in ttsCache.values) {
            cached.tts.release()
        }
        ttsCache.clear()
    }
}
