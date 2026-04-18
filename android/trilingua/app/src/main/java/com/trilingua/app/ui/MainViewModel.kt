package com.trilingua.app.ui

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.trilingua.app.di.AppContainer
import com.trilingua.app.model.BootState
import com.trilingua.app.model.Language
import com.trilingua.app.model.MainUiState
import com.trilingua.app.model.PipelineState
import com.trilingua.app.model.TrilinguaError
import com.trilingua.app.model.TtsSettings
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.SharingStarted
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.stateIn
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch

/**
 * Main ViewModel for the Trilingua app.
 *
 * Owns [MainUiState], coordinates asset extraction on boot, and delegates mic/pipeline
 * actions to [AppContainer]. Thread-safe: all state mutations go through [_ui.update].
 *
 * Lifecycle: tied to the Activity via [viewModelScope]. Pipeline subscriptions are
 * launched as children of [viewModelScope] and cancel automatically on ViewModel.clear().
 */
class MainViewModel(private val c: AppContainer) : ViewModel() {
    private val _ui = MutableStateFlow(MainUiState.initial())
    val uiState: StateFlow<MainUiState> = _ui

    /** Current TTS settings, persisted via DataStore. */
    val ttsSettings: StateFlow<TtsSettings> = c.ttsSettingsStore.settings.stateIn(
        scope = viewModelScope,
        started = SharingStarted.Eagerly,
        initialValue = TtsSettings.DEFAULT
    )

    /** Persist updated TTS settings. */
    fun saveTtsSettings(settings: TtsSettings) {
        viewModelScope.launch { c.ttsSettingsStore.update(settings) }
    }

    init {
        startExtraction()
    }

    private fun startExtraction() {
        viewModelScope.launch(Dispatchers.IO) {
            val ok = c.assetExtractor.ensureExtracted { pct ->
                _ui.update { it.copy(bootState = BootState.Extracting(pct)) }
            }
            if (!ok) {
                _ui.update {
                    it.copy(
                        bootState = BootState.Failed,
                        error = TrilinguaError.ModelMissing("boot:extraction")
                    )
                }
                return@launch
            }
            // Engines construct ONLY after extraction completes. Referencing c.pipeline
            // here triggers stt/mt/tts lazy init when model files are on disk.
            c.stt; c.mt; c.tts
            val pipeline = c.pipeline
            _ui.update { it.copy(bootState = BootState.Ready) }

            launch {
                pipeline.uiState.collect { ps ->
                    android.util.Log.d("Trilingua", "[${System.currentTimeMillis()}] VM pipelineState -> $ps")
                    _ui.update { it.copy(pipelineState = ps) }
                    if (ps is PipelineState.Failed) {
                        _ui.update { it.copy(error = ps.error) }
                    }
                }
            }
            launch {
                pipeline.transcripts.collect { t ->
                    _ui.update { it.copy(sourceText = t.source, targetText = t.target) }
                }
            }
        }
    }

    /** Re-run asset extraction after a previous [BootState.Failed]. */
    fun retryExtraction() {
        _ui.update { it.copy(bootState = BootState.Initializing, error = null) }
        startExtraction()
    }

    fun setSource(l: Language) { _ui.update { it.copy(source = l).normalize() } }
    fun setTarget(l: Language) { _ui.update { it.copy(target = l).normalize() } }

    fun swap() {
        _ui.update { s ->
            val active = s.pipelineState !is PipelineState.Idle &&
                         s.pipelineState !is PipelineState.Done &&
                         s.pipelineState !is PipelineState.Failed
            s.copy(
                source     = s.target,
                target     = s.source,
                sourceText = if (active) "" else s.targetText,
                targetText = if (active) "" else s.sourceText
            ).normalize()
        }
    }

    fun pressMic() {
        android.util.Log.d("Trilingua", "[${System.currentTimeMillis()}] VM.pressMic direction=${_ui.value.direction.id}")
        c.pipeline.onMicPressed(_ui.value.direction)
    }

    fun releaseMic() {
        android.util.Log.d("Trilingua", "[${System.currentTimeMillis()}] VM.releaseMic direction=${_ui.value.direction.id}")
        c.pipeline.onMicReleased(_ui.value.direction)
    }

    fun cancel() {
        android.util.Log.d("Trilingua", "[${System.currentTimeMillis()}] VM.cancel state=${_ui.value.pipelineState}")
        c.pipeline.cancel()
    }

    fun dismissError() { _ui.update { it.copy(error = null) } }
    fun setError(e: TrilinguaError) { _ui.update { it.copy(error = e) } }

    fun openSettings()  { _ui.update { it.copy(showSettings = true) } }
    fun closeSettings() { _ui.update { it.copy(showSettings = false) } }

    /**
     * Show a transient message (Snackbar) that auto-dismisses after [durationMs].
     */
    fun showTransientMessage(msg: String, durationMs: Long = 3000L) {
        _ui.update { it.copy(transientMessage = msg) }
        viewModelScope.launch {
            delay(durationMs)
            _ui.update { if (it.transientMessage == msg) it.copy(transientMessage = null) else it }
        }
    }

    fun clearTransientMessage() { _ui.update { it.copy(transientMessage = null) } }
}
