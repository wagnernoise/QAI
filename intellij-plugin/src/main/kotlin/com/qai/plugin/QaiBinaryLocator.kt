package com.qai.plugin

import java.io.File

/**
 * Locates the `qai-cli` binary.
 *
 * Search order:
 * 1. Common PATH directories (cross-platform).
 * 2. Bundled resource inside the plugin JAR (future: ship binary with plugin).
 */
object QaiBinaryLocator {

    private val BINARY_NAME = if (isWindows()) "qai-cli.exe" else "qai-cli"

    private val COMMON_PATHS = listOf(
        "/usr/local/bin",
        "/usr/bin",
        "/opt/homebrew/bin",
        System.getProperty("user.home") + "/bin",
        System.getProperty("user.home") + "/.cargo/bin",
    )

    /** Returns the absolute path to the binary, or null if not found. */
    fun find(): String? {
        // 1. Search PATH environment variable entries
        val pathEnv = System.getenv("PATH") ?: ""
        val pathDirs = pathEnv.split(File.pathSeparator)
        for (dir in pathDirs + COMMON_PATHS) {
            val candidate = File(dir, BINARY_NAME)
            if (candidate.exists() && candidate.canExecute()) {
                return candidate.absolutePath
            }
        }
        return null
    }

    /** Returns true when running on Windows. */
    fun isWindows(): Boolean =
        System.getProperty("os.name", "").lowercase().contains("win")
}
