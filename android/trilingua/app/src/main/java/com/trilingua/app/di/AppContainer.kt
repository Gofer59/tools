package com.trilingua.app.di

import android.content.Context
import android.util.Log
import com.trilingua.app.audio.AudioCapture
import com.trilingua.app.mt.Ct2Translator
import com.trilingua.app.pipeline.TranslationPipeline
import com.trilingua.app.stt.WhisperSpeechRecognizer
import com.trilingua.app.tts.PiperTextToSpeechEngine
import com.trilingua.app.util.AssetExtractor
import com.trilingua.app.util.TtsSettingsStore
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob

/**
 * Manual DI container scoped to the Application lifecycle.
 * All engine instances are lazily initialised — they must only be accessed after
 * [AssetExtractor.ensureExtracted] completes, which writes model files to disk.
 */
class AppContainer(private val app: Context) {
    init {
        // whisper_jni and ct2_jni are loaded by the companion objects of the nativebridge
        // classes (Whisper.kt / Ct2Translator.kt) on first class reference — no duplicate
        // System.loadLibrary calls needed here for those two.
        //
        // Load sherpa-onnx libs explicitly so missing-library crashes surface here (before the
        // first TTS call), decoupled from OfflineTts class-loading order.
        try {
            System.loadLibrary("onnxruntime")
            Log.i("Trilingua", "AppContainer: onnxruntime loaded OK")
        } catch (e: UnsatisfiedLinkError) {
            Log.e("Trilingua", "AppContainer: onnxruntime load FAILED: ${e.message}")
        }
        try {
            System.loadLibrary("sherpa-onnx-jni")
            Log.i("Trilingua", "AppContainer: sherpa-onnx-jni loaded OK")
        } catch (e: UnsatisfiedLinkError) {
            Log.e("Trilingua", "AppContainer: sherpa-onnx-jni load FAILED: ${e.message}")
        }
    }

    val appScope = CoroutineScope(SupervisorJob() + Dispatchers.Default)
    val assetExtractor = AssetExtractor(app)
    val ttsSettingsStore = TtsSettingsStore(app)
    val audio = AudioCapture(appScope)
    val stt by lazy { WhisperSpeechRecognizer(assetExtractor.whisperModelPath()) }
    val mt  by lazy { Ct2Translator(assetExtractor.mtModelsRoot()) }
    val tts by lazy { PiperTextToSpeechEngine(app, assetExtractor.ttsVoicesRoot(), ttsSettingsStore) }
    val pipeline by lazy { TranslationPipeline(audio, stt, mt, tts, appScope) }
}
