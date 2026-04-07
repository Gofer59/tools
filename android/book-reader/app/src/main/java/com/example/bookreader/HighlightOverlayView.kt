package com.example.bookreader

import android.content.Context
import android.graphics.Canvas
import android.graphics.Color
import android.graphics.Paint
import android.graphics.RectF
import android.util.AttributeSet
import android.view.MotionEvent
import android.view.View
import com.example.bookreader.model.TextRegion

/**
 * Custom overlay view that draws bounding-box highlights over detected text regions
 * and handles tap-to-select interaction.
 *
 * Coordinate transformation (FILL_CENTER / centerCrop matching):
 * PreviewView with FILL_CENTER and ImageView with centerCrop both scale the image
 * to fill the view, cropping any overflow. The transform is:
 *   scale = max(viewW / imgW, viewH / imgH)
 *   offsetX = (viewW - imgW * scale) / 2   (negative when image is wider than view)
 *   offsetY = (viewH - imgH * scale) / 2   (negative when image is taller than view)
 *
 * This ensures bounding boxes from ML Kit (in image coordinates) align perfectly
 * with the displayed frozen frame. Portrait orientation lock simplifies this to
 * a single scale factor since rotation is handled before OCR.
 */
class HighlightOverlayView @JvmOverloads constructor(
    context: Context,
    attrs: AttributeSet? = null,
    defStyleAttr: Int = 0
) : View(context, attrs, defStyleAttr) {

    private var regions: List<TextRegion> = emptyList()
    private var imageWidth: Int = 1
    private var imageHeight: Int = 1
    private var onRegionTapped: ((Int) -> Unit)? = null

    private val detectedPaint = Paint().apply {
        color = Color.argb(100, 66, 133, 244) // semi-transparent blue
        style = Paint.Style.FILL
    }

    private val detectedStrokePaint = Paint().apply {
        color = Color.argb(200, 66, 133, 244) // blue stroke
        style = Paint.Style.STROKE
        strokeWidth = 3f
    }

    private val selectedPaint = Paint().apply {
        color = Color.argb(100, 255, 140, 0) // semi-transparent orange
        style = Paint.Style.FILL
    }

    private val selectedStrokePaint = Paint().apply {
        color = Color.argb(220, 255, 140, 0) // orange stroke
        style = Paint.Style.STROKE
        strokeWidth = 4f
    }

    /**
     * Updates the regions to draw and triggers a redraw.
     * Does not hold a reference to the source bitmap — only region metadata.
     */
    fun setRegions(regions: List<TextRegion>, imgWidth: Int, imgHeight: Int) {
        this.regions = regions
        this.imageWidth = imgWidth
        this.imageHeight = imgHeight
        invalidate()
    }

    /**
     * Clears all overlay regions. Called when returning to live preview.
     */
    fun clearRegions() {
        this.regions = emptyList()
        invalidate()
    }

    fun setOnRegionTappedListener(listener: (Int) -> Unit) {
        onRegionTapped = listener
    }

    override fun onDraw(canvas: Canvas) {
        super.onDraw(canvas)
        if (regions.isEmpty()) return

        val transform = computeTransform()

        for (region in regions) {
            val mapped = mapRect(region.boundingBox, transform)
            if (region.isSelected) {
                canvas.drawRect(mapped, selectedPaint)
                canvas.drawRect(mapped, selectedStrokePaint)
            } else {
                canvas.drawRect(mapped, detectedPaint)
                canvas.drawRect(mapped, detectedStrokePaint)
            }
        }
    }

    override fun onTouchEvent(event: MotionEvent): Boolean {
        if (regions.isEmpty()) return false
        if (event.action != MotionEvent.ACTION_UP) return true

        val transform = computeTransform()
        val touchX = event.x
        val touchY = event.y

        // Iterate in reverse so top-most (last-drawn) regions get priority
        for (region in regions.reversed()) {
            val mapped = mapRect(region.boundingBox, transform)
            if (mapped.contains(touchX, touchY)) {
                onRegionTapped?.invoke(region.id)
                performClick()
                return true
            }
        }
        return false
    }

    override fun performClick(): Boolean {
        return super.performClick()
    }

    // --- Coordinate transformation ---

    private data class Transform(
        val scaleX: Float,
        val scaleY: Float,
        val offsetX: Float,
        val offsetY: Float
    )

    /**
     * Computes the transform from image coordinates to view coordinates,
     * matching FILL_CENTER / centerCrop scaling behavior.
     */
    private fun computeTransform(): Transform {
        val viewW = width.toFloat()
        val viewH = height.toFloat()
        val imgW = imageWidth.toFloat()
        val imgH = imageHeight.toFloat()

        // FILL_CENTER: scale uniformly to fill the entire view, cropping overflow
        val scale = maxOf(viewW / imgW, viewH / imgH)

        // Center offset: the scaled image may extend beyond the view in one dimension
        val offsetX = (viewW - imgW * scale) / 2f
        val offsetY = (viewH - imgH * scale) / 2f

        return Transform(scale, scale, offsetX, offsetY)
    }

    private fun mapRect(rect: android.graphics.Rect, transform: Transform): RectF {
        return RectF(
            rect.left * transform.scaleX + transform.offsetX,
            rect.top * transform.scaleY + transform.offsetY,
            rect.right * transform.scaleX + transform.offsetX,
            rect.bottom * transform.scaleY + transform.offsetY
        )
    }
}
