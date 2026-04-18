package com.trilingua.app.ui.components

import androidx.compose.animation.core.*
import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Mic
import androidx.compose.material.icons.filled.MicOff
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.scale
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import com.trilingua.app.R
import com.trilingua.app.model.PipelineState

@Composable
fun MicButton(
    pipelineState: PipelineState,
    enabled: Boolean,
    onPressDown: () -> Unit,
    onPressUp: () -> Unit,
    modifier: Modifier = Modifier
) {
    val isRecording = pipelineState is PipelineState.Recording
    var isPressed by remember { mutableStateOf(false) }

    // Use rememberUpdatedState so the gesture handler always calls the latest lambdas,
    // even when enabled or the callbacks change between recompositions.
    val enabledState        by rememberUpdatedState(enabled)
    val currentOnPressDown by rememberUpdatedState(onPressDown)
    val currentOnPressUp   by rememberUpdatedState(onPressUp)

    // Scale animation on press
    val scale by animateFloatAsState(
        targetValue = if (isPressed) 1.08f else 1.0f,
        animationSpec = spring(stiffness = Spring.StiffnessMediumLow),
        label = "mic_scale"
    )

    // Pulsing ring while recording
    val pulseTransition = rememberInfiniteTransition(label = "pulse")
    val pulseScale by pulseTransition.animateFloat(
        initialValue = 1.0f,
        targetValue = 1.2f,
        animationSpec = infiniteRepeatable(
            animation = tween(durationMillis = 1000, easing = FastOutSlowInEasing),
            repeatMode = RepeatMode.Reverse
        ),
        label = "pulse_scale"
    )
    val pulseAlpha by pulseTransition.animateFloat(
        initialValue = 0.6f,
        targetValue = 0.0f,
        animationSpec = infiniteRepeatable(
            animation = tween(durationMillis = 1000),
            repeatMode = RepeatMode.Reverse
        ),
        label = "pulse_alpha"
    )

    Box(
        modifier = modifier.fillMaxWidth(),
        contentAlignment = Alignment.Center
    ) {
        // Outer pulse ring (only while recording)
        if (isRecording) {
            Surface(
                modifier = Modifier
                    .size(120.dp)
                    .scale(pulseScale),
                shape = CircleShape,
                color = MaterialTheme.colorScheme.primary.copy(alpha = pulseAlpha)
            ) {}
        }

        // Main button
        Surface(
            modifier = Modifier
                .size(120.dp)
                .scale(scale)
                .pointerInput(Unit) {
                    detectTapGestures(
                        onPress = {
                            if (enabledState) {
                                android.util.Log.d("Trilingua", "[${System.currentTimeMillis()}] MicButton: onPressDown")
                                isPressed = true
                                currentOnPressDown()
                                try {
                                    tryAwaitRelease()
                                } finally {
                                    isPressed = false
                                    android.util.Log.d("Trilingua", "[${System.currentTimeMillis()}] MicButton: onPressUp")
                                    currentOnPressUp()
                                }
                            }
                        }
                    )
                },
            shape = CircleShape,
            color = if (isRecording)
                MaterialTheme.colorScheme.error
            else
                MaterialTheme.colorScheme.primary,
            shadowElevation = if (isPressed) 2.dp else 8.dp
        ) {
            Box(contentAlignment = Alignment.Center) {
                Icon(
                    imageVector = if (enabled) Icons.Default.Mic else Icons.Default.MicOff,
                    contentDescription = if (enabled) stringResource(R.string.mic_hold_to_talk)
                                         else stringResource(R.string.mic_disabled),
                    tint = if (enabled)
                        MaterialTheme.colorScheme.onPrimary
                    else
                        MaterialTheme.colorScheme.onSurface.copy(alpha = 0.38f),
                    modifier = Modifier.size(48.dp)
                )
            }
        }
    }
}
