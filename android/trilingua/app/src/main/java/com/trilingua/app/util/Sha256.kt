package com.trilingua.app.util

import java.io.File
import java.security.MessageDigest

object Sha256 {
    fun ofFile(file: File): String {
        val digest = MessageDigest.getInstance("SHA-256")
        file.inputStream().use { stream ->
            val buf = ByteArray(1024 * 1024) // 1 MB chunks
            var n: Int
            while (stream.read(buf).also { n = it } != -1) {
                digest.update(buf, 0, n)
            }
        }
        return digest.digest().joinToString("") { "%02x".format(it) }
    }
}
