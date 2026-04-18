package com.trilingua.app.ui.components

import androidx.compose.foundation.layout.*
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.SwapHoriz
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import com.trilingua.app.R
import com.trilingua.app.model.Language

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun LanguageRow(
    source: Language,
    target: Language,
    onSwap: () -> Unit,
    onSourceChange: (Language) -> Unit,
    onTargetChange: (Language) -> Unit,
    enabled: Boolean,
    modifier: Modifier = Modifier
) {
    Row(
        modifier = modifier.fillMaxWidth(),
        verticalAlignment = Alignment.CenterVertically
    ) {
        LanguageDropdown(
            selected = source,
            onSelect = onSourceChange,
            enabled = enabled,
            disabledOption = null,
            modifier = Modifier.weight(1f)
        )
        IconButton(
            onClick = onSwap,
            enabled = enabled,
            modifier = Modifier.padding(horizontal = 4.dp)
        ) {
            Icon(
                imageVector = Icons.Default.SwapHoriz,
                // Icon inside a button with no visible text label needs a descriptive string
                contentDescription = stringResource(R.string.swap_languages)
            )
        }
        LanguageDropdown(
            selected = target,
            onSelect = onTargetChange,
            enabled = enabled,
            // P5: disable the source language in target dropdown so normalize() never silently flips
            disabledOption = source,
            modifier = Modifier.weight(1f)
        )
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun LanguageDropdown(
    selected: Language,
    onSelect: (Language) -> Unit,
    enabled: Boolean,
    disabledOption: Language?,
    modifier: Modifier = Modifier
) {
    var expanded by remember { mutableStateOf(false) }
    ExposedDropdownMenuBox(
        expanded = expanded && enabled,
        onExpandedChange = { if (enabled) expanded = it },
        modifier = modifier
    ) {
        OutlinedTextField(
            value = selected.displayName,
            onValueChange = {},
            readOnly = true,
            enabled = enabled,
            trailingIcon = { ExposedDropdownMenuDefaults.TrailingIcon(expanded = expanded && enabled) },
            colors = ExposedDropdownMenuDefaults.outlinedTextFieldColors(),
            modifier = Modifier
                .menuAnchor()
                .fillMaxWidth()
        )
        ExposedDropdownMenu(
            expanded = expanded && enabled,
            onDismissRequest = { expanded = false }
        ) {
            Language.values().forEach { lang ->
                val itemEnabled = lang != disabledOption
                DropdownMenuItem(
                    text = { Text(lang.displayName) },
                    onClick = {
                        if (itemEnabled) {
                            onSelect(lang)
                            expanded = false
                        }
                    },
                    enabled = itemEnabled
                )
            }
        }
    }
}
