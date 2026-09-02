# Homebrew Cask maintenance

The public tap is [KokiMizuno531/homebrew-legra](https://github.com/KokiMizuno531/homebrew-legra).
This directory keeps the matching cask in the main repository as a release reference.

Release flow:

1. Bump every product version and push the release commit to `main`.
2. Wait for the main CI workflow to pass.
3. Push an annotated tag matching the product version, for example `v0.3.0`.
4. Wait for the `Release` workflow to publish all platform assets and `SHA256SUMS.txt`.
5. Read the checksum for `Legra_<version>_aarch64.app.zip` from the published checksums.
6. Update `version` and `sha256` in both `Packaging/homebrew/Casks/legra.rb` and the public tap's
   `Casks/legra.rb`.
7. Commit and push the main-repository reference cask and the public tap cask.
8. Test the published upgrade:

```sh
brew update
brew info --cask legra
brew upgrade --cask legra
open -a Legra
```

The GitHub Release alone does not update Homebrew users; the tap commit is required. Do not update
the cask before the matching asset exists. After an upgrade, opening Legra refreshes any previously
installed Chrome Native Host or MCP connector when its recorded version is stale. First-time users
must still install each connector from Settings.
