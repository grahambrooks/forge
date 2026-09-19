package com.grahambrooks.forge.preview

import com.grahambrooks.forge.cli.ForgeView
import com.grahambrooks.forge.cli.RenderResult
import com.intellij.icons.AllIcons
import com.intellij.ide.ui.LafManagerListener
import com.intellij.openapi.Disposable
import com.intellij.openapi.actionSystem.ActionManager
import com.intellij.openapi.actionSystem.AnActionEvent
import com.intellij.openapi.actionSystem.DefaultActionGroup
import com.intellij.openapi.application.ApplicationManager
import com.intellij.openapi.editor.colors.EditorColorsManager
import com.intellij.openapi.project.DumbAwareAction
import com.intellij.openapi.ui.ComboBox
import com.intellij.openapi.util.Disposer
import com.intellij.ui.ColorUtil
import com.intellij.ui.JBColor
import com.intellij.ui.SimpleListCellRenderer
import com.intellij.ui.components.JBLabel
import com.intellij.ui.jcef.JBCefApp
import com.intellij.ui.jcef.JBCefBrowser
import com.intellij.util.ui.JBUI
import com.intellij.util.ui.UIUtil
import org.cef.browser.CefBrowser
import org.cef.browser.CefFrame
import org.cef.handler.CefLoadHandlerAdapter
import java.awt.BorderLayout
import javax.swing.DefaultComboBoxModel
import javax.swing.Icon
import javax.swing.JPanel
import javax.swing.SwingConstants

/**
 * Toolbar (view picker, refresh, zoom), an error banner, and a JCEF browser showing the SVG.
 *
 * When a render fails the last good diagram stays on screen under the error, so a half-typed
 * edit doesn't blank the preview.
 */
class ForgePreviewPanel(private val onRefresh: () -> Unit) : JPanel(BorderLayout()), Disposable {

    private val views = DefaultComboBoxModel<ForgeView>()
    private var replacingViews = false
    private val viewPicker = ComboBox(views).apply {
        renderer = SimpleListCellRenderer.create("") { it.title }
        addActionListener { if (!replacingViews) showSelected() }
    }
    private val errorBanner = JBLabel().apply {
        isVisible = false
        foreground = JBColor.RED
        border = JBUI.Borders.empty(4, 8)
        verticalAlignment = SwingConstants.TOP
    }
    private val browser: JBCefBrowser? = if (JBCefApp.isSupported()) JBCefBrowser() else null

    // Scripts run before the page has loaded are dropped, so the latest diagram is held until then.
    private var pageLoaded = false
    private var pendingScript: String? = null

    init {
        val toolbar = ActionManager.getInstance().createActionToolbar("ForgePreview", actions(), true)
        toolbar.targetComponent = this
        val header = JPanel(BorderLayout()).apply {
            // The picker takes the slack; the toolbar keeps its full width so zoom stays visible.
            add(viewPicker, BorderLayout.CENTER)
            add(toolbar.component, BorderLayout.EAST)
            border = JBUI.Borders.emptyLeft(4)
            add(errorBanner, BorderLayout.SOUTH)
        }
        add(header, BorderLayout.NORTH)

        if (browser == null) {
            add(JBLabel("The Forge preview needs JCEF, which this IDE runtime does not provide.", SwingConstants.CENTER))
        } else {
            Disposer.register(this, browser)
            browser.jbCefClient.addLoadHandler(
                object : CefLoadHandlerAdapter() {
                    override fun onLoadEnd(cefBrowser: CefBrowser, frame: CefFrame, httpStatusCode: Int) {
                        if (!frame.isMain) return
                        UIUtil.invokeLaterIfNeeded {
                            pageLoaded = true
                            pendingScript?.let(::execute)
                            pendingScript = null
                        }
                    }
                },
                browser.cefBrowser,
            )
            val (background, foreground) = themeColors()
            browser.loadHTML(PreviewHtml.page(background, foreground))
            add(browser.component, BorderLayout.CENTER)

            ApplicationManager.getApplication().messageBus.connect(this)
                .subscribe(LafManagerListener.TOPIC, LafManagerListener { onThemeChanged() })
        }
        display(PreviewHtml.messageScript("Rendering…"))
    }

    /** Must be called on the EDT. */
    fun show(result: RenderResult) {
        when (result) {
            is RenderResult.Failed -> {
                errorBanner.text = "<html>${escape(result.message).replace("\n", "<br>")}</html>"
                errorBanner.isVisible = true
                if (views.size == 0) display(PreviewHtml.messageScript("No diagram to show"))
            }
            is RenderResult.Rendered -> {
                errorBanner.isVisible = false
                val selectedKey = (views.selectedItem as? ForgeView)?.key
                replacingViews = true
                try {
                    views.removeAllElements()
                    views.addAll(result.views)
                    views.selectedItem =
                        result.views.firstOrNull { it.key == selectedKey } ?: result.views.firstOrNull()
                } finally {
                    replacingViews = false
                }
                viewPicker.isEnabled = result.views.isNotEmpty()
                if (result.views.isEmpty()) {
                    display(PreviewHtml.messageScript("This model defines no views"))
                } else {
                    showSelected()
                }
            }
        }
        revalidate()
    }

    private fun showSelected() {
        val view = views.selectedItem as? ForgeView ?: return
        display(PreviewHtml.showScript(PreviewHtml.applyTheme(view.svg, dark = !JBColor.isBright())))
    }

    private fun onThemeChanged() {
        val (background, foreground) = themeColors()
        execute(PreviewHtml.themeScript(background, foreground))
        showSelected()
    }

    /** Replaces what the page shows. Held until the page has loaded if necessary. */
    private fun display(script: String) {
        if (pageLoaded) execute(script) else pendingScript = script
    }

    /** Runs a script against the loaded page; before that there is nothing to zoom or restyle. */
    private fun execute(script: String) {
        val browser = browser ?: return
        if (!pageLoaded) return
        browser.cefBrowser.executeJavaScript(script, browser.cefBrowser.url, 0)
    }

    private fun actions() = DefaultActionGroup(
        action("Refresh", AllIcons.Actions.Refresh) { onRefresh() },
        action("Zoom In", AllIcons.General.ZoomIn) { execute(PreviewHtml.zoomScript(PreviewHtml.Zoom.IN)) },
        action("Zoom Out", AllIcons.General.ZoomOut) { execute(PreviewHtml.zoomScript(PreviewHtml.Zoom.OUT)) },
        action("Actual Size", AllIcons.General.ActualZoom) { execute(PreviewHtml.zoomScript(PreviewHtml.Zoom.ACTUAL)) },
        action("Fit Width", AllIcons.General.FitContent) { execute(PreviewHtml.zoomScript(PreviewHtml.Zoom.FIT)) },
    )

    private fun action(text: String, icon: Icon, perform: () -> Unit) =
        object : DumbAwareAction(text, null, icon) {
            override fun actionPerformed(e: AnActionEvent) = perform()
        }

    private fun themeColors(): Pair<String, String> {
        val scheme = EditorColorsManager.getInstance().globalScheme
        return "#" + ColorUtil.toHex(scheme.defaultBackground) to "#" + ColorUtil.toHex(scheme.defaultForeground)
    }

    private fun escape(text: String) = text.replace("&", "&amp;").replace("<", "&lt;").replace(">", "&gt;")

    override fun dispose() = Unit
}
