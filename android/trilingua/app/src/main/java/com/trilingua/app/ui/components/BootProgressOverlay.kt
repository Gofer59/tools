package com.trilingua.app.ui.components

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.*
import androidx.compose.material3.*
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import com.trilingua.app.R
import com.trilingua.app.model.BootState
import kotlin.math.roundToInt

@Composable
fun BootProgressOverlay(
    bootState: BootState,
    onRetry: (() -> Unit)? = null,
    modifier: Modifier = Modifier
) {
    Box(
        modifier = modifier
            .fillMaxSize()
            .background(Color.Black.copy(alpha = 0.75f)),
        contentAlignment = Alignment.Center
    ) {
        Column(
            horizontalAlignment = Alignment.CenterHorizontally,
            verticalArrangement = Arrangement.spacedBy(16.dp)
        ) {
            when (bootState) {
                is BootState.Extracting -> {
                    CircularProgressIndicator(
                        progress = { bootState.pct },
                        modifier = Modifier.size(64.dp),
                        strokeWidth = 6.dp
                    )
                    Text(
                        text = stringResource(R.string.boot_extracting_pct, (bootState.pct * 100).roundToInt()),
                        style = MaterialTheme.typography.bodyLarge,
                        color = Color.White
                    )
                }
                is BootState.Failed -> {
                    Text(
                        text = stringResource(R.string.boot_failed),
                        style = MaterialTheme.typography.bodyLarge,
                        color = MaterialTheme.colorScheme.error
                    )
                    if (onRetry != null) {
                        Spacer(Modifier.height(8.dp))
                        Button(onClick = onRetry) {
                            Text(stringResource(R.string.boot_retry))
                        }
                    }
                }
                else -> {
                    CircularProgressIndicator(modifier = Modifier.size(64.dp))
                    Text(
                        text = stringResource(R.string.boot_initializing),
                        style = MaterialTheme.typography.bodyLarge,
                        color = Color.White
                    )
                }
            }
        }
    }
}
