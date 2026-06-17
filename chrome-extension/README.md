# paper-manager Chrome extension

This unpacked Chrome extension sends the current paper page to paper-manager through Chrome Native Messaging.

## Setup

1. Build the native host:

```bash
cd ../src-tauri
cargo build --bin paper_manager_native_host
```

2. Load this directory from `chrome://extensions` with Developer mode enabled.

3. Copy the displayed extension ID.

4. Edit `native-messaging/com.mizuno.paper_manager.json`:

- Replace `REPLACE_WITH_EXTENSION_ID` with the extension ID.
- Replace `/ABSOLUTE/PATH/TO/paper_manager_native_host` with the absolute path to `src-tauri/target/debug/paper_manager_native_host`.

5. Install the Native Messaging manifest:

```bash
mkdir -p "$HOME/Library/Application Support/Google/Chrome/NativeMessagingHosts"
cp native-messaging/com.mizuno.paper_manager.json \
  "$HOME/Library/Application Support/Google/Chrome/NativeMessagingHosts/com.mizuno.paper_manager.json"
```

## Use

Open an arXiv page, a publisher paper page, or a PDF tab. Click the extension icon, optionally enter a folder category, then click `Import to paper-manager`.

The extension writes an import request through the native host. Open paper-manager and click `Import inbox` if it has not imported automatically.
