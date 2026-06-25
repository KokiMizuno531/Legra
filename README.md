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

The Chrome extension only works with pages and PDFs that the user can legitimately access in their browser.

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
```

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
