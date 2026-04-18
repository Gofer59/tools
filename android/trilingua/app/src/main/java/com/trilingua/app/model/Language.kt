package com.trilingua.app.model

import java.util.Locale

enum class Language(val tag: String, val displayName: String) {
    EN("en", "English"),
    FR("fr", "Français"),
    HU("hu", "Magyar");

    fun toLocale(): Locale = when (this) {
        EN -> Locale.forLanguageTag("en-US")
        FR -> Locale.forLanguageTag("fr-FR")
        HU -> Locale.forLanguageTag("hu-HU")
    }

    companion object {
        fun fromTag(tag: String): Language = values().first { it.tag == tag }
    }
}
