package com.trilingua.app

import android.content.pm.PackageManager
import android.os.Bundle
import com.trilingua.app.R
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.result.contract.ActivityResultContracts
import androidx.activity.viewModels
import androidx.compose.runtime.CompositionLocalProvider
import androidx.core.content.ContextCompat
import com.trilingua.app.model.TrilinguaError
import com.trilingua.app.ui.LocalActivity
import com.trilingua.app.ui.MainViewModel
import com.trilingua.app.ui.MainViewModelFactory
import com.trilingua.app.ui.TrilinguaApp

/**
 * Single-activity Compose host for the Trilingua app.
 *
 * Declared portrait-only (screenOrientation="portrait") with largeHeap="true" in the manifest.
 * largeHeap is required because the app embeds ~881 MB of STT/MT/TTS model assets that are
 * extracted to internal storage on first launch via JNI; the additional heap headroom avoids
 * OOM during asset extraction and model loading.
 *
 * Permission strategy: [ensureMicPermission] does NOT auto-retry [pressMic] after the user
 * grants the permission. The permission dialog resolves the Compose gesture (tryAwaitRelease
 * returns), so [onPressUp] fires before the grant callback arrives. Retaining the callback
 * caused a race condition where recording started with a null recordJob. Instead, the user
 * must press the mic button a second time after granting; the banner clears on grant to signal
 * readiness.
 */
class MainActivity : ComponentActivity() {
    private val vm: MainViewModel by viewModels {
        MainViewModelFactory((application as TrilinguaApplication).container)
    }

    /**
     * Guards the "Microphone ready" snackbar so it shows only on the very first
     * denied→granted transition. Persisted across recreation (rotation) via
     * [onSaveInstanceState] / [onCreate] so a config change does not re-trigger it.
     */
    private var firstGrantSeen: Boolean = false

    // Do NOT auto-invoke onGranted after dialog dismissal: the permission dialog
    // resolves the press gesture (tryAwaitRelease returns) so onPressUp already fired.
    // Starting recording on grant would race with a late onMicReleased(recordJob=null).
    // Instead: first press triggers the dialog, user grants, banner clears, user presses again.
    private val permLauncher = registerForActivityResult(
        ActivityResultContracts.RequestPermission()
    ) { granted ->
        if (!granted) {
            vm.setError(TrilinguaError.MicDenied)
        } else {
            vm.dismissError()
            if (!firstGrantSeen) {
                firstGrantSeen = true
                vm.showTransientMessage(getString(R.string.mic_ready_hint))
            }
        }
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        firstGrantSeen = savedInstanceState?.getBoolean(KEY_FIRST_GRANT_SEEN) ?: false
        setContent {
            CompositionLocalProvider(LocalActivity provides this@MainActivity) {
                TrilinguaApp(vm)
            }
        }
    }

    override fun onSaveInstanceState(outState: Bundle) {
        super.onSaveInstanceState(outState)
        outState.putBoolean(KEY_FIRST_GRANT_SEEN, firstGrantSeen)
    }

    companion object {
        private const val KEY_FIRST_GRANT_SEEN = "first_grant_seen"
    }

    fun ensureMicPermission(onGranted: () -> Unit) {
        if (ContextCompat.checkSelfPermission(
                this,
                android.Manifest.permission.RECORD_AUDIO
            ) == PackageManager.PERMISSION_GRANTED
        ) {
            onGranted()
        } else {
            // Do not retain onGranted — see permLauncher comment above.
            permLauncher.launch(android.Manifest.permission.RECORD_AUDIO)
        }
    }
}
