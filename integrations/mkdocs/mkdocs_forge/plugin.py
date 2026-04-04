"""MkDocs Forge plugin — renders .forge views as inline SVGs in documentation.

Usage in mkdocs.yml:
    plugins:
      - forge:
          source: architecture.forge
          style: filled  # or outline

Usage in markdown:
    {{ forge_view "SystemContext" }}
    {{ forge_view "Containers" }}
"""

import os
import re
import subprocess
import tempfile
import logging
from pathlib import Path

from mkdocs.plugins import BasePlugin
from mkdocs.config import config_options

log = logging.getLogger("mkdocs.plugins.forge")


class ForgePlugin(BasePlugin):
    config_scheme = (
        ("source", config_options.Type(str, default="forge.forge")),
        ("style", config_options.Type(str, default="filled")),
        ("forge_bin", config_options.Type(str, default="forge")),
    )

    def __init__(self):
        super().__init__()
        self._svgs = {}
        self._source_path = None

    def on_config(self, config):
        """Build all SVGs once at startup."""
        docs_dir = config.get("docs_dir", ".")
        source = os.path.join(docs_dir, "..", self.config["source"])
        if not os.path.isabs(source):
            source = os.path.abspath(source)

        self._source_path = source
        if not os.path.exists(source):
            log.warning(f"Forge source not found: {source}")
            return config

        self._build_svgs()
        return config

    def on_page_markdown(self, markdown, page, config, files):
        """Replace {{ forge_view "Key" }} with inline SVGs."""
        pattern = r'\{\{\s*forge_view\s+"([^"]+)"\s*\}\}'

        def replace_view(match):
            view_key = match.group(1)
            svg = self._svgs.get(view_key)
            if svg:
                return f'<div class="forge-diagram-wrap">\n{svg}\n</div>'
            else:
                log.warning(f"Forge view '{view_key}' not found")
                return f'<!-- forge_view "{view_key}" not found -->'

        return re.sub(pattern, replace_view, markdown)

    def on_serve(self, server, config, builder):
        """Watch .forge files for changes during mkdocs serve."""
        if self._source_path and os.path.exists(self._source_path):
            source_dir = os.path.dirname(self._source_path)
            server.watch(source_dir, builder)
        return server

    def _build_svgs(self):
        """Run forge build and load the resulting SVGs."""
        self._svgs = {}
        source = self._source_path
        if not source or not os.path.exists(source):
            return

        with tempfile.TemporaryDirectory() as tmpdir:
            try:
                result = subprocess.run(
                    [
                        self.config["forge_bin"],
                        "build",
                        "--source", source,
                        "--out", tmpdir,
                        "--style", self.config["style"],
                    ],
                    capture_output=True,
                    text=True,
                    timeout=60,
                )
                if result.returncode != 0:
                    log.error(f"Forge build failed: {result.stderr}")
                    return

                # Load all generated SVGs
                for svg_file in Path(tmpdir).glob("*.svg"):
                    view_key = svg_file.stem
                    self._svgs[view_key] = svg_file.read_text()
                    log.info(f"Loaded forge view: {view_key}")

            except FileNotFoundError:
                log.error(
                    f"Forge binary not found: {self.config['forge_bin']}. "
                    "Install with: cargo install --path forge"
                )
            except subprocess.TimeoutExpired:
                log.error("Forge build timed out after 60 seconds")
