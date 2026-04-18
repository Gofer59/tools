package com.trilingua.app.mt

import com.trilingua.app.model.Language

interface Translator {
    suspend fun translate(text: String, from: Language, to: Language): String
    fun close()
}
