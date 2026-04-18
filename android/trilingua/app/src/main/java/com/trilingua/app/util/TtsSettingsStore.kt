package com.trilingua.app.util

import android.content.Context
import androidx.datastore.core.DataStore
import androidx.datastore.preferences.core.Preferences
import androidx.datastore.preferences.core.edit
import androidx.datastore.preferences.core.floatPreferencesKey
import androidx.datastore.preferences.core.stringPreferencesKey
import androidx.datastore.preferences.preferencesDataStore
import com.trilingua.app.model.Language
import com.trilingua.app.model.TtsSettings
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.map

/**
 * Persists TTS settings (speed, noise scale, voice per language) using Jetpack DataStore Preferences.
 * Thread-safe: DataStore serialises all writes. Reads are a cold Flow — collect on any dispatcher.
 */
class TtsSettingsStore(private val context: Context) {

    private val Context.dataStore: DataStore<Preferences> by preferencesDataStore(name = "tts_settings")

    private object Keys {
        val SPEED       = floatPreferencesKey("speed")
        val NOISE_SCALE = floatPreferencesKey("noise_scale")
        val PITCH       = floatPreferencesKey("tts_pitch")
        // One key per language for the chosen voice directory name
        val voiceKey: (Language) -> Preferences.Key<String> = { lang ->
            stringPreferencesKey("voice_${lang.tag}")
        }
    }

    /** Emits the current settings and re-emits on each change. */
    val settings: Flow<TtsSettings> = context.dataStore.data.map { prefs ->
        val speed      = prefs[Keys.SPEED]       ?: TtsSettings.DEFAULT.speed
        val noiseScale = prefs[Keys.NOISE_SCALE] ?: TtsSettings.DEFAULT.noiseScale
        val pitch      = prefs[Keys.PITCH]       ?: TtsSettings.DEFAULT.pitch
        val voicePerLang = Language.values().mapNotNull { lang ->
            prefs[Keys.voiceKey(lang)]?.let { voice -> lang to voice }
        }.toMap()
        TtsSettings(speed = speed, noiseScale = noiseScale, pitch = pitch, voicePerLang = voicePerLang)
    }

    /** Persist updated TtsSettings. Suspends until the write is committed. */
    suspend fun update(settings: TtsSettings) {
        context.dataStore.edit { prefs ->
            prefs[Keys.SPEED]       = settings.speed
            prefs[Keys.NOISE_SCALE] = settings.noiseScale
            prefs[Keys.PITCH]       = settings.pitch
            for ((lang, voice) in settings.voicePerLang) {
                prefs[Keys.voiceKey(lang)] = voice
            }
        }
    }
}
