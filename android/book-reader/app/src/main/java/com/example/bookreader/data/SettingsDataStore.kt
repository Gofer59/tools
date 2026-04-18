package com.example.bookreader.data

import android.content.Context
import androidx.datastore.core.DataStore
import androidx.datastore.preferences.core.Preferences
import androidx.datastore.preferences.core.edit
import androidx.datastore.preferences.core.floatPreferencesKey
import androidx.datastore.preferences.preferencesDataStore
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.map

val Context.prefsDataStore: DataStore<Preferences> by preferencesDataStore(name = "settings")

object SettingsKeys {
    val SPEECH_RATE = floatPreferencesKey("speech_rate")
}

const val SPEECH_RATE_DEFAULT = 1.0f
const val SPEECH_RATE_MIN = 0.5f
const val SPEECH_RATE_MAX = 2.0f
const val SPEECH_RATE_STEP = 0.1f

fun Context.speechRateFlow(): Flow<Float> =
    prefsDataStore.data.map { prefs ->
        (prefs[SettingsKeys.SPEECH_RATE] ?: SPEECH_RATE_DEFAULT)
            .coerceIn(SPEECH_RATE_MIN, SPEECH_RATE_MAX)
    }

suspend fun Context.setSpeechRate(rate: Float) {
    val clamped = rate.coerceIn(SPEECH_RATE_MIN, SPEECH_RATE_MAX)
    prefsDataStore.edit { it[SettingsKeys.SPEECH_RATE] = clamped }
}
