package com.trilingua.app.pipeline

import android.media.AudioRecord
import android.util.Log
import com.trilingua.app.audio.AudioCapture
import com.trilingua.app.model.*
import com.trilingua.app.mt.Translator
import com.trilingua.app.stt.SpeechRecognizer
import com.trilingua.app.tts.TextToSpeechEngine
import kotlinx.coroutines.*
import kotlinx.coroutines.flow.*

/**
 * Orchestrates the full STT → MT → TTS pipeline for a single translation session.
 *
 * [onMicPressed] starts audio capture; [onMicReleased] stops capture and runs the
 * full pipeline asynchronously. [cancel] interrupts any in-flight stage.
 *
 * Thread-safety: [onMicPressed], [onMicReleased], [cancel] may be called from the UI thread.
 * All heavy work runs in [scope] on Dispatchers.Default/IO.
 */
class TranslationPipeline(
    private val audio: AudioCapture,
    private val stt: SpeechRecognizer,
    private val mt: Translator,
    private val tts: TextToSpeechEngine,
    private val scope: CoroutineScope,
) {
    private val _state = MutableStateFlow<PipelineState>(PipelineState.Idle)
    val uiState: StateFlow<PipelineState> = _state.asStateFlow()

    private val _transcripts = MutableStateFlow(TranscriptPair("", ""))
    val transcripts: StateFlow<TranscriptPair> = _transcripts.asStateFlow()

    private var currentJob: Job? = null
    private var recordJob: Job? = null

    data class TranscriptPair(val source: String, val target: String)

    fun onMicPressed(direction: Direction) {
        if (_state.value !is PipelineState.Idle && _state.value !is PipelineState.Done
            && _state.value !is PipelineState.Failed) return
        _transcripts.value = TranscriptPair("", "")
        recordJob = scope.launch {
            try {
                Log.d("Trilingua", "[${System.currentTimeMillis()}] Pipeline.onMicPressed direction=${direction.id}")
                _state.value = PipelineState.Recording(0L)
                audio.start()
            } catch (se: SecurityException) {
                // Mic permission revoked mid-session
                Log.w("Trilingua", "[${System.currentTimeMillis()}] Pipeline.onMicPressed SecurityException: ${se.message}")
                _state.value = PipelineState.Failed(TrilinguaError.MicDenied)
            } catch (t: Throwable) {
                Log.w("Trilingua", "[${System.currentTimeMillis()}] Pipeline.onMicPressed error: ${t.message}")
                _state.value = PipelineState.Failed(TrilinguaError.NativeCrash(t.message ?: "audio"))
            }
        }
    }

    fun onMicReleased(direction: Direction) {
        val rj = recordJob ?: run {
            // recordJob is null — mic was released before recording started; reset to Idle
            Log.w("Trilingua", "[${System.currentTimeMillis()}] Pipeline.onMicReleased: recordJob=null, resetting to Idle")
            _state.value = PipelineState.Idle
            return
        }
        Log.d("Trilingua", "[${System.currentTimeMillis()}] Pipeline.onMicReleased direction=${direction.id}")
        currentJob = scope.launch {
            rj.join()
            val samples = try {
                audio.stop()
            } catch (se: SecurityException) {
                // Mic permission revoked between start and stop
                Log.w("Trilingua", "[${System.currentTimeMillis()}] Pipeline: audio.stop() SecurityException: ${se.message}")
                _state.value = PipelineState.Failed(TrilinguaError.MicDenied)
                return@launch
            } catch (t: Throwable) {
                Log.w("Trilingua", "[${System.currentTimeMillis()}] Pipeline: audio.stop() error: ${t.message}")
                _state.value = PipelineState.Failed(TrilinguaError.NativeCrash(t.message ?: "audio"))
                return@launch
            }
            // Surface AudioRecord.read() errors as typed failures
            val readErr = audio.getReadErrorCode()
            if (readErr != 0) {
                // All negative AudioRecord.read codes indicate mic-access or session failure
                // (ERROR=-1, ERROR_BAD_VALUE=-2, ERROR_INVALID_OPERATION=-3, ERROR_DEAD_OBJECT=-6).
                // Treat uniformly as MicDenied so OEMs with aggressive denoise don't mislead users.
                val trilinguaError = if (readErr < 0) TrilinguaError.MicDenied
                else TrilinguaError.NativeCrash("AudioRecord.read error=$readErr")
                Log.w("Trilingua", "[${System.currentTimeMillis()}] Pipeline: AudioRecord read error=$readErr -> $trilinguaError")
                _state.value = PipelineState.Failed(trilinguaError)
                return@launch
            }
            val wasTruncated = audio.wasTruncated()
            Log.d("Trilingua", "[${System.currentTimeMillis()}] Pipeline: samples=${samples.size} wasTruncated=$wasTruncated")
            if (samples.size < AudioCapture.MIN_SAMPLES) {
                _state.value = PipelineState.Failed(TrilinguaError.TooShort); return@launch
            }
            try {
                val stageStart = System.currentTimeMillis()
                _state.value = PipelineState.Transcribing(direction.from)
                Log.d("Trilingua", "[${System.currentTimeMillis()}] Pipeline: stage=Transcribing samples=${samples.size}")
                val src = withContext(Dispatchers.Default) { stt.transcribe(samples, direction.from) }
                Log.d("Trilingua", "[${System.currentTimeMillis()}] Pipeline: Transcribing done srcLen=${src.length} durationMs=${System.currentTimeMillis()-stageStart}")
                _transcripts.update { it.copy(source = src) }

                val transStart = System.currentTimeMillis()
                _state.value = PipelineState.Translating(direction)
                Log.d("Trilingua", "[${System.currentTimeMillis()}] Pipeline: stage=Translating srcLen=${src.length}")
                val tgt = withContext(Dispatchers.Default) { mt.translate(src, direction.from, direction.to) }
                Log.d("Trilingua", "[${System.currentTimeMillis()}] Pipeline: Translating done tgtLen=${tgt.length} durationMs=${System.currentTimeMillis()-transStart}")
                _transcripts.update { it.copy(target = tgt) }

                val speakStart = System.currentTimeMillis()
                _state.value = PipelineState.Speaking(direction.to)
                Log.d("Trilingua", "[${System.currentTimeMillis()}] Pipeline: stage=Speaking tgtLen=${tgt.length}")
                tts.speak(tgt, direction.to)
                Log.d("Trilingua", "[${System.currentTimeMillis()}] Pipeline: Speaking done durationMs=${System.currentTimeMillis()-speakStart}")
                // TooLong is informational: translation completed but recording was capped
                if (wasTruncated) {
                    _state.value = PipelineState.Failed(TrilinguaError.TooLong)
                } else {
                    _state.value = PipelineState.Done
                }
            } catch (ce: CancellationException) { throw ce
            } catch (pe: PipelineException) {
                Log.w("Trilingua", "[${System.currentTimeMillis()}] Pipeline: PipelineException ${pe.error}")
                _state.value = PipelineState.Failed(pe.error)
            } catch (t: Throwable) {
                Log.w("Trilingua", "[${System.currentTimeMillis()}] Pipeline: unexpected error ${t.message}")
                _state.value = PipelineState.Failed(TrilinguaError.NativeCrash(t.message ?: "pipeline"))
            }
        }
    }

    fun cancel() {
        recordJob?.cancel(); currentJob?.cancel()
        runCatching { audio.abort() }
        tts.stop()
        _state.value = PipelineState.Idle
    }
}
