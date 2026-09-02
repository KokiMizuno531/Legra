# Legra

Legra is a local-first desktop app for managing research papers, PDFs, Markdown notes, and BibTeX exports.

It is built with Tauri, React, and Rust. The app is designed for researchers who want their paper library to stay as ordinary files on disk instead of being locked inside a cloud service.

## Features

- Register papers from DOI, arXiv ID, paper URLs, or manual metadata.
- Organize PDFs into a managed directory with configurable filename rules.
- Keep linked Markdown notes and open them in an external editor such as MarkText.
- Export selected papers as BibTeX.
- Configure BibTeX citation key rules.
- Normalize journal names at export time using editable journal aliases.
- Import metadata and accessible PDFs from Chrome via a companion extension.
- Queue DOI and arXiv papers found by Codex App or Claude Desktop through a local MCP connector.
- Create shared workspaces in Google Drive, Dropbox, iCloud Drive, or any synced folder.
- Keep PDFs and notes as regular files that can be backed up or inspected without Legra.

## Philosophy

Legra is not a Zotero or Mendeley replacement. It focuses on a smaller workflow:

- local files first
- transparent folder structure
- Markdown notes
- lightweight BibTeX export
- no account requirement
- no publisher authentication bypass

The Chrome extension only works with pages and PDFs that the user can legitimately access in their
browser. The MCP connector accepts DOI and arXiv identifiers only. It makes a best-effort attempt to
retrieve open-access PDFs and otherwise keeps a metadata-only record; it does not accept arbitrary
download URLs, reuse browser sessions, bypass paywalls, or circumvent publisher authentication.

## Current Limitations

- Windows and macOS builds are unsigned. Windows may show a SmartScreen warning.
- PDF annotation is not built in.
- Word, LibreOffice, and Google Docs citation plugins are not included.
- Shared workspaces use regular file sync. They detect conflicting saves but do not merge simultaneous edits.
- Chrome Native Messaging setup is manual during development.

## Requirements

- macOS 11 or later on Apple Silicon
- Windows 10 or 11 x64 with Microsoft Edge WebView2
- Linux x64 with WebKitGTK 4.1; Ubuntu 22.04 or Debian 12 are the supported baselines

Development also requires:

- Node.js and npm
- Rust toolchain
- Tauri prerequisites for the development operating system
- Chrome or a Chromium-based browser for the optional extension

## Install

On macOS, install Legra with Homebrew:

```sh
brew tap KokiMizuno531/legra
brew install --cask legra
```

Upgrade an existing installation:

```sh
brew update
brew upgrade --cask legra
```

Open Legra:

```sh
open -a Legra
```

On Windows, download `Legra_<version>_windows_x86_64-setup.exe` from GitHub Releases and run the installer. The installer is currently unsigned.

On Linux, download either the AppImage or Debian package from GitHub Releases:

```sh
chmod +x Legra_<version>_linux_x86_64.AppImage
./Legra_<version>_linux_x86_64.AppImage

# Debian/Ubuntu alternative
sudo apt install ./Legra_<version>_linux_amd64.deb
```

The Chrome extension is not distributed through Homebrew yet. To use Chrome import, load `chrome-extension/` as an unpacked Chrome extension, copy its extension ID, paste it into `Settings -> Chrome extension ID`, and click `Install Native Host`.

After updating Legra through Homebrew, start Legra and click `Settings -> Install Native Host` again if Chrome import or category loading still appears stale. Reloading the unpacked Chrome extension only reloads extension JavaScript; it does not update the Native Host binary or manifest that Chrome launches.

## Development

Install dependencies:

```sh
npm install
```

Run the app in development mode:

```sh
npm run tauri dev
```

Basic workflow:

1. Open Legra and set a managed directory from the storage controls.
2. Register a paper from a DOI, arXiv ID, or paper URL.
3. Choose a category to organize the PDF into folders.
4. Link or create Markdown notes for the paper.
5. Export selected papers as BibTeX when needed.

### Managed directory sync

Each managed directory is an independent Legra library. Legra stores shared catalog data under
`.legra/library.json` and scans the directory when the app starts, when `Reload / Sync` is used,
and immediately after `Set directory`.

- PDFs added outside Legra are discovered recursively.
- DOI and arXiv metadata is resolved from text in the first three pages when possible.
- PDFs that disappear remain in the catalog with a `Missing` status.
- Moving or renaming a PDF preserves its registration through a content fingerprint.
- Paper metadata and Legra-managed notes use relative paths so the same Drive folder works on
  another device.

Image-only PDFs are not OCRed. They are added with the filename as a temporary title and marked
`Metadata incomplete` for manual review.

Build the frontend:

```sh
npm run build
```

Check the Rust backend:

```sh
cd src-tauri
cargo check
cargo check --bin paper_manager_native_host
cargo check --bin legra_mcp
```

## Codex App and Claude Desktop

Legra ships a local MCP server named `legra_mcp`. It lets Codex App and Claude Desktop check the
library, list categories, normalize a DOI or arXiv identifier and check for an existing record,
queue an import, and check the queued job's result. The MCP process never writes Legra's catalog or
managed PDF directory directly. It places a versioned job in Legra's import queue; the Legra app
validates and performs the import.

### Codex App

1. Open Legra and go to `Settings -> Codex / Claude connector`.
2. Click `Install connector`, then `Check connector`.
3. Click `Copy TOML` and merge the copied table into `~/.codex/config.toml` without replacing
   unrelated settings.
4. Restart Codex App, open a new task, and use `/mcp` to confirm that `legra` is connected.

Legra also shows a ready-to-copy TOML snippet. Its shape is:

```toml
[mcp_servers.legra]
command = "/absolute/path/to/legra_mcp"
env = { LEGRA_SETTING_DIR = "/absolute/path/to/Legra/data" }
default_tools_approval_mode = "writes"
```

Legra installs or updates only its own connector binary. It does not modify Codex configuration.
Codex App's `Settings -> MCP servers -> Add server` STDIO form is also available for a standard data
directory, but the generated TOML is recommended because it preserves `LEGRA_SETTING_DIR` and the
write approval policy. Codex desktop, CLI, and the IDE extension share this configuration on the
same host.

For complete installation, upgrade, verification, prompt examples, and troubleshooting steps, see
[Import papers from Codex App or Claude Desktop](docs/codex-claude-paper-import.md).

### Claude Desktop

For a packaged release on macOS or Windows, download the matching file from GitHub Releases and
open it with Claude Desktop:

- `Legra_<version>_claude_desktop_macos_arm64.mcpb`
- `Legra_<version>_claude_desktop_windows_x86_64.mcpb`

The MCPB is the recommended one-click installation route. Linux is not currently an official Claude
Desktop target, so Legra does not publish a Linux MCPB.

For an unpackaged development build, use the manual fallback: install the connector from Legra
Settings, open Claude Desktop's `Settings -> Developer -> Edit Config`, and merge the JSON snippet
shown by Legra into the existing configuration. Do not replace unrelated MCP server entries. The
snippet has this shape:

```json
{
  "mcpServers": {
    "legra": {
      "command": "/absolute/path/to/legra_mcp",
      "args": [],
      "env": {
        "LEGRA_SETTING_DIR": "/absolute/path/to/Legra/data"
      }
    }
  }
}
```

Restart Claude Desktop after installing the MCPB or editing its developer configuration.

### Queue and download behavior

When Legra is running in its main window, it processes agent jobs at startup and every 15 seconds.
When Legra is closed, Codex or Claude can still enqueue an import; the job remains pending and is
processed the next time Legra starts. Consequently, an MCP `import_paper` result confirms that the
job was queued, not necessarily that a PDF has already been downloaded. Use `get_import_status` to
distinguish `pending`, `processing`, `done`, and `failed` jobs.

Agent imports accept normalized DOI or arXiv identifiers only. PDF retrieval is unauthenticated and
best-effort: arXiv or a direct PDF link advertised by Crossref may yield a PDF, while inaccessible
DOI records are registered as metadata-only. `open_access_only` means Legra will not use a logged-in
session or bypass a paywall; it is not a legal determination of the document's reuse license.

## Chrome Extension

The optional Chrome extension lives in `chrome-extension/`.

It can detect DOI/arXiv metadata on the current page, download accessible PDFs when possible, and queue an import request through Chrome Native Messaging.

See [chrome-extension/README.md](chrome-extension/README.md) for setup instructions.

During development, load the extension as an unpacked extension and paste its extension ID into `Settings -> Chrome extension ID`. Then click `Install Native Host` in Settings. Legra writes the Chrome Native Messaging manifest to the user-level Chrome configuration directory.

After changing the native host code, rebuild it before testing Chrome import:

```sh
cd src-tauri
cargo build --bin paper_manager_native_host
```

For packaged releases, the same Settings action is intended to install or refresh the Native Messaging manifest after Legra is installed. If Legra was updated through Homebrew or another installer, launch the updated app before reinstalling the Native Host so the copied host binary matches the installed Legra version.

## Shared Workspaces

Legra can create a shared workspace in a synced folder such as Google Drive, Dropbox, or iCloud Drive.

The workspace contains:

```text
paper-manager-workspace.json
papers/
notes/
exports/
.paper-manager/
```

The `paper-manager` names are kept for compatibility with earlier development versions. They may be migrated in a future release.

Shared workspaces are intentionally lightweight. If another machine changes the workspace before your save, Legra stops the write and asks you to reload instead of silently overwriting collaborator changes.

## Releases and Homebrew

Tagged releases run the GitHub Actions release workflow. A release is published only after the macOS ARM64 app, Windows x64 NSIS installer, Linux x64 AppImage, and Linux amd64 Debian package all build successfully. `SHA256SUMS.txt` covers every release asset.

The public Homebrew tap is:

```sh
brew tap KokiMizuno531/legra
brew install --cask legra
```

A copy of the cask is also kept in [Packaging/homebrew](Packaging/homebrew) for reference.

Upgrade an installed cask with:

```sh
brew update
brew upgrade --cask legra
open -a Legra
```

Launching the upgraded app refreshes an already-installed MCP connector when its version changes.
Install the connector from Settings if it has never been installed, then restart Codex so it starts
the updated MCP binary.

After installing on any supported operating system, open Legra, set the Chrome extension ID in Settings, and click `Install Native Host` to enable Chrome or Chromium import. If Chrome still reports old category behavior after an app update, reinstall the Native Host from Settings; reloading the Chrome extension alone does not refresh the Native Host binary.

## Repository Safety

Do not commit:

- generated `dist/`
- `node_modules/`
- `src-tauri/target/`
- local app data under `setting/`
- personal Chrome Native Messaging manifests

Use `chrome-extension/native-messaging/app.legra.importer.example.json` as the template for local setup.

## Verification

Before opening a pull request, run:

```sh
npm run build
cd src-tauri
cargo test
```

For Chrome extension changes:

```sh
node --check chrome-extension/service_worker.js
node --check chrome-extension/popup.js
```

## License

MIT
