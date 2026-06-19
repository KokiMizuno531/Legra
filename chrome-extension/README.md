# Legra Chrome Extension

This unpacked Chrome extension sends the current paper page to Legra through Chrome Native Messaging.

It supports arXiv pages, publisher pages with DOI metadata, and PDF tabs that the user can legitimately access in Chrome. It does not bypass subscriptions, authentication, or publisher restrictions.

## Build the Native Host

From the repository root:

```sh
cd src-tauri
cargo build --bin paper_manager_native_host
```

The development binary is usually written to:

```text
src-tauri/target/debug/paper_manager_native_host
```

## Load the Extension

1. Open `chrome://extensions`.
2. Enable Developer mode.
3. Click `Load unpacked`.
4. Select the `chrome-extension/` directory.
5. Copy the generated extension ID.

## Install the Native Messaging Manifest

The easiest setup is from Legra:

1. Start Legra.
2. Open Settings.
3. Paste the Chrome extension ID into `Chrome extension ID`.
4. Click `Install Native Host`.

This writes the Native Messaging manifest into Chrome's user-specific configuration directory.

Manual setup is also possible:

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

## Use

1. Start Legra.
2. Open an arXiv page, publisher paper page, or PDF tab in Chrome.
3. Click the Legra extension icon.
4. Optionally choose an existing category or type a new category.
5. Click `Import to Legra`, or press Enter in the category field.

Legra polls the import inbox automatically. You can also click `More -> Import inbox` in the main app window.

## Troubleshooting

- Reload the extension after changing `service_worker.js`, `popup.js`, or the manifest.
- Rebuild `paper_manager_native_host` after Rust changes.
- If existing categories do not appear, rebuild `paper_manager_native_host`, reload the extension, and open the popup again.
- Confirm the Native Messaging manifest path is absolute.
- Confirm `allowed_origins` matches the Chrome extension ID.
- If PDF download is unavailable, Legra should still queue a metadata-only import.
