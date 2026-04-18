package com.trilingua.app.tts

import com.trilingua.app.model.Language
import kotlinx.coroutines.flow.StateFlow

interface TextToSpeechEngine {
    sealed interface State {
        data object Initializing : State
        data object Ready : State
        data object Speaking : State
        data class VoiceMissing(val language: Language) : State
        data class Error(val message: String) : State
    }
    val state: StateFlow<State>
    suspend fun speak(text: String, language: Language)
    fun stop()
    fun close()
}
