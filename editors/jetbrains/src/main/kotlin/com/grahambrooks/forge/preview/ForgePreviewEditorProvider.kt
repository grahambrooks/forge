package com.grahambrooks.forge.preview

import com.grahambrooks.forge.ForgeFileType
import com.intellij.diff.editor.DiffContentVirtualFile
import com.intellij.openapi.fileEditor.FileEditor
import com.intellij.openapi.fileEditor.FileEditorPolicy
import com.intellij.openapi.fileEditor.FileEditorProvider
import com.intellij.openapi.fileEditor.TextEditor
import com.intellij.openapi.fileEditor.TextEditorWithPreview
import com.intellij.openapi.fileEditor.impl.text.TextEditorProvider
import com.intellij.openapi.project.DumbAware
import com.intellij.openapi.project.Project
import com.intellij.openapi.vfs.VirtualFile

/** Opens `.forge` files in a split "source | diagram" editor. */
class ForgePreviewEditorProvider : FileEditorProvider, DumbAware {

    /**
     * Only files being edited in place get the preview.
     *
     * A diff tab is not one: reviewing a change shows two revisions of the text, neither of which
     * is a file `forge` could be run on, and the tab has no directory for `!include` to resolve
     * against. The same goes for anything outside the local file system, such as a revision opened
     * from the Git log. Without this, the preview attaches to those tabs and renders something
     * misleading or nothing at all.
     */
    override fun accept(project: Project, file: VirtualFile): Boolean =
        ForgeFileType.matches(file) && file !is DiffContentVirtualFile && file.isInLocalFileSystem

    override fun createEditor(project: Project, file: VirtualFile): FileEditor {
        val textEditor = TextEditorProvider.getInstance().createEditor(project, file) as TextEditor
        return TextEditorWithPreview(
            textEditor,
            ForgePreviewFileEditor(file),
            "Forge",
            TextEditorWithPreview.Layout.SHOW_EDITOR_AND_PREVIEW,
        )
    }

    override fun getEditorTypeId(): String = "forge-preview-editor"

    override fun getPolicy(): FileEditorPolicy = FileEditorPolicy.HIDE_DEFAULT_EDITOR
}
