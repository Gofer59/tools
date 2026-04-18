package com.trilingua.app

import com.trilingua.app.model.Language
import org.junit.Assert.*
import org.junit.Test

class LanguageTest {

    @Test
    fun `fromTag round-trips for all languages`() {
        for (lang in Language.values()) {
            assertEquals(lang, Language.fromTag(lang.tag))
        }
    }

    @Test
    fun `fromTag returns correct language for known tags`() {
        assertEquals(Language.EN, Language.fromTag("en"))
        assertEquals(Language.FR, Language.fromTag("fr"))
        assertEquals(Language.HU, Language.fromTag("hu"))
    }

    @Test(expected = NoSuchElementException::class)
    fun `fromTag throws for unknown tag`() {
        Language.fromTag("zz")
    }

    @Test
    fun `all languages have non-blank displayName and tag`() {
        for (lang in Language.values()) {
            assertTrue("${lang.name}.tag is blank", lang.tag.isNotBlank())
            assertTrue("${lang.name}.displayName is blank", lang.displayName.isNotBlank())
        }
    }
}
