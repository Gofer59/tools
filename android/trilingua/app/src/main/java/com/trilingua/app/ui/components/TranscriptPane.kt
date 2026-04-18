package com.trilingua.app.ui.components

import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.*
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import com.trilingua.app.R
import com.trilingua.app.model.Language

@Composable
fun TranscriptPane(
    sourceText: String,
    targetText: String,
    sourceLang: Language,
    targetLang: Language,
    modifier: Modifier = Modifier
) {
    Column(modifier = modifier, verticalArrangement = Arrangement.spacedBy(8.dp)) {
        TranscriptCard(
            label = sourceLang.displayName,
            text = sourceText,
            placeholder = stringResource(R.string.source_hint),
            modifier = Modifier.weight(1f)
        )
        TranscriptCard(
            label = targetLang.displayName,
            text = targetText,
            placeholder = stringResource(R.string.target_hint),
            modifier = Modifier.weight(1f)
        )
    }
}

@Composable
private fun TranscriptCard(
    label: String,
    text: String,
    placeholder: String,
    modifier: Modifier = Modifier
) {
    Card(
        modifier = modifier.fillMaxWidth(),
        colors = CardDefaults.cardColors(
            containerColor = MaterialTheme.colorScheme.surfaceVariant
        )
    ) {
        Column(
            modifier = Modifier
                .padding(12.dp)
                .fillMaxSize()
        ) {
            Text(
                text = label,
                style = MaterialTheme.typography.labelLarge,
                color = MaterialTheme.colorScheme.primary
            )
            Spacer(Modifier.height(4.dp))
            Box(
                modifier = Modifier
                    .weight(1f)
                    .verticalScroll(rememberScrollState())
            ) {
                if (text.isBlank()) {
                    Text(
                        text = placeholder,
                        style = MaterialTheme.typography.bodyLarge,
                        color = MaterialTheme.colorScheme.onSurfaceVariant.copy(alpha = 0.5f)
                    )
                } else {
                    Text(
                        text = text,
                        style = MaterialTheme.typography.bodyLarge,
                        color = MaterialTheme.colorScheme.onSurfaceVariant
                    )
                }
            }
        }
    }
}
