package com.trilingua.app

import com.trilingua.app.model.Direction
import com.trilingua.app.model.Language
import org.junit.Assert.*
import org.junit.Test

class DirectionTest {

    @Test
    fun `supported has exactly 6 entries`() {
        assertEquals(6, Direction.supported.size)
    }

    @Test
    fun `all supported pairs have distinct source and target`() {
        for (d in Direction.supported) {
            assertNotEquals("Direction ${d.id} has source == target", d.from, d.to)
        }
    }

    @Test
    fun `supported contains all language pairs`() {
        val langs = Language.values()
        for (a in langs) {
            for (b in langs) {
                if (a != b) {
                    assertTrue("Direction(${a.tag},${b.tag}) missing from supported",
                        Direction.supported.contains(Direction(a, b)))
                }
            }
        }
    }

    @Test(expected = IllegalArgumentException::class)
    fun `Direction rejects same source and target`() {
        Direction(Language.EN, Language.EN)
    }
}
