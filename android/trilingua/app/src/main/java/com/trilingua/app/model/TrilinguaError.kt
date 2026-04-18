package com.trilingua.app.model

import androidx.annotation.StringRes
import com.trilingua.app.R

/**
 * Typed error surface for the Trilingua pipeline.
 * Each variant carries the minimum context needed to display a localised message.
 * UI consumers call [errorMessage] in ErrorBanner via
 * `stringResource(error.messageRes, *error.formatArgs)`.
 */
sealed class TrilinguaError(
    @StringRes val messageRes: Int,
    val formatArgs: Array<out Any> = emptyArray()
) {
    data object MicDenied : TrilinguaError(R.string.err_mic_denied)
    data class ModelMissing(val which: String) : TrilinguaError(R.string.err_model_missing, arrayOf(which))
    data class VoiceMissing(val lang: Language) : TrilinguaError(R.string.err_voice_missing, arrayOf(lang.displayName))
    data class NativeCrash(val msg: String) : TrilinguaError(R.string.err_native_crash, arrayOf(msg))
    data class UnsupportedPair(val direction: Direction) : TrilinguaError(R.string.err_unsupported_pair, arrayOf(direction.id))
    data object TooShort : TrilinguaError(R.string.err_too_short)
    data object TooLong : TrilinguaError(R.string.err_too_long)
}
