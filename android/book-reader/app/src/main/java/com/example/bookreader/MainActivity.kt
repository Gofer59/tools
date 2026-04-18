package com.example.bookreader

import android.Manifest
import android.content.Intent
import android.content.pm.PackageManager
import android.graphics.Bitmap
import android.graphics.Matrix
import android.os.Bundle
import android.speech.tts.TextToSpeech
import android.view.View
import android.widget.Toast
import androidx.activity.result.contract.ActivityResultContracts
import androidx.activity.viewModels
import androidx.appcompat.app.AppCompatActivity
import androidx.camera.core.CameraSelector
import androidx.camera.core.ImageCapture
import androidx.camera.core.ImageCaptureException
import androidx.camera.core.ImageProxy
import androidx.camera.lifecycle.ProcessCameraProvider
import androidx.core.content.ContextCompat
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.lifecycleScope
import androidx.lifecycle.repeatOnLifecycle
import com.example.bookreader.databinding.ActivityMainBinding
import com.example.bookreader.model.CameraState
import com.example.bookreader.model.TtsState
import com.google.android.material.snackbar.Snackbar
import kotlinx.coroutines.launch
import java.util.Locale

class MainActivity : AppCompatActivity() {

    private lateinit var binding: ActivityMainBinding
    private val viewModel: CameraViewModel by viewModels()
    private var imageCapture: ImageCapture? = null

    private val cameraPermissionLauncher = registerForActivityResult(
        ActivityResultContracts.RequestPermission()
    ) { granted ->
        if (granted) {
            startCamera()
        } else {
            Toast.makeText(this, R.string.permission_camera_denied, Toast.LENGTH_LONG).show()
        }
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        binding = ActivityMainBinding.inflate(layoutInflater)
        setContentView(binding.root)

        requestCameraPermission()
        setupButtons()
        setupOverlay()
        observeState()
    }

    private fun requestCameraPermission() {
        if (ContextCompat.checkSelfPermission(this, Manifest.permission.CAMERA)
            == PackageManager.PERMISSION_GRANTED
        ) {
            startCamera()
        } else {
            cameraPermissionLauncher.launch(Manifest.permission.CAMERA)
        }
    }

    private fun startCamera() {
        val cameraProviderFuture = ProcessCameraProvider.getInstance(this)
        cameraProviderFuture.addListener({
            val cameraProvider = cameraProviderFuture.get()

            val preview = androidx.camera.core.Preview.Builder()
                .build()
                .also { it.setSurfaceProvider(binding.previewView.surfaceProvider) }

            imageCapture = ImageCapture.Builder()
                .setCaptureMode(ImageCapture.CAPTURE_MODE_MINIMIZE_LATENCY)
                .build()

            cameraProvider.unbindAll()
            cameraProvider.bindToLifecycle(
                this,
                CameraSelector.DEFAULT_BACK_CAMERA,
                preview,
                imageCapture
            )
        }, ContextCompat.getMainExecutor(this))
    }

    private fun setupButtons() {
        binding.btnFreeze.setOnClickListener {
            captureFrame()
        }

        binding.btnReadSelected.setOnClickListener {
            when (viewModel.ttsState.value) {
                is TtsState.Ready, is TtsState.Speaking -> {
                    if (!viewModel.readSelected()) {
                        Snackbar.make(binding.rootLayout, R.string.no_selection, Snackbar.LENGTH_SHORT).show()
                    }
                }
                is TtsState.MissingVoice -> showMissingVoiceSnackbar()
                is TtsState.Initializing -> {
                    Snackbar.make(binding.rootLayout, R.string.tts_not_ready, Snackbar.LENGTH_SHORT).show()
                }
                is TtsState.Error -> {
                    Snackbar.make(binding.rootLayout, R.string.tts_error, Snackbar.LENGTH_SHORT).show()
                }
            }
        }

        binding.btnClear.setOnClickListener {
            viewModel.clear()
        }

        // Language toggle: FR ↔ EN
        binding.btnLanguage.setOnClickListener {
            val current = viewModel.ttsManager.currentLocale.value
            if (current.language == "fr") {
                viewModel.ttsManager.setLocale(Locale("en", "US"))
                binding.btnLanguage.text = getString(R.string.lang_en)
            } else {
                viewModel.ttsManager.setLocale(Locale("fr", "FR"))
                binding.btnLanguage.text = getString(R.string.lang_fr)
            }
        }

        binding.btnSettings.setOnClickListener {
            startActivity(Intent(this, SettingsActivity::class.java))
        }
    }

    private fun setupOverlay() {
        binding.overlayView.setOnRegionTappedListener { regionId ->
            viewModel.toggleSelection(regionId)
        }
    }

    /**
     * Captures a single frame from ImageCapture.
     * Rotation handling: CameraX provides rotation metadata via imageInfo.rotationDegrees.
     * We apply it via Matrix.postRotate() so ML Kit receives an upright image.
     */
    private fun captureFrame() {
        val capture = imageCapture ?: return

        capture.takePicture(
            ContextCompat.getMainExecutor(this),
            object : ImageCapture.OnImageCapturedCallback() {
                override fun onCaptureSuccess(imageProxy: ImageProxy) {
                    val rotation = imageProxy.imageInfo.rotationDegrees
                    val bitmap = imageProxy.toBitmap()
                    imageProxy.close()

                    val rotatedBitmap = if (rotation != 0) {
                        val matrix = Matrix().apply { postRotate(rotation.toFloat()) }
                        val rotated = Bitmap.createBitmap(bitmap, 0, 0, bitmap.width, bitmap.height, matrix, true)
                        bitmap.recycle()
                        rotated
                    } else {
                        bitmap
                    }

                    viewModel.freezeFrame(rotatedBitmap, rotatedBitmap.width, rotatedBitmap.height)
                }

                override fun onError(exception: ImageCaptureException) {
                    Toast.makeText(
                        this@MainActivity,
                        getString(R.string.capture_error, exception.message ?: ""),
                        Toast.LENGTH_SHORT
                    ).show()
                }
            }
        )
    }

    private fun observeState() {
        lifecycleScope.launch {
            repeatOnLifecycle(Lifecycle.State.STARTED) {
                launch { observeCameraState() }
                launch { observeTtsState() }
            }
        }
    }

    private suspend fun observeCameraState() {
        viewModel.cameraState.collect { state ->
            when (state) {
                is CameraState.LivePreview -> {
                    binding.previewView.visibility = View.VISIBLE
                    binding.frozenImageView.visibility = View.GONE
                    binding.frozenImageView.setImageBitmap(null)
                    binding.overlayView.clearRegions()
                    binding.progressBar.visibility = View.GONE
                    binding.btnFreeze.visibility = View.VISIBLE
                    binding.btnReadSelected.visibility = View.GONE
                    binding.btnClear.visibility = View.GONE
                }
                is CameraState.Processing -> {
                    binding.previewView.visibility = View.GONE
                    binding.progressBar.visibility = View.VISIBLE
                    binding.btnFreeze.visibility = View.GONE
                }
                is CameraState.Frozen -> {
                    binding.previewView.visibility = View.GONE
                    binding.frozenImageView.setImageBitmap(state.bitmap)
                    binding.frozenImageView.visibility = View.VISIBLE
                    binding.overlayView.setRegions(state.regions, state.imageWidth, state.imageHeight)
                    binding.progressBar.visibility = View.GONE
                    binding.btnFreeze.visibility = View.GONE
                    binding.btnReadSelected.visibility = View.VISIBLE
                    binding.btnClear.visibility = View.VISIBLE

                    if (state.regions.isEmpty()) {
                        Snackbar.make(binding.rootLayout, R.string.no_text_detected, Snackbar.LENGTH_SHORT).show()
                    }
                }
            }
        }
    }

    private suspend fun observeTtsState() {
        viewModel.ttsState.collect { state ->
            if (state is TtsState.MissingVoice) {
                showMissingVoiceSnackbar()
            }
        }
    }

    private fun showMissingVoiceSnackbar() {
        val langName = if (viewModel.ttsManager.currentLocale.value.language == "fr") "française" else "anglaise"
        Snackbar.make(binding.rootLayout, getString(R.string.voice_missing, langName), Snackbar.LENGTH_LONG)
            .setAction(R.string.install_voice) {
                startActivity(Intent(TextToSpeech.Engine.ACTION_INSTALL_TTS_DATA))
            }
            .show()
    }
}
