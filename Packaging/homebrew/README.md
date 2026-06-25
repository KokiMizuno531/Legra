# Homebrew Cask

This directory contains the starter cask for a personal Homebrew tap.

Recommended first release flow:

1. Create a GitHub release tag, for example `v0.1.0`.
2. Let the `Release` workflow build the macOS artifact.
3. Download `SHA256SUMS.txt` from the draft release.
4. Copy `Packaging/homebrew/Casks/legra.rb` into a tap repository such as `homebrew-legra`.
5. Replace `OWNER`, asset filename, `version`, and `sha256`.
6. Test locally:

```sh
brew tap OWNER/legra
brew install --cask legra
brew uninstall --cask legra
```

For Chrome import, start Legra after installation or upgrade, open Settings, paste the Chrome extension ID, and click `Install Native Host`. Reloading the Chrome extension does not refresh the Native Host binary.
