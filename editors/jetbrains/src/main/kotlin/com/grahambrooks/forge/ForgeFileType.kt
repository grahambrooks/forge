package com.grahambrooks.forge

import com.intellij.openapi.fileTypes.FileType
import com.intellij.openapi.util.IconLoader
import com.intellij.openapi.vfs.VirtualFile
import javax.swing.Icon

/**
 * A plain-text file type for `.forge`. There is deliberately no language or parser here --
 * editing support comes from `forge lsp` (see EDITORS.md); this plugin only adds the preview.
 */
object ForgeFileType : FileType {
    private val forgeIcon: Icon by lazy { IconLoader.getIcon("/icons/forge.svg", ForgeFileType::class.java) }

    override fun getName(): String = "Forge"

    override fun getDescription(): String = "Forge model"

    override fun getDefaultExtension(): String = "forge"

    override fun getIcon(): Icon = forgeIcon

    override fun isBinary(): Boolean = false

    fun matches(file: VirtualFile): Boolean = !file.isDirectory && file.extension == defaultExtension
}
