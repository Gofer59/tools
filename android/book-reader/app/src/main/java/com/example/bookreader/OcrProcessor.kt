package com.example.bookreader

import android.graphics.Bitmap
import android.graphics.Rect
import com.example.bookreader.model.TextRegion
import com.google.mlkit.vision.common.InputImage
import com.google.mlkit.vision.text.TextRecognition
import com.google.mlkit.vision.text.latin.TextRecognizerOptions
import kotlinx.coroutines.tasks.await

/**
 * On-device OCR using ML Kit Text Recognition (bundled Latin model).
 *
 * Design decisions:
 * - Uses the bundled model (com.google.mlkit:text-recognition) not the thin
 *   Google Play Services variant, ensuring offline operation on all devices.
 * - The bitmap passed here must already be rotation-corrected (ImageProxy rotation
 *   applied via Matrix.postRotate before calling this).
 * - Results are sorted in reading order: top-to-bottom, then left-to-right.
 * - Operates at text block level for cleaner bounding boxes and more natural
 *   reading units compared to word-level elements.
 */
object OcrProcessor {

    private val recognizer = TextRecognition.getClient(TextRecognizerOptions.DEFAULT_OPTIONS)

    /**
     * Processes a rotation-corrected [bitmap] and returns detected text regions
     * sorted in reading order.
     *
     * @param bitmap The captured frame, already rotated to match display orientation.
     * @return List of [TextRegion] with bounding boxes in image coordinates.
     */
    suspend fun process(bitmap: Bitmap): List<TextRegion> {
        // Rotation is 0 because the bitmap was pre-rotated before reaching here
        val inputImage = InputImage.fromBitmap(bitmap, 0)
        val visionText = recognizer.process(inputImage).await()

        return visionText.textBlocks
            .filter { it.boundingBox != null }
            .sortedWith(compareBy({ it.boundingBox!!.top }, { it.boundingBox!!.left }))
            .mapIndexed { index, block ->
                TextRegion(
                    id = index,
                    text = block.text,
                    boundingBox = block.boundingBox ?: Rect(),
                    isSelected = false
                )
            }
    }
}
