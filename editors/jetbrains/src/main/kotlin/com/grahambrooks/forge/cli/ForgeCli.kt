package com.grahambrooks.forge.cli

import com.google.gson.JsonParseException
import com.google.gson.JsonParser
import java.io.File
import java.io.IOException
import java.nio.file.Files
import java.time.Duration
import java.util.concurrent.CompletableFuture
import java.util.concurrent.TimeUnit

/** What to render: a file as it is on disk, or editor text that has not been saved yet. */
sealed interface ForgeSource {
    val workingDir: File?

    data class OnDisk(val file: File) : ForgeSource {
        override val workingDir: File? get() = file.absoluteFile.parentFile
    }

    /** `!include` paths in [text] resolve against [workingDir]. */
    data class InMemory(val text: String, override val workingDir: File?) : ForgeSource
}

data class ForgeView(
    val key: String,
    val title: String,
    val kind: String,
    val svg: String,
)

sealed interface RenderResult {
    data class Rendered(val views: List<ForgeView>) : RenderResult

    data class Failed(val message: String) : RenderResult
}

/**
 * Renders a model by shelling out to the `forge` binary. The plugin never parses the DSL itself:
 * the CLI is the single implementation, so the preview cannot drift from `forge build`.
 *
 * One render is two invocations -- `export` for the view list and titles, `build` for the SVGs --
 * because `build` writes files named by view key and reports nothing else.
 */
class ForgeCli(
    private val binary: String,
    private val environment: Map<String, String> = emptyMap(),
    private val timeout: Duration = Duration.ofSeconds(30),
) {
    fun render(source: ForgeSource, style: String): RenderResult {
        try {
            val export = run(source, listOf("export", "--format", "json"))
            if (!export.ok) return RenderResult.Failed(errorMessage(export, source))
            val views = try {
                parseViews(export.stdout)
            } catch (e: JsonParseException) {
                return RenderResult.Failed("Unexpected output from `forge export`: ${e.message}")
            } catch (e: IllegalStateException) {
                return RenderResult.Failed("Unexpected output from `forge export`: ${e.message}")
            }

            val outDir = Files.createTempDirectory("forge-preview").toFile()
            try {
                val build = run(source, listOf("build", "--out", outDir.path, "--style", style))
                if (!build.ok) return RenderResult.Failed(errorMessage(build, source))
                return RenderResult.Rendered(
                    views.mapNotNull { view ->
                        val svg = File(outDir, "${view.key}.svg")
                        if (svg.isFile) view.copy(svg = svg.readText()) else null
                    },
                )
            } finally {
                outDir.deleteRecursively()
            }
        } catch (e: IOException) {
            return RenderResult.Failed(
                "Could not run `$binary`: ${e.message}\n" +
                    "Install forge, or set its location in Settings | Tools | Forge Preview.",
            )
        }
    }

    private data class Output(val exitCode: Int, val stdout: String, val stderr: String) {
        val ok: Boolean get() = exitCode == 0
    }

    private fun run(source: ForgeSource, args: List<String>): Output {
        val sourceArg = when (source) {
            is ForgeSource.OnDisk -> source.file.absolutePath
            is ForgeSource.InMemory -> "-"
        }
        val command = listOf(binary, args.first(), "--source", sourceArg) + args.drop(1)
        val process = ProcessBuilder(command)
            .directory(source.workingDir?.takeIf { it.isDirectory })
            .apply { environment().putAll(environment) }
            .start()

        // Drain both pipes concurrently so neither can fill and block the other.
        val stdout = CompletableFuture.supplyAsync { process.inputStream.bufferedReader().readText() }
        val stderr = CompletableFuture.supplyAsync { process.errorStream.bufferedReader().readText() }
        process.outputStream.use { stdin ->
            if (source is ForgeSource.InMemory) stdin.write(source.text.toByteArray())
        }

        if (!process.waitFor(timeout.toMillis(), TimeUnit.MILLISECONDS)) {
            process.destroyForcibly()
            return Output(-1, "", "Error: forge did not finish within ${timeout.seconds}s")
        }
        return Output(process.exitValue(), stdout.join(), stderr.join())
    }

    private fun errorMessage(output: Output, source: ForgeSource): String {
        // Releases before stdin support treat `-` as a file name.
        if (source is ForgeSource.InMemory && output.stderr.contains("-: No such file")) {
            return "This forge release cannot render unsaved changes. " +
                "Save the file to preview it, or upgrade forge."
        }
        val errors = output.stderr.lines()
            .filter { it.startsWith("Error:") }
            .map { it.removePrefix("Error:").trim() }
        return when {
            errors.isNotEmpty() -> errors.joinToString("\n")
            output.stderr.isNotBlank() -> output.stderr.trim()
            else -> "forge exited with code ${output.exitCode}"
        }
    }

    internal companion object {
        fun parseViews(exportJson: String): List<ForgeView> {
            val views = JsonParser.parseString(exportJson).asJsonObject.getAsJsonArray("views")
                ?: return emptyList()
            return views.map { element ->
                val view = element.asJsonObject
                val key = view.get("key").asString
                ForgeView(
                    key = key,
                    title = view.get("title")?.takeUnless { it.isJsonNull }?.asString ?: key,
                    kind = view.get("kind")?.takeUnless { it.isJsonNull }?.asString ?: "",
                    svg = "",
                )
            }
        }
    }
}
