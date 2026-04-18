package com.trilingua.app.model

/**
 * State machine for a single translation pipeline run.
 * Transitions: Idle → Recording → Transcribing → Translating → Speaking → Done|Failed.
 * [cancel] returns the pipeline to Idle from any state.
 */
sealed interface PipelineState {
    data object Idle : PipelineState
    data class Recording(val elapsedMs: Long) : PipelineState
    data class Transcribing(val from: Language) : PipelineState
    data class Translating(val direction: Direction) : PipelineState
    data class Speaking(val to: Language) : PipelineState
    data object Done : PipelineState
    data class Failed(val error: TrilinguaError) : PipelineState
}
