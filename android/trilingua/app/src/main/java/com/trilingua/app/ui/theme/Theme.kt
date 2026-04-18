package com.trilingua.app.ui.theme

import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.darkColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.graphics.Color

// Always use the explicit Trilingua brand scheme — dynamic color is intentionally disabled
// to preserve the paprika/ember identity across all Android 12+ devices.
private val TrilinguaColorScheme = darkColorScheme(
    primary          = Trilingua_Primary,
    onPrimary        = Trilingua_OnPrimary,
    secondary        = Trilingua_Secondary,
    onSecondary      = Color(0xFF001C3D),
    tertiary         = Trilingua_Tertiary,
    onTertiary       = Color(0xFF003910),
    background       = Trilingua_Background,
    onBackground     = Color(0xFFE3E2E6),
    surface          = Trilingua_Surface,
    onSurface        = Color(0xFFE3E2E6),
    error            = Trilingua_Error,
    onError          = Color(0xFF410002)
)

@Composable
fun TrilinguaTheme(content: @Composable () -> Unit) {
    MaterialTheme(colorScheme = TrilinguaColorScheme, typography = TrilinguaTypography, content = content)
}
