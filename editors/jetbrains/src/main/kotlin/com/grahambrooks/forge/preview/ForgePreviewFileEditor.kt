package com.grahambrooks.forge.preview

import com.grahambrooks.forge.ForgeFileType
import com.grahambrooks.forge.cli.ForgeCli
import com.grahambrooks.forge.cli.ForgeSource
import com.grahambrooks.forge.cli.RenderResult
import com.grahambrooks.forge.settings.ForgeSettings
import com.intellij.openapi.application.ApplicationManager
import com.intellij.openapi.application.ModalityState
import com.intellij.openapi.application.runReadAction
import com.intellij.openapi.editor.event.DocumentEvent
import com.intellij.openapi.editor.event.DocumentListener
import com.intellij.openapi.fileEditor.FileDocumentManager
import com.intellij.openapi.fileEditor.FileEditor
import com.intellij.openapi.fileEditor.FileEditorState
import com.intellij.openapi.util.Disposer
import com.intellij.openapi.util.UserDataHolderBase
import com.intellij.openapi.vfs.VirtualFile
import com.intellij.openapi.vfs.VirtualFileManager
import com.intellij.openapi.vfs.newvfs.BulkFileListener
import com.intellij.openapi.vfs.newvfs.events.VFileEvent
import com.intellij.util.Alarm
import com.intellij.util.EnvironmentUtil
import java.beans.PropertyChangeListener
import java.io.File
import java.util.concurrent.atomic.AtomicInteger
import javax.swing.JComponent

/**
 * The diagram half of the split editor. Re-renders through the `forge` CLI, debounced, whenever
 * the document changes, another `.forge` file changes on disk (it may be `!include`d), or the
 * settings change.
 */
class ForgePreviewFileEditor(private val file: VirtualFile) : UserDataHolderBase(), FileEditor {

    private val panel = ForgePreviewPanel(onRefresh = { scheduleRender(0) })
    private val alarm = Alarm(Alarm.ThreadToUse.POOLED_THREAD, this)

    // Renders run off the EDT and can overlap; only the newest one may update the panel.
    private val generation = AtomicInteger()

    @Volatile
    private var disposed = false

    init {
        Disposer.register(this, panel)

        FileDocumentManager.getInstance().getDocument(file)?.addDocumentListener(
            object : DocumentListener {
                override fun documentChanged(event: DocumentEvent) = scheduleRender(RENDER_DELAY_MS)
            },
            this,
        )

        val connection = ApplicationManager.getApplication().messageBus.connect(this)
        connection.subscribe(
            VirtualFileManager.VFS_CHANGES,
            object : BulkFileListener {
                override fun after(events: List<VFileEvent>) {
                    if (events.any { it.file?.let(ForgeFileType::matches) == true && it.file != file }) {
                        scheduleRender(RENDER_DELAY_MS)
                    }
                }
            },
        )
        connection.subscribe(ForgeSettings.TOPIC, ForgeSettings.Listener { scheduleRender(0) })

        scheduleRender(0)
    }

    private fun scheduleRender(delayMs: Int) {
        val ticket = generation.incrementAndGet()
        alarm.cancelAllRequests()
        alarm.addRequest({ render(ticket) }, delayMs)
    }

    /** Runs on a pooled thread. */
    private fun render(ticket: Int) {
        if (!file.isValid) return
        val source = runReadAction { currentSource() }
        val settings = ForgeSettings.get()
        val result = ForgeCli(settings.binary, EnvironmentUtil.getEnvironmentMap())
            .render(source, settings.style)
        ApplicationManager.getApplication().invokeLater(
            { if (ticket == generation.get()) panel.show(result) },
            ModalityState.any(),
        ) { disposed }
    }

    /**
     * Saved files are rendered from disk, which works with every forge release. Unsaved edits are
     * piped through stdin, which needs a release that accepts `--source -`.
     */
    private fun currentSource(): ForgeSource {
        val documents = FileDocumentManager.getInstance()
        val document = documents.getDocument(file)
        val localFile = file.takeIf { it.isInLocalFileSystem }?.let { File(it.path) }
        return if (localFile != null && (document == null || !documents.isDocumentUnsaved(document))) {
            ForgeSource.OnDisk(localFile)
        } else {
            ForgeSource.InMemory(document?.text ?: String(file.contentsToByteArray(), file.charset), localFile?.parentFile)
        }
    }

    override fun getComponent(): JComponent = panel

    override fun getPreferredFocusedComponent(): JComponent = panel

    override fun getName(): String = "Forge Preview"

    override fun getFile(): VirtualFile = file

    override fun setState(state: FileEditorState) = Unit

    override fun isModified(): Boolean = false

    override fun isValid(): Boolean = file.isValid

    override fun addPropertyChangeListener(listener: PropertyChangeListener) = Unit

    override fun removePropertyChangeListener(listener: PropertyChangeListener) = Unit

    override fun dispose() {
        disposed = true
    }

    private companion object {
        const val RENDER_DELAY_MS = 400
    }
}
