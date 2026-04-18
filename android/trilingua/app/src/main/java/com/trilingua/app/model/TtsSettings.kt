package com.trilingua.app.model

/**
 * User-configurable TTS parameters persisted via DataStore.
 *
 * [speed]: playback speed multiplier 0.5–2.0 (default 1.0). Inverted to Piper's lengthScale
 *          (lengthScale = 1.0 / speed) before synthesis.
 * [noiseScale]: Piper voice variability 0.0–1.0 (default 0.667 per Piper recommendation).
 * [pitch]: AudioTrack playback pitch multiplier 0.5–2.0 (default 1.0). Applied via
 *          PlaybackParams.setPitch — independent of Piper lengthScale.
 * [voicePerLang]: selected voice directory per language. Falls back to the hardcoded default if absent.
 */
data class TtsSettings(
    val speed: Float = 1.0f,
    val noiseScale: Float = 0.667f,
    val pitch: Float = 1.0f,
    val voicePerLang: Map<Language, String> = emptyMap()
) {
    companion object {
        val DEFAULT = TtsSettings()

        /** Default voice directory names matching the bundled assets. */
        val DEFAULT_VOICES: Map<Language, String> = mapOf(
            Language.EN to "en_US-lessac-medium",
            Language.FR to "fr_FR-siwis-medium",
            Language.HU to "hu_HU-anna-medium"
        )
    }
}

/** Describes a single bundled or downloadable voice option. */
data class VoiceOption(val id: String, val displayName: String, val bundled: Boolean)

/** Registry of available voices per language. */
object VoiceRegistry {
    val en = listOf(VoiceOption("en_US-lessac-medium", "Lessac (US)", true))
    val fr = listOf(VoiceOption("fr_FR-siwis-medium", "Siwis (FR)", true))
    val hu = listOf(VoiceOption("hu_HU-anna-medium", "Anna (HU)", true))

    fun forLang(lang: Language): List<VoiceOption> = when (lang) {
        Language.EN -> en
        Language.FR -> fr
        Language.HU -> hu
    }
}
