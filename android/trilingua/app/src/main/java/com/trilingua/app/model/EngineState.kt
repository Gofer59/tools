package com.trilingua.app.model

sealed interface EngineState {
    data object Initializing : EngineState
    data object Ready : EngineState
    data object Busy : EngineState
    data class Error(val message: String) : EngineState
}
