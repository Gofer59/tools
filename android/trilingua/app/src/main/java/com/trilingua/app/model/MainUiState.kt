package com.trilingua.app.model

/**
 * Immutable UI state snapshot for MainScreen.
 * All mutations go through [MainViewModel._ui.update] — never mutate directly.
 * [normalize] must be called after any language change to enforce source ≠ target.
 */
sealed interface BootState {
    data object Initializing : BootState
    data class Extracting(val pct: Float) : BootState
    data object Ready : BootState
    data object Failed : BootState
}

data class MainUiState(
    val source: Language,
    val target: Language,
    val bootState: BootState,
    val pipelineState: PipelineState,
    val sourceText: String,
    val targetText: String,
    val error: TrilinguaError?,
    val showSettings: Boolean = false,
    val transientMessage: String? = null
) {
    val direction: Direction get() = Direction(source, target)

    val isInteractive: Boolean
        get() = bootState is BootState.Ready &&
                (pipelineState is PipelineState.Idle ||
                 pipelineState is PipelineState.Done ||
                 pipelineState is PipelineState.Failed)

    val isMicEnabled: Boolean
        get() = bootState is BootState.Ready &&
                (pipelineState is PipelineState.Idle ||
                 pipelineState is PipelineState.Done ||
                 pipelineState is PipelineState.Failed ||
                 pipelineState is PipelineState.Recording)

    val canCancel: Boolean
        get() = bootState is BootState.Ready &&
                (pipelineState is PipelineState.Recording ||
                 pipelineState is PipelineState.Transcribing ||
                 pipelineState is PipelineState.Translating ||
                 pipelineState is PipelineState.Speaking)

    fun normalize(): MainUiState {
        return if (source == target) {
            val newTarget = Language.values().first { it != source }
            copy(target = newTarget)
        } else this
    }

    companion object {
        fun initial() = MainUiState(
            source = Language.EN,
            target = Language.HU,
            bootState = BootState.Initializing,
            pipelineState = PipelineState.Idle,
            sourceText = "",
            targetText = "",
            error = null
        )
    }
}
