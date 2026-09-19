package com.grahambrooks.forge.cli

import java.io.File

/** Locates the `forge` executable when the user has not configured one. */
object ForgeBinary {
    /**
     * IDEs launched from the macOS Dock do not inherit the login shell's `PATH`, so the usual
     * install locations (Homebrew, `cargo install`) are searched after `PATH`.
     */
    fun resolve(
        configured: String,
        path: String? = System.getenv("PATH"),
        home: String = System.getProperty("user.home"),
        isWindows: Boolean = System.getProperty("os.name").startsWith("Windows"),
    ): String {
        if (configured.isNotBlank()) return configured.trim()
        val exe = if (isWindows) "forge.exe" else "forge"
        val dirs = path.orEmpty().split(File.pathSeparator).filter { it.isNotBlank() } +
            listOf("$home/.cargo/bin", "/opt/homebrew/bin", "/usr/local/bin")
        return dirs.map { File(it, exe) }.firstOrNull { it.isFile && it.canExecute() }?.path ?: exe
    }
}
