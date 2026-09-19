package com.grahambrooks.forge.preview

import com.google.gson.Gson

/**
 * The page hosted in the preview browser. It is loaded once; diagrams are swapped in through
 * [showScript] so that re-rendering while typing keeps the scroll position and zoom level.
 */
object PreviewHtml {
    private val gson = Gson()

    private val darkSchemeQuery = Regex("""@media\s*\(\s*prefers-color-scheme\s*:\s*dark\s*\)""")

    /**
     * Forge SVGs switch palette on `prefers-color-scheme`, which the embedded browser answers
     * from the OS. The preview should follow the IDE theme instead, so the query is rewritten
     * to match unconditionally (dark IDE) or never (light IDE).
     */
    fun applyTheme(svg: String, dark: Boolean): String =
        svg.replace(darkSchemeQuery, if (dark) "@media all" else "@media not all")

    fun showScript(svg: String): String = "forgeShow(${gson.toJson(svg)});"

    fun messageScript(message: String): String = "forgeMessage(${gson.toJson(message)});"

    fun zoomScript(zoom: Zoom): String = "forgeZoom(${gson.toJson(zoom.name)});"

    fun themeScript(background: String, foreground: String): String =
        "forgeTheme(${gson.toJson(background)}, ${gson.toJson(foreground)});"

    enum class Zoom { IN, OUT, FIT, ACTUAL }

    fun page(background: String, foreground: String): String = """
        <!DOCTYPE html>
        <html>
        <head>
        <meta charset="utf-8">
        <style>
          html, body { margin: 0; padding: 0; }
          body { background: $background; color: $foreground;
                 font-family: system-ui, -apple-system, 'Segoe UI', sans-serif; font-size: 13px; }
          #stage { padding: 12px; }
          #stage svg { display: block; margin: 0 auto; }
          #stage.fit svg { max-width: 100%; height: auto; }
          .message { opacity: 0.7; padding: 24px; text-align: center; }
        </style>
        </head>
        <body>
        <div id="stage" class="fit"></div>
        <script>
          const stage = document.getElementById('stage');
          let zoom = null; // null = fit to width

          function naturalSize(svg) {
            const w = parseFloat(svg.getAttribute('width'));
            const h = parseFloat(svg.getAttribute('height'));
            if (w && h) return [w, h];
            const vb = svg.viewBox && svg.viewBox.baseVal;
            return vb && vb.width ? [vb.width, vb.height] : [0, 0];
          }

          function applyZoom() {
            const svg = stage.querySelector('svg');
            stage.classList.toggle('fit', zoom === null);
            if (!svg) return;
            const [w, h] = naturalSize(svg);
            if (zoom === null || !w) {
              svg.style.width = svg.style.height = '';
            } else {
              svg.style.width = (w * zoom) + 'px';
              svg.style.height = (h * zoom) + 'px';
            }
          }

          function currentScale() {
            const svg = stage.querySelector('svg');
            if (!svg) return 1;
            const [w] = naturalSize(svg);
            return w ? svg.getBoundingClientRect().width / w : 1;
          }

          function forgeShow(svg) { stage.innerHTML = svg; applyZoom(); }

          function forgeMessage(text) {
            const div = document.createElement('div');
            div.className = 'message';
            div.textContent = text;
            stage.replaceChildren(div);
          }

          function forgeZoom(op) {
            if (op === 'FIT') zoom = null;
            else if (op === 'ACTUAL') zoom = 1;
            else {
              const base = zoom === null ? currentScale() : zoom;
              zoom = Math.min(8, Math.max(0.1, op === 'IN' ? base * 1.25 : base / 1.25));
            }
            applyZoom();
          }

          function forgeTheme(bg, fg) { document.body.style.background = bg; document.body.style.color = fg; }

          window.addEventListener('wheel', e => {
            if (!e.ctrlKey && !e.metaKey) return;
            e.preventDefault();
            forgeZoom(e.deltaY < 0 ? 'IN' : 'OUT');
          }, { passive: false });
        </script>
        </body>
        </html>
    """.trimIndent()
}
