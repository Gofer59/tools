package com.example.bookreader

import android.os.Bundle
import androidx.appcompat.app.AppCompatActivity
import androidx.lifecycle.lifecycleScope
import com.example.bookreader.data.setSpeechRate
import com.example.bookreader.data.speechRateFlow
import com.example.bookreader.databinding.ActivitySettingsBinding
import com.google.android.material.slider.Slider
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.launch

class SettingsActivity : AppCompatActivity() {

    private lateinit var binding: ActivitySettingsBinding

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        binding = ActivitySettingsBinding.inflate(layoutInflater)
        setContentView(binding.root)

        binding.toolbar.setNavigationOnClickListener { finish() }

        lifecycleScope.launch {
            val initial = applicationContext.speechRateFlow().first()
            binding.speechRateSlider.value = initial
            binding.speechRateValue.text = getString(R.string.settings_speech_rate_value, initial)
        }

        binding.speechRateSlider.addOnChangeListener(Slider.OnChangeListener { _, value, _ ->
            binding.speechRateValue.text = getString(R.string.settings_speech_rate_value, value)
        })

        binding.speechRateSlider.addOnSliderTouchListener(object : Slider.OnSliderTouchListener {
            override fun onStartTrackingTouch(slider: Slider) = Unit
            override fun onStopTrackingTouch(slider: Slider) {
                val value = slider.value
                lifecycleScope.launch {
                    applicationContext.setSpeechRate(value)
                }
            }
        })
    }
}
