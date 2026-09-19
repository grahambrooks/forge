package com.grahambrooks.forge.settings

import com.grahambrooks.forge.cli.ForgeBinary
import com.intellij.openapi.application.ApplicationManager
import com.intellij.openapi.components.PersistentStateComponent
import com.intellij.openapi.components.Service
import com.intellij.openapi.components.State
import com.intellij.openapi.components.Storage
import com.intellij.openapi.components.service
import com.intellij.openapi.options.BoundConfigurable
import com.intellij.openapi.ui.DialogPanel
import com.intellij.ui.dsl.builder.bindItem
import com.intellij.ui.dsl.builder.bindText
import com.intellij.ui.dsl.builder.columns
import com.intellij.ui.dsl.builder.COLUMNS_LARGE
import com.intellij.ui.dsl.builder.panel
import com.intellij.util.messages.Topic

@Service(Service.Level.APP)
@State(name = "ForgePreviewSettings", storages = [Storage("forge-preview.xml")])
class ForgeSettings : PersistentStateComponent<ForgeSettings.State> {
    data class State(
        var binaryPath: String = "",
        var style: String = STYLES.first(),
    )

    private var state = State()

    override fun getState(): State = state

    override fun loadState(state: State) {
        this.state = state
    }

    /** The configured binary, or the one found on `PATH` / in the usual install locations. */
    val binary: String get() = ForgeBinary.resolve(state.binaryPath)

    val style: String get() = state.style.takeIf { it in STYLES } ?: STYLES.first()

    fun interface Listener {
        fun settingsChanged()
    }

    companion object {
        val STYLES = listOf("outline", "filled")

        @Topic.AppLevel
        val TOPIC: Topic<Listener> = Topic.create("Forge preview settings", Listener::class.java)

        fun get(): ForgeSettings = service()
    }
}

class ForgeSettingsConfigurable : BoundConfigurable("Forge Preview") {
    override fun createPanel(): DialogPanel {
        val state = ForgeSettings.get().state
        return panel {
            row("forge binary:") {
                textField()
                    .columns(COLUMNS_LARGE)
                    .bindText({ state.binaryPath }, { state.binaryPath = it.trim() })
                    .comment("Leave empty to use <code>${ForgeBinary.resolve("")}</code>")
            }
            row("Diagram style:") {
                comboBox(ForgeSettings.STYLES)
                    .bindItem({ state.style }, { state.style = it ?: ForgeSettings.STYLES.first() })
            }
        }
    }

    override fun apply() {
        super.apply()
        ApplicationManager.getApplication().messageBus.syncPublisher(ForgeSettings.TOPIC).settingsChanged()
    }
}
