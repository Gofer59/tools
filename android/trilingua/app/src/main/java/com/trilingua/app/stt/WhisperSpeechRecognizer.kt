package com.trilingua.app.stt

import com.trilingua.app.model.Language
import com.trilingua.app.model.TrilinguaError
import com.trilingua.app.nativebridge.Whisper
import com.trilingua.app.pipeline.PipelineException
import com.trilingua.app.util.Logger
import java.io.File
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock

/**
 * Whisper-based offline speech recogniser backed by the native whisper_jni bridge.
 * Thread-safe: a [Mutex] serialises concurrent [transcribe] calls (Whisper context is not reentrant).
 */
class WhisperSpeechRecognizer(private val modelPath: String) : SpeechRecognizer {

    private val _state = MutableStateFlow<SpeechRecognizer.State>(SpeechRecognizer.State.Initializing)
    override val state: StateFlow<SpeechRecognizer.State> = _state.asStateFlow()

    private var whisper: Whisper? = null
    private val mutex = Mutex()

    init {
        if (!File(modelPath).exists()) {
            Logger.e("WhisperSpeechRecognizer: model missing at $modelPath")
            _state.value = SpeechRecognizer.State.Error("model missing")
        } else {
            try {
                whisper = Whisper.open(modelPath)
                _state.value = SpeechRecognizer.State.Ready
                Logger.i("WhisperSpeechRecognizer: initialized from $modelPath")
            } catch (e: Exception) {
                Logger.e("WhisperSpeechRecognizer: init failed: ${e.message}")
                _state.value = SpeechRecognizer.State.Error(e.message ?: "init failed")
            }
        }
    }

    override suspend fun transcribe(audio: ShortArray, language: Language): String {
        return mutex.withLock {
            val w = whisper ?: throw PipelineException(TrilinguaError.ModelMissing("stt:whisper-small"))
            _state.value = SpeechRecognizer.State.Busy
            try {
                val langTag = language.tag
                val result = w.transcribe(audio, langTag)
                _state.value = SpeechRecognizer.State.Ready
                result.trim()
            } catch (e: Exception) {
                Logger.e("WhisperSpeechRecognizer: transcribe failed: ${e.message}")
                _state.value = SpeechRecognizer.State.Error(e.message ?: "transcribe failed")
                throw e
            }
        }
    }

    override fun close() {
        whisper?.close()
        whisper = null
        _state.value = SpeechRecognizer.State.Error("closed")
    }
}
