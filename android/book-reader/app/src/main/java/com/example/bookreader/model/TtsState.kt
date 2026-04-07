package com.example.bookreader.model

/**
 * Represents the text-to-speech engine state.
 * - [Initializing]: TTS engine is loading (speak() must not be called)
 * - [Ready]: voice available for current locale, ready to speak
 * - [Speaking]: currently reading text aloud
 * - [MissingVoice]: no voice installed for current locale — UI should show install prompt
 * - [Error]: TTS engine failed to initialize
 */
sealed interface TtsState {
    data object Initializing : TtsState
    data object Ready : TtsState
    data object Speaking : TtsState
    data object MissingVoice : TtsState
    data class Error(val message: String) : TtsState
}
