# Contributing to Legra

Thanks for your interest in improving Legra.

## Development Setup

Install dependencies:

```sh
npm install
```

Run the desktop app:

```sh
npm run tauri dev
```

Run checks before submitting changes:

```sh
npm run build
cd src-tauri
cargo check
cargo check --bin paper_manager_native_host
```

For Chrome extension changes:

```sh
node --check chrome-extension/service_worker.js
node --check chrome-extension/popup.js
```

## Pull Requests

- Keep changes focused.
- Prefer existing React, Rust, and Tauri patterns in the repository.
- Do not commit generated build output, local app data, or personal Native Messaging manifests.
- Include a short description of user-visible behavior changes.
- Include verification commands you ran.

## Project Direction

Legra is local-first and file-transparent. Features that keep PDFs, notes, and metadata inspectable as normal files fit the project best.

The Chrome extension must not bypass publisher authentication, subscriptions, paywalls, or access controls.
