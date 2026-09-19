package com.grahambrooks.forge.preview

import com.intellij.diff.DiffContentFactory
import com.intellij.diff.chains.SimpleDiffRequestChain
import com.intellij.diff.editor.ChainDiffVirtualFile
import com.intellij.diff.requests.SimpleDiffRequest
import com.intellij.openapi.fileEditor.FileEditorProvider
import com.intellij.openapi.fileEditor.ex.FileEditorProviderManager
import com.intellij.openapi.vfs.LocalFileSystem
import com.intellij.openapi.vfs.VirtualFile
import com.intellij.testFramework.LightVirtualFile
import com.intellij.testFramework.fixtures.BasePlatformTestCase
import java.nio.file.Files

/**
 * Which files the preview attaches to. The interesting cases are the ones where the editor shows
 * something other than the working copy on disk: no directory for `!include` to resolve against,
 * and content that is not a file `forge` could be run on.
 *
 * The diff case is a constructed one. Opening a diff on a `.forge` file in the IDE shows the text
 * diff alone, so the platform is not offering the preview there anyway; the test pins that a diff
 * tab named after the file it reviews cannot pick the preview up.
 */
class ForgePreviewEditorProviderTest : BasePlatformTestCase() {

    private val model = """
        forge "Preview" {
          model {
            customer = person "Customer"
          }
        }
    """.trimIndent()

    /** A real file on disk: the light fixture's own temp files are in an in-memory file system. */
    private fun localForgeFile(): VirtualFile {
        val dir = Files.createTempDirectory("forge-preview-test")
        val path = Files.writeString(dir.resolve("payments.forge"), model)
        path.toFile().deleteOnExit()
        dir.toFile().deleteOnExit()
        return LocalFileSystem.getInstance().refreshAndFindFileByNioFile(path)!!
    }

    fun `test attaches to a forge file on disk`() {
        assertTrue(providersFor(localForgeFile()).any { it is ForgePreviewEditorProvider })
    }

    fun `test does not attach to a diff tab for a forge file`() {
        val file = localForgeFile()
        val content = DiffContentFactory.getInstance().create(project, file)
        val chain = SimpleDiffRequestChain(SimpleDiffRequest(file.name, content, content, "Before", "After"))

        val diffFile = ChainDiffVirtualFile(chain, file.name)

        assertFalse(providersFor(diffFile).any { it is ForgePreviewEditorProvider })
    }

    fun `test does not attach to a file outside the local file system`() {
        // Stands in for a VCS revision opened from the Git log: named `.forge`, with content, but
        // no directory on disk, so includes could not resolve.
        val revision = LightVirtualFile("payments.forge", model)

        assertFalse(providersFor(revision).any { it is ForgePreviewEditorProvider })
    }

    private fun providersFor(file: VirtualFile): List<FileEditorProvider> =
        FileEditorProviderManager.getInstance().getProviderList(project, file)
}
