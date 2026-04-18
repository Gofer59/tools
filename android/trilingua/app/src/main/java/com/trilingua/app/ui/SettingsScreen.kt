package com.trilingua.app.ui

import androidx.compose.foundation.layout.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import com.trilingua.app.R
import com.trilingua.app.model.Language
import com.trilingua.app.model.TtsSettings
import com.trilingua.app.model.VoiceRegistry
import kotlinx.coroutines.launch

/**
 * TTS Settings displayed as a ModalBottomSheet.
 * Persists changes via [TtsSettingsStore] injected through the ViewModel's AppContainer.
 * No INTERNET permission required — voice download CTA is UI-only disabled.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun SettingsBottomSheet(
    currentSettings: TtsSettings,
    onSave: suspend (TtsSettings) -> Unit,
    onDismiss: () -> Unit
) {
    val sheetState = rememberModalBottomSheetState(skipPartiallyExpanded = true)
    val scope = rememberCoroutineScope()

    // Local mutable copies for the sliders
    var speed      by remember(currentSettings.speed)      { mutableFloatStateOf(currentSettings.speed) }
    var noiseScale by remember(currentSettings.noiseScale) { mutableFloatStateOf(currentSettings.noiseScale) }
    var pitch      by remember(currentSettings.pitch)      { mutableFloatStateOf(currentSettings.pitch) }

    // Local mutable copy for per-language voice selection
    var voicePerLang by remember(currentSettings.voicePerLang) {
        mutableStateOf(currentSettings.voicePerLang.toMutableMap() as MutableMap<Language, String>)
    }

    ModalBottomSheet(
        onDismissRequest = onDismiss,
        sheetState = sheetState
    ) {
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .padding(horizontal = 24.dp)
                .padding(bottom = 32.dp),
            verticalArrangement = Arrangement.spacedBy(16.dp)
        ) {
            Text(
                text = stringResource(R.string.settings_title),
                style = MaterialTheme.typography.titleLarge
            )

            // --- Speed slider ---
            Text(
                text = stringResource(R.string.settings_speed_label, speed),
                style = MaterialTheme.typography.labelLarge
            )
            Slider(
                value = speed,
                onValueChange = { speed = it },
                valueRange = 0.5f..2.0f,
                steps = 29, // 0.05 increments: (2.0-0.5)/0.05 - 1 = 29 intermediate steps
                modifier = Modifier.fillMaxWidth()
            )
            Text(
                text = stringResource(R.string.settings_speed_description),
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant
            )

            HorizontalDivider()

            // --- Noise scale (voice variability) slider ---
            Text(
                text = stringResource(R.string.settings_noise_scale_label, noiseScale),
                style = MaterialTheme.typography.labelLarge
            )
            Slider(
                value = noiseScale,
                onValueChange = { noiseScale = it },
                valueRange = 0.0f..1.0f,
                steps = 19, // 0.05 increments
                modifier = Modifier.fillMaxWidth()
            )
            Text(
                text = stringResource(R.string.settings_noise_scale_description),
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant
            )

            HorizontalDivider()

            // --- Pitch slider ---
            Text(
                text = stringResource(R.string.settings_pitch_label, pitch),
                style = MaterialTheme.typography.labelLarge
            )
            Slider(
                value = pitch,
                onValueChange = { pitch = it },
                valueRange = 0.5f..2.0f,
                steps = 29, // 0.05 increments
                modifier = Modifier.fillMaxWidth()
            )
            Text(
                text = stringResource(R.string.settings_pitch_description),
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant
            )

            HorizontalDivider()

            // --- Voice selection per language (dropdown picker) ---
            Text(
                text = stringResource(R.string.settings_voice_label),
                style = MaterialTheme.typography.labelLarge
            )
            Language.values().forEach { lang ->
                val options = VoiceRegistry.forLang(lang)
                val selectedId = voicePerLang[lang] ?: (TtsSettings.DEFAULT_VOICES[lang] ?: options.first().id)
                var expanded by remember { mutableStateOf(false) }

                Column(verticalArrangement = Arrangement.spacedBy(4.dp)) {
                    Text(
                        text = lang.displayName,
                        style = MaterialTheme.typography.bodyMedium
                    )
                    ExposedDropdownMenuBox(
                        expanded = expanded,
                        onExpandedChange = { expanded = it }
                    ) {
                        OutlinedTextField(
                            value = options.find { it.id == selectedId }?.displayName ?: selectedId,
                            onValueChange = {},
                            readOnly = true,
                            trailingIcon = { ExposedDropdownMenuDefaults.TrailingIcon(expanded = expanded) },
                            modifier = Modifier
                                .fillMaxWidth()
                                .menuAnchor(),
                            colors = ExposedDropdownMenuDefaults.outlinedTextFieldColors()
                        )
                        ExposedDropdownMenu(
                            expanded = expanded,
                            onDismissRequest = { expanded = false }
                        ) {
                            options.forEach { option ->
                                DropdownMenuItem(
                                    text = { Text(option.displayName) },
                                    onClick = {
                                        voicePerLang = (voicePerLang + (lang to option.id)).toMutableMap()
                                        expanded = false
                                    },
                                    enabled = option.bundled
                                )
                            }
                            // Disabled CTA for future downloadable voices
                            DropdownMenuItem(
                                text = {
                                    Text(
                                        text = stringResource(R.string.settings_voice_more),
                                        color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.38f)
                                    )
                                },
                                onClick = {},
                                enabled = false
                            )
                        }
                    }
                }
            }

            HorizontalDivider()

            // --- Save / Close buttons ---
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.End
            ) {
                TextButton(onClick = onDismiss) {
                    Text(stringResource(R.string.settings_close))
                }
                Spacer(Modifier.width(8.dp))
                Button(onClick = {
                    val updated = currentSettings.copy(
                        speed = speed,
                        noiseScale = noiseScale,
                        pitch = pitch,
                        voicePerLang = voicePerLang.toMap()
                    )
                    scope.launch {
                        onSave(updated)
                        sheetState.hide()
                        onDismiss()
                    }
                }) {
                    Text(stringResource(R.string.settings_save))
                }
            }
        }
    }
}
