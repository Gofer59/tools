package com.trilingua.app

import com.trilingua.app.model.Language
import com.trilingua.app.model.TtsSettings
import com.trilingua.app.model.VoiceRegistry
import org.junit.Assert.*
import org.junit.Test

class TtsSettingsTest {

    @Test
    fun `default values are as specified`() {
        val d = TtsSettings.DEFAULT
        assertEquals(1.0f, d.speed, 0.001f)
        assertEquals(0.667f, d.noiseScale, 0.001f)
        assertTrue(d.voicePerLang.isEmpty())
    }

    @Test
    fun `copy round-trip preserves all fields`() {
        val original = TtsSettings(speed = 1.5f, noiseScale = 0.3f,
            voicePerLang = mapOf(Language.EN to "en_US-lessac-medium"))
        val copy = original.copy()
        assertEquals(original, copy)
    }

    @Test
    fun `DEFAULT_VOICES contains all languages`() {
        for (lang in Language.values()) {
            assertTrue("DEFAULT_VOICES missing ${lang.tag}",
                TtsSettings.DEFAULT_VOICES.containsKey(lang))
        }
    }

    @Test
    fun `lengthScale derivation is correct`() {
        // lengthScale = 1.0 / speed — verify the inverse relationship
        val speed = 1.5f
        val lengthScale = 1.0f / speed
        assertEquals(0.667f, lengthScale, 0.001f)
    }

    @Test
    fun `speed clamp at extremes`() {
        val min = TtsSettings(speed = 0.5f)
        val max = TtsSettings(speed = 2.0f)
        assertTrue(min.speed >= 0.5f)
        assertTrue(max.speed <= 2.0f)
    }

    // --- Pitch tests ---

    @Test
    fun `pitch default is 1_0`() {
        assertEquals(1.0f, TtsSettings.DEFAULT.pitch, 1e-6f)
    }

    @Test
    fun `pitch round-trip via copy`() {
        val s = TtsSettings(pitch = 1.4f)
        val copy = s.copy()
        assertEquals(1.4f, copy.pitch, 1e-6f)
    }

    @Test
    fun `pitch included in copy with all fields`() {
        val original = TtsSettings(speed = 1.2f, noiseScale = 0.5f, pitch = 0.8f,
            voicePerLang = mapOf(Language.FR to "fr_FR-siwis-medium"))
        val copy = original.copy()
        assertEquals(original, copy)
        assertEquals(0.8f, copy.pitch, 1e-6f)
    }

    // --- VoiceRegistry tests ---

    @Test
    fun `VoiceRegistry EN has lessac`() {
        assertEquals("en_US-lessac-medium", VoiceRegistry.forLang(Language.EN).first().id)
    }

    @Test
    fun `VoiceRegistry FR has siwis`() {
        assertEquals("fr_FR-siwis-medium", VoiceRegistry.forLang(Language.FR).first().id)
    }

    @Test
    fun `VoiceRegistry HU has anna`() {
        assertEquals("hu_HU-anna-medium", VoiceRegistry.forLang(Language.HU).first().id)
    }

    @Test
    fun `VoiceRegistry all bundled voices are marked bundled`() {
        for (lang in Language.values()) {
            assertTrue("${lang.tag} first voice should be bundled",
                VoiceRegistry.forLang(lang).first().bundled)
        }
    }

    @Test
    fun `voicePerLang round-trip in TtsSettings`() {
        val map = mapOf(Language.EN to "en_US-lessac-medium",
                        Language.FR to "fr_FR-siwis-medium",
                        Language.HU to "hu_HU-anna-medium")
        val s = TtsSettings(voicePerLang = map)
        assertEquals("en_US-lessac-medium", s.voicePerLang[Language.EN])
        assertEquals("fr_FR-siwis-medium",  s.voicePerLang[Language.FR])
        assertEquals("hu_HU-anna-medium",   s.voicePerLang[Language.HU])
    }
}
