# Forge LSP — Editor Setup

The Forge language server provides IDE features for `.forge` files:

- **Diagnostics** — Parse errors and architectural lint warnings as you type
- **Hover** — Element details (kind, description, technology, tags) on mouseover
- **Go to Definition** — Jump to element declarations
- **Completion** — DSL keywords and element identifiers
- **Document Symbols** — File outline / breadcrumbs

## Prerequisites

Build the forge binary:

```bash
cd forge
cargo build --release
```

The LSP server is started with:

```bash
forge lsp
```

It communicates via stdio (stdin/stdout) using the Language Server Protocol.

---

## VS Code

### Option 1: Generic LSP extension

Install the [vscode-languageclient](https://marketplace.visualstudio.com/items?itemName=matklad.vscode-generic-lsp) or configure via a simple extension.

Create `.vscode/settings.json` in your workspace:

```json
{
  "files.associations": {
    "*.forge": "forge"
  }
}
```

Then install a generic LSP client extension like **[LSP Client](https://marketplace.visualstudio.com/items?itemName=AlanWalk.lsp-client)** or create a minimal extension with this `package.json`:

```json
{
  "name": "forge-lsp",
  "displayName": "Forge Language Support",
  "version": "0.1.0",
  "engines": { "vscode": "^1.75.0" },
  "activationEvents": ["onLanguage:forge"],
  "contributes": {
    "languages": [{
      "id": "forge",
      "extensions": [".forge"],
      "configuration": "./language-configuration.json"
    }]
  },
  "main": "./out/extension.js"
}
```

Extension `src/extension.ts`:

```typescript
import * as vscode from 'vscode';
import { LanguageClient, TransportKind } from 'vscode-languageclient/node';

let client: LanguageClient;

export function activate(context: vscode.ExtensionContext) {
  const serverOptions = {
    command: 'forge',
    args: ['lsp'],
    transport: TransportKind.stdio,
  };
  const clientOptions = {
    documentSelector: [{ scheme: 'file', language: 'forge' }],
  };
  client = new LanguageClient('forge', 'Forge LSP', serverOptions, clientOptions);
  client.start();
}

export function deactivate() {
  return client?.stop();
}
```

---

## Neovim

### Using nvim-lspconfig (recommended)

Add to your `init.lua`:

```lua
-- Register the forge filetype
vim.filetype.add({
  extension = {
    forge = 'forge',
  },
})

-- Configure the LSP
local lspconfig = require('lspconfig')
local configs = require('lspconfig.configs')

if not configs.forge then
  configs.forge = {
    default_config = {
      cmd = { 'forge', 'lsp' },
      filetypes = { 'forge' },
      root_dir = lspconfig.util.root_pattern('*.forge', '.git'),
      settings = {},
    },
  }
end

lspconfig.forge.setup({})
```

### Using native vim.lsp (no plugins)

```lua
vim.filetype.add({ extension = { forge = 'forge' } })

vim.api.nvim_create_autocmd('FileType', {
  pattern = 'forge',
  callback = function()
    vim.lsp.start({
      name = 'forge',
      cmd = { 'forge', 'lsp' },
      root_dir = vim.fs.dirname(vim.fs.find({ '.git' }, { upward = true })[1]),
    })
  end,
})
```

---

## Helix

Add to `~/.config/helix/languages.toml`:

```toml
[[language]]
name = "forge"
scope = "source.forge"
file-types = ["forge"]
roots = [".git"]
language-servers = ["forge-lsp"]
comment-token = "//"

[language-server.forge-lsp]
command = "forge"
args = ["lsp"]
```

---

## Zed

Add to your Zed settings (`~/.config/zed/settings.json`):

```json
{
  "lsp": {
    "forge": {
      "binary": {
        "path": "forge",
        "arguments": ["lsp"]
      }
    }
  },
  "file_types": {
    "forge": ["forge"]
  },
  "languages": {
    "forge": {
      "language_servers": ["forge"]
    }
  }
}
```

---

## Sublime Text

Install the [LSP](https://packagecontrol.io/packages/LSP) package, then add to LSP settings (`Preferences > Package Settings > LSP > Settings`):

```json
{
  "clients": {
    "forge": {
      "enabled": true,
      "command": ["forge", "lsp"],
      "selector": "source.forge",
      "schemes": ["file"]
    }
  }
}
```

Create a syntax file at `Packages/User/Forge.sublime-syntax`:

```yaml
%YAML 1.2
---
name: Forge
scope: source.forge
file_extensions: [forge]
contexts:
  main:
    - match: '//'
      scope: comment.line.forge
      push: line_comment
    - match: '"'
      scope: punctuation.definition.string.begin.forge
      push: string
  line_comment:
    - meta_scope: comment.line.forge
    - match: '$'
      pop: true
  string:
    - meta_scope: string.quoted.double.forge
    - match: '"'
      scope: punctuation.definition.string.end.forge
      pop: true
```

---

## JetBrains IDEs (IntelliJ, WebStorm, GoLand, etc.)

Install the [LSP4IJ](https://plugins.jetbrains.com/plugin/23257-lsp4ij) plugin, then:

1. Go to **Settings > Languages & Frameworks > Language Server Protocol**
2. Click **+** to add a new server
3. Configure:
   - **Name:** Forge
   - **Command:** `forge lsp`
   - **File patterns:** `*.forge`

Alternatively, with the older [LSP Support](https://plugins.jetbrains.com/plugin/10209-lsp-support) plugin:

1. Go to **Settings > Languages & Frameworks > Language Server Protocol > Server Definitions**
2. Select **Executable**
3. Set:
   - **Extension:** `forge`
   - **Path:** path to `forge` binary
   - **Args:** `lsp`

---

## Emacs

Using `lsp-mode`:

```elisp
(define-derived-mode forge-mode prog-mode "Forge"
  "Major mode for Forge DSL files."
  (setq-local comment-start "// ")
  (setq-local comment-end ""))

(add-to-list 'auto-mode-alist '("\\.forge\\'" . forge-mode))

(with-eval-after-load 'lsp-mode
  (add-to-list 'lsp-language-id-configuration '(forge-mode . "forge"))
  (lsp-register-client
   (make-lsp-client
    :new-connection (lsp-stdio-connection '("forge" "lsp"))
    :activation-fn (lsp-activate-on "forge")
    :server-id 'forge-lsp)))
```

Using `eglot` (built into Emacs 29+):

```elisp
(define-derived-mode forge-mode prog-mode "Forge"
  (setq-local comment-start "// "))

(add-to-list 'auto-mode-alist '("\\.forge\\'" . forge-mode))
(add-to-list 'eglot-server-programs '(forge-mode "forge" "lsp"))
```

---

## Verifying the setup

Open a `.forge` file and verify:

1. **Diagnostics**: Intentionally break the syntax — you should see red squiggly underlines
2. **Hover**: Move your cursor over an element name — you should see its description
3. **Completion**: Type inside a block and trigger completion (Ctrl+Space) — keywords and element names appear
4. **Go to Definition**: Ctrl+click or F12 on an element reference — jumps to its definition
5. **Symbols**: Open the symbol outline (Ctrl+Shift+O in VS Code) — see elements and views listed

## Troubleshooting

- **LSP not starting**: Ensure `forge` is on your PATH or use an absolute path
- **No diagnostics**: Check that the file has a `.forge` extension
- **Logs**: Most editors show LSP logs in an output panel (e.g., VS Code: Output > Forge LSP)
