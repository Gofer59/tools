package com.trilingua.app.ui

import androidx.compose.foundation.layout.*
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Settings
import androidx.compose.material.icons.filled.Stop
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.trilingua.app.R
import com.trilingua.app.model.BootState
import com.trilingua.app.ui.components.*

@Composable
fun MainScreen(vm: MainViewModel) {
    val state    by vm.uiState.collectAsStateWithLifecycle()
    val settings by vm.ttsSettings.collectAsStateWithLifecycle()
    val activity = LocalActivity.current

    val snackbarHostState = remember { SnackbarHostState() }
    val transientMessage = state.transientMessage
    LaunchedEffect(transientMessage) {
        if (transientMessage != null) {
            snackbarHostState.showSnackbar(transientMessage, duration = SnackbarDuration.Short)
            vm.clearTransientMessage()
        }
    }

    Scaffold(
        snackbarHost = { SnackbarHost(snackbarHostState) },
        containerColor = MaterialTheme.colorScheme.background
    ) { innerPadding ->
        Box(
            Modifier
                .fillMaxSize()
                .padding(innerPadding)
        ) {
            // Top-right settings icon
            IconButton(
                onClick = vm::openSettings,
                modifier = Modifier
                    .align(Alignment.TopEnd)
                    .padding(8.dp)
            ) {
                Icon(
                    imageVector = Icons.Default.Settings,
                    contentDescription = stringResource(R.string.settings)
                )
            }

            Column(
                Modifier
                    .fillMaxSize()
                    .padding(16.dp)
            ) {
                LanguageRow(
                    source = state.source,
                    target = state.target,
                    onSwap = vm::swap,
                    onSourceChange = vm::setSource,
                    onTargetChange = vm::setTarget,
                    enabled = state.isInteractive
                )
                Spacer(Modifier.height(16.dp))
                TranscriptPane(
                    sourceText = state.sourceText,
                    targetText = state.targetText,
                    sourceLang = state.source,
                    targetLang = state.target,
                    modifier = Modifier.weight(1f)
                )
                Spacer(Modifier.height(16.dp))
                MicButton(
                    pipelineState = state.pipelineState,
                    enabled = state.isMicEnabled,
                    onPressDown = {
                        if (activity != null) {
                            activity.ensureMicPermission {
                                vm.pressMic()
                            }
                        } else {
                            vm.pressMic()
                        }
                    },
                    onPressUp = vm::releaseMic
                )
                if (state.canCancel) {
                    Spacer(Modifier.height(12.dp))
                    FilledTonalButton(
                        onClick = vm::cancel,
                        modifier = Modifier.align(Alignment.CenterHorizontally)
                    ) {
                        Icon(
                            imageVector = Icons.Filled.Stop,
                            // Icon decorative — button text label "Cancel" provides context
                            contentDescription = null,
                            modifier = Modifier.size(18.dp)
                        )
                        Spacer(Modifier.width(6.dp))
                        Text(stringResource(R.string.cancel))
                    }
                }
            }

            state.error?.let {
                ErrorBanner(
                    error = it,
                    onDismiss = vm::dismissError,
                    modifier = Modifier
                        .align(Alignment.TopCenter)
                        .statusBarsPadding()
                        .padding(horizontal = 12.dp, vertical = 8.dp)
                )
            }

            if (state.bootState !is BootState.Ready) {
                BootProgressOverlay(
                    bootState = state.bootState,
                    onRetry = if (state.bootState is BootState.Failed) vm::retryExtraction else null
                )
            }
        }
    }

    // TTS Settings ModalBottomSheet
    if (state.showSettings) {
        SettingsBottomSheet(
            currentSettings = settings,
            onSave = { updated ->
                vm.saveTtsSettings(updated)
            },
            onDismiss = vm::closeSettings
        )
    }
}
