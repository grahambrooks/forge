package com.grahambrooks.forge.cli

import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertTrue
import org.junit.jupiter.api.Assumptions.assumeTrue
import org.junit.jupiter.api.Test
import org.junit.jupiter.api.io.TempDir
import java.io.File

/** Drives the crate's debug build (`make build`); skipped when it has not been built. */
class ForgeCliTest {
    private val binary = File(System.getProperty("forge.binary", ""))
    private val examples = File(System.getProperty("forge.examples", ""))

    private fun cli(): ForgeCli {
        assumeTrue(binary.canExecute(), "forge debug binary not built: $binary")
        return ForgeCli(binary.path)
    }

    @Test
    fun `renders every view of a file on disk`() {
        val result = cli().render(ForgeSource.OnDisk(File(examples, "payments.forge")), "outline")

        assertTrue(result is RenderResult.Rendered, result.toString())
        val views = (result as RenderResult.Rendered).views
        assertTrue(views.size > 5, "expected the payments example's views, got ${views.map { it.key }}")
        assertEquals("SystemContext", views.first().key)
        assertTrue(views.first().title.contains("System Context"))
        assertTrue(views.all { it.svg.startsWith("<svg") })
    }

    @Test
    fun `renders unsaved text with includes resolved from the working directory`() {
        val dir = File(examples, "multi-file")
        val text = File(dir, "forge.forge").readText()

        val result = cli().render(ForgeSource.InMemory(text, dir), "filled")

        assertTrue(result is RenderResult.Rendered, result.toString())
        assertTrue((result as RenderResult.Rendered).views.isNotEmpty())
    }

    @Test
    fun `reports parse errors without the CLI's Error prefix`(@TempDir dir: File) {
        val result = cli().render(ForgeSource.InMemory("forge \"Broken\" {\n  model {\n", dir), "outline")

        assertTrue(result is RenderResult.Failed, result.toString())
        val message = (result as RenderResult.Failed).message
        assertTrue(message.isNotBlank())
        assertTrue(!message.startsWith("Error:"), message)
    }

    @Test
    fun `explains a missing binary`(@TempDir dir: File) {
        val result = ForgeCli(File(dir, "no-such-forge").path)
            .render(ForgeSource.InMemory("", dir), "outline")

        assertTrue(result is RenderResult.Failed)
        assertTrue((result as RenderResult.Failed).message.contains("Settings | Tools | Forge Preview"))
    }

    @Test
    fun `parses view metadata from export json`() {
        val views = ForgeCli.parseViews(
            """{"views":[{"key":"Ctx","title":"Context","kind":"SystemContext"},{"key":"Bare"}]}""",
        )

        assertEquals(listOf("Ctx", "Bare"), views.map { it.key })
        assertEquals(listOf("Context", "Bare"), views.map { it.title })
    }

    @Test
    fun `a model without views has none`() {
        assertEquals(emptyList<ForgeView>(), ForgeCli.parseViews("""{"name":"x"}"""))
    }
}
