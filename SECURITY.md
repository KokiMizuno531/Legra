# Security Policy

## Reporting a Vulnerability

Please report security issues privately to the maintainer before public disclosure.

If a public issue is the only available contact path, avoid posting exploit details or private data. Provide a minimal description and ask for a private follow-up channel.

## Scope

Security-sensitive areas include:

- Chrome Native Messaging
- file path handling
- workspace import/export
- PDF and note file operations
- metadata fetching from DOI/arXiv sources

## Chrome Extension Policy

The Legra Chrome extension only imports metadata and PDFs that the user can legitimately access in their browser.

It must not:

- bypass paywalls
- bypass authentication
- scrape restricted publisher content without permission
- exfiltrate local files
- upload PDFs or notes to a remote service

## Local Data

Legra stores paper metadata, workspace files, PDFs, and notes locally or in user-selected synced folders. Users are responsible for the access permissions of shared folders such as Google Drive, Dropbox, or iCloud Drive.
