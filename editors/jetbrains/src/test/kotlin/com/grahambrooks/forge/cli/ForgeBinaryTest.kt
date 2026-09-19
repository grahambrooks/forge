package com.grahambrooks.forge.cli

import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Test
import org.junit.jupiter.api.io.TempDir
import java.io.File

class ForgeBinaryTest {
    @Test
    fun `configured path wins`() {
        assertEquals("/opt/forge", ForgeBinary.resolve(" /opt/forge ", path = "", home = "/nowhere"))
    }

    @Test
    fun `finds forge in cargo bin when PATH lacks it`(@TempDir home: File) {
        val forge = File(home, ".cargo/bin/forge").apply {
            parentFile.mkdirs()
            writeText("#!/bin/sh\n")
            setExecutable(true)
        }

        assertEquals(forge.path, ForgeBinary.resolve("", path = "", home = home.path, isWindows = false))
    }

    @Test
    fun `falls back to the bare name`(@TempDir home: File) {
        val resolved = ForgeBinary.resolve("", path = home.path, home = home.path, isWindows = false)

        // Either the bare name or a real system install, never something from the empty dirs.
        assert(resolved == "forge" || File(resolved).canExecute())
    }
}
