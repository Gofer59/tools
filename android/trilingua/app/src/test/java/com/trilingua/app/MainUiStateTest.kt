package com.trilingua.app

import com.trilingua.app.model.*
import org.junit.Assert.*
import org.junit.Test

class MainUiStateTest {

    private fun makeState(
        source: Language = Language.EN,
        target: Language = Language.HU,
        pipelineState: PipelineState = PipelineState.Idle,
        bootState: BootState = BootState.Ready
    ) = MainUiState(
        source = source,
        target = target,
        bootState = bootState,
        pipelineState = pipelineState,
        sourceText = "",
        targetText = "",
        error = null
    )

    @Test
    fun `normalize changes target when source equals target`() {
        val state = makeState(source = Language.EN, target = Language.EN)
        val normalized = state.normalize()
        assertNotEquals(normalized.source, normalized.target)
    }

    @Test
    fun `normalize is identity when source and target differ`() {
        val state = makeState(source = Language.EN, target = Language.HU)
        assertEquals(state, state.normalize())
    }

    @Test
    fun `canCancel is true for Recording`() {
        assertTrue(makeState(pipelineState = PipelineState.Recording(0L)).canCancel)
    }

    @Test
    fun `canCancel is true for Transcribing`() {
        assertTrue(makeState(pipelineState = PipelineState.Transcribing(Language.EN)).canCancel)
    }

    @Test
    fun `canCancel is true for Translating`() {
        assertTrue(makeState(pipelineState = PipelineState.Translating(Direction(Language.EN, Language.FR))).canCancel)
    }

    @Test
    fun `canCancel is true for Speaking`() {
        assertTrue(makeState(pipelineState = PipelineState.Speaking(Language.FR)).canCancel)
    }

    @Test
    fun `canCancel is false for Idle`() {
        assertFalse(makeState(pipelineState = PipelineState.Idle).canCancel)
    }

    @Test
    fun `canCancel is false for Done`() {
        assertFalse(makeState(pipelineState = PipelineState.Done).canCancel)
    }

    @Test
    fun `canCancel is false for Failed`() {
        assertFalse(makeState(pipelineState = PipelineState.Failed(TrilinguaError.TooShort)).canCancel)
    }

    @Test
    fun `initial has Initializing bootState`() {
        assertEquals(BootState.Initializing, MainUiState.initial().bootState)
    }
}
