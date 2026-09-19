package com.grahambrooks.forge.preview

import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertFalse
import org.junit.jupiter.api.Assertions.assertTrue
import org.junit.jupiter.api.Test

class PreviewHtmlTest {
    private val svg = """<svg><style>.a{} @media(prefers-color-scheme:dark) { .a{} }</style></svg>"""

    @Test
    fun `dark IDE forces the dark palette`() {
        val themed = PreviewHtml.applyTheme(svg, dark = true)

        assertTrue(themed.contains("@media all"))
        assertFalse(themed.contains("prefers-color-scheme"))
    }

    @Test
    fun `light IDE disables the dark palette`() {
        assertTrue(PreviewHtml.applyTheme(svg, dark = false).contains("@media not all"))
    }

    @Test
    fun `svg is passed to the page as a JS string literal`() {
        val script = PreviewHtml.showScript("<svg>\"quoted\"\n</svg>")

        assertTrue(script.startsWith("forgeShow(\""))
        assertFalse(script.contains("\n"))
        assertEquals(");", script.takeLast(2))
    }
}
