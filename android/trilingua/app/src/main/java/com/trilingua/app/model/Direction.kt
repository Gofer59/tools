package com.trilingua.app.model

data class Direction(val from: Language, val to: Language) {
    init { require(from != to) { "source and target must differ" } }
    val id: String get() = "${from.tag}-${to.tag}"
    companion object {
        val supported: Set<Direction> = buildSet {
            val langs = Language.values()
            for (a in langs) for (b in langs) if (a != b) add(Direction(a, b))
        }
    }
}
