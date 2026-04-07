package com.example.bookreader

import android.app.Application
import android.graphics.Bitmap
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import com.example.bookreader.model.CameraState
import com.example.bookreader.model.TtsState
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch

/**
 * Central state machine for the BookReader camera-to-TTS pipeline.
 *
 * State flow: LivePreview → Processing → Frozen(bitmap, regions) → LivePreview
 *
 * Design decisions:
 * - Uses [AndroidViewModel] for application context access (needed by [TtsManager]).
 * - Bitmap lifecycle: only [CameraState.Frozen] holds a bitmap reference.
 *   [clear] replaces state with [CameraState.LivePreview], making the bitmap GC-eligible.
 *   [HighlightOverlayView] stores only region metadata, never the bitmap itself.
 * - TTS is initialized eagerly so it's ready by the time the user wants to read.
 * - Reading order for selected text: top-to-bottom, then left-to-right (same sort
 *   as OCR output, but re-applied to selected subset for correctness).
 */
class CameraViewModel(application: Application) : AndroidViewModel(application) {

    private val _cameraState = MutableStateFlow<CameraState>(CameraState.LivePreview)
    val cameraState: StateFlow<CameraState> = _cameraState.asStateFlow()

    val ttsManager = TtsManager(application.applicationContext)
    val ttsState: StateFlow<TtsState> = ttsManager.ttsState

    /**
     * Freezes the camera on the given [bitmap] and launches OCR processing.
     * The bitmap dimensions ([imageWidth], [imageHeight]) are stored for
     * coordinate mapping in the overlay view.
     */
    fun freezeFrame(bitmap: Bitmap, imageWidth: Int, imageHeight: Int) {
        _cameraState.value = CameraState.Processing
        viewModelScope.launch {
            val regions = OcrProcessor.process(bitmap)
            _cameraState.value = CameraState.Frozen(bitmap, regions, imageWidth, imageHeight)
        }
    }

    /**
     * Toggles the selection state of the region with the given [regionId].
     * Only operates when in [CameraState.Frozen] state.
     */
    fun toggleSelection(regionId: Int) {
        val current = _cameraState.value
        if (current is CameraState.Frozen) {
            val updated = current.regions.map {
                if (it.id == regionId) it.copy(isSelected = !it.isSelected) else it
            }
            _cameraState.value = current.copy(regions = updated)
        }
    }

    /**
     * Concatenates selected text regions in reading order and speaks them.
     * @return true if text was spoken, false if nothing was selected
     */
    fun readSelected(): Boolean {
        val current = _cameraState.value
        if (current is CameraState.Frozen) {
            val text = current.regions
                .filter { it.isSelected }
                .sortedWith(compareBy({ it.boundingBox.top }, { it.boundingBox.left }))
                .joinToString(" ") { it.text.replace("\n", " ") }
            if (text.isNotBlank()) {
                ttsManager.speak(text)
                return true
            }
        }
        return false
    }

    /**
     * Stops TTS and returns to live preview.
     * The previous [CameraState.Frozen] bitmap becomes GC-eligible since
     * no other component holds a reference to it.
     */
    fun clear() {
        ttsManager.stop()
        _cameraState.value = CameraState.LivePreview
    }

    override fun onCleared() {
        ttsManager.shutdown()
        super.onCleared()
    }
}
