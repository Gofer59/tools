package com.example.bookreader

import android.content.Context
import android.speech.tts.TextToSpeech
import android.speech.tts.UtteranceProgressListener
import com.example.bookreader.model.TtsState
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import java.util.Locale

/**
 * Wraps Android's [TextToSpeech] engine with [StateFlow]-based state management.
 *
 * Supports French and English locales, switchable at runtime via [setLocale].
 * The active locale is set both at init AND per [speak] call to prevent locale drift
 * on devices with multiple TTS languages installed.
 */
class TtsManager(context: Context) : TextToSpeech.OnInitListener {

    private val tts = TextToSpeech(context.applicationContext, this)

    private val _ttsState = MutableStateFlow<TtsState>(TtsState.Initializing)
    val ttsState: StateFlow<TtsState> = _ttsState.asStateFlow()

    private val _currentLocale = MutableStateFlow(Locale("fr", "FR"))
    val currentLocale: StateFlow<Locale> = _currentLocale.asStateFlow()

    private var engineReady = false

    @Volatile
    private var currentRate: Float = 1.0f

    fun setSpeechRate(rate: Float) {
        currentRate = rate
    }

    override fun onInit(status: Int) {
        // Register listener BEFORE setting Ready state to avoid race where speak()
        // is called before callbacks are wired up
        tts.setOnUtteranceProgressListener(object : UtteranceProgressListener() {
            override fun onStart(utteranceId: String?) {
                _ttsState.value = TtsState.Speaking
            }

            override fun onDone(utteranceId: String?) {
                _ttsState.value = TtsState.Ready
            }

            @Deprecated("Deprecated in API level 21")
            override fun onError(utteranceId: String?) {
                _ttsState.value = TtsState.Ready
            }

            override fun onError(utteranceId: String?, errorCode: Int) {
                _ttsState.value = TtsState.Ready
            }
        })

        if (status == TextToSpeech.SUCCESS) {
            engineReady = true
            applyLocale(_currentLocale.value)
        } else {
            _ttsState.value = TtsState.Error("Échec d'initialisation du moteur vocal")
        }
    }

    /**
     * Switches TTS locale between French and English.
     * Checks voice availability and updates state accordingly.
     */
    fun setLocale(locale: Locale) {
        _currentLocale.value = locale
        if (engineReady) {
            applyLocale(locale)
        }
    }

    private fun applyLocale(locale: Locale) {
        val result = tts.setLanguage(locale)
        if (result == TextToSpeech.LANG_MISSING_DATA || result == TextToSpeech.LANG_NOT_SUPPORTED) {
            _ttsState.value = TtsState.MissingVoice
        } else {
            _ttsState.value = TtsState.Ready
        }
    }

    /**
     * Speaks the given [text] in the current locale.
     * Re-sets the locale before each call to prevent drift.
     */
    fun speak(text: String) {
        val currentState = _ttsState.value
        if (currentState != TtsState.Ready && currentState != TtsState.Speaking) return

        tts.setLanguage(_currentLocale.value)
        tts.setSpeechRate(currentRate)
        tts.speak(text, TextToSpeech.QUEUE_FLUSH, null, "bookreader_utterance")
    }

    fun stop() {
        tts.stop()
        if (_ttsState.value == TtsState.Speaking) {
            _ttsState.value = TtsState.Ready
        }
    }

    fun shutdown() {
        tts.stop()
        tts.shutdown()
    }
}
