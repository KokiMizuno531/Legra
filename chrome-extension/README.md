# Legra Chrome Extension

This unpacked Chrome extension sends the current paper page to Legra through Chrome Native Messaging.

It supports arXiv pages, publisher pages with DOI metadata, and PDF tabs that the user can legitimately access in Chrome. It does not bypass subscriptions, authentication, or publisher restrictions.

## Install Legra

Install and open Legra first. On macOS:

```sh
brew tap KokiMizuno531/legra
brew install --cask legra
open -a Legra
```

The Homebrew cask includes the Chrome Native Messaging host used by this extension.

On Windows or Linux, install the matching Legra package from GitHub Releases. Every packaged build includes the Native Messaging host.

## Load the Extension

1. Open `chrome://extensions`.
2. Enable Developer mode.
3. Click `Load unpacked`.
4. Select the `chrome-extension/` directory.
5. Copy the generated extension ID.

## Install the Native Messaging Manifest

Install the Native Messaging manifest from Legra:

1. Start Legra.
2. Open `Settings`.
3. Paste the Chrome extension ID into `Chrome extension ID`.
4. Click `Install Native Host`.
5. Return to `chrome://extensions` and reload the Legra extension.

This copies the Native Host to Legra's user data directory and registers it for Google Chrome and Chromium. Windows uses the current user's registry; macOS and Linux use each browser's user-specific manifest directory.

After updating Legra, start the updated app and click `Install Native Host` again if Chrome still shows old import or category behavior. Reloading the unpacked extension only refreshes extension files; it does not replace the Native Host binary or the manifest path Chrome uses.

## Development Setup

If you are developing Legra from this repository instead of using Homebrew, build the native host first:

```sh
cd src-tauri
cargo build --bin paper_manager_native_host
```

The development binary is usually written to:

```text
src-tauri/target/debug/paper_manager_native_host
```

Manual manifest setup is also possible:

Copy the template:

```sh
cp chrome-extension/native-messaging/app.legra.importer.example.json \
  chrome-extension/native-messaging/app.legra.importer.json
```

Edit `chrome-extension/native-messaging/app.legra.importer.json`:

- Replace `REPLACE_WITH_EXTENSION_ID` with the extension ID from Chrome.
- Replace `/ABSOLUTE/PATH/TO/paper_manager_native_host` with the absolute path to the native host binary.

Install the manifest for Chrome on macOS:

```sh
mkdir -p "$HOME/Library/Application Support/Google/Chrome/NativeMessagingHosts"
cp chrome-extension/native-messaging/app.legra.importer.json \
  "$HOME/Library/Application Support/Google/Chrome/NativeMessagingHosts/app.legra.importer.json"
```

The local `app.legra.importer.json` file is ignored by git because it contains machine-specific paths and extension IDs.

For Windows and Linux development, prefer the `Install Native Host` button so Legra performs the required registry or manifest setup correctly.

## Use

1. Start Legra.
2. Open an arXiv page, publisher paper page, or PDF tab in Chrome.
3. Click the Legra extension icon.
4. Optionally choose an existing category or type a new category.
5. Click `Import to Legra`, or press Enter in the category field.

Legra polls the import inbox automatically. You can also click `More -> Import inbox` in the main app window.

## Troubleshooting

- Reload the extension after changing `service_worker.js`, `popup.js`, or the extension manifest.
- If Legra was installed through Homebrew, open Settings and click `Install Native Host` again after changing the Chrome extension ID.
- If Legra was updated through Homebrew, open the updated Legra app and click `Install Native Host` again so Chrome launches the current Native Host.
- If Legra is running from the development repository, rebuild `paper_manager_native_host` after Rust changes.
- If existing categories do not appear, rebuild or reinstall `paper_manager_native_host`, reload the extension, and open the popup again. The popup reports whether the Native Host returned zero categories or appears too old to provide diagnostics.
- Confirm the Native Messaging host path exists. On macOS and Linux it must be absolute.
- Confirm `allowed_origins` matches the Chrome extension ID.
- If PDF download is unavailable, Legra should still queue a metadata-only import.
