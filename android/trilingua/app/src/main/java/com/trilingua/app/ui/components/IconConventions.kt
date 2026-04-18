package com.trilingua.app.ui.components

/**
 * Icon contentDescription conventions used throughout Trilingua UI components:
 *
 * - **Icon inside a Button with a visible Text label**: `contentDescription = null` (decorative).
 *   The Text label already conveys the action; a duplicate description adds noise for TalkBack.
 *
 * - **Standalone Icon or IconButton without a text label**: `contentDescription = stringResource(R.string.…)`.
 *   The description must unambiguously name the action or state the icon represents.
 *
 * - **IconButton where both icon and state vary**: description must reflect the current
 *   enabled/pressed state (e.g. "Recording — release to stop" vs "Hold to talk").
 *   Never use a static string when the visible state changes.
 */
// This file is documentation-only; no runtime code is declared here.
