package com.example.bookreader.model

import android.graphics.Rect

/**
 * A detected text region from OCR, with its bounding box in image coordinates.
 * [id] is assigned during OCR processing for identification during selection toggling.
 */
data class TextRegion(
    val id: Int,
    val text: String,
    val boundingBox: Rect,
    val isSelected: Boolean = false
)
