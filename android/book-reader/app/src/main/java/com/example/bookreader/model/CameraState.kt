package com.example.bookreader.model

import android.graphics.Bitmap

/**
 * Represents the camera pipeline state.
 * - [LivePreview]: camera is streaming live frames
 * - [Processing]: frame is frozen, OCR is running
 * - [Frozen]: OCR complete, bitmap and detected regions available
 *
 * Bitmap lifecycle: only [Frozen] holds a bitmap reference.
 * When state transitions to [LivePreview], the bitmap becomes GC-eligible.
 */
sealed interface CameraState {
    data object LivePreview : CameraState
    data object Processing : CameraState
    data class Frozen(
        val bitmap: Bitmap,
        val regions: List<TextRegion> = emptyList(),
        val imageWidth: Int,
        val imageHeight: Int
    ) : CameraState
}
