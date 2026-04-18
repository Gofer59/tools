package com.trilingua.app.stt

import com.trilingua.app.model.Language
import kotlinx.coroutines.flow.StateFlow

interface SpeechRecognizer {
    sealed interface State {
        data object Initializing : State
        data object Ready : State
        data object Busy : State
        data class Error(val message: String) : State
    }
    val state: StateFlow<State>
    suspend fun transcribe(audio: ShortArray, language: Language): String
    fun close()
}
