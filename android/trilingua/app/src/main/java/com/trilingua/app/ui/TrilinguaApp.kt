package com.trilingua.app.ui

import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import com.trilingua.app.ui.theme.TrilinguaTheme

@Composable
fun TrilinguaApp(vm: MainViewModel) {
    TrilinguaTheme {
        Surface(Modifier.fillMaxSize(), color = MaterialTheme.colorScheme.background) {
            MainScreen(vm)
        }
    }
}
