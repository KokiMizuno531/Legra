cask "legra" do
  version "0.3.1"
  sha256 "7fcb3b98204fdaa2392c3aa2723eacf0af35e2359515f85ec7a8e17982b70675"

  url "https://github.com/KokiMizuno531/Legra/releases/download/v#{version}/Legra_#{version}_aarch64.app.zip"
  name "Legra"
  desc "Local-first paper, PDF, note, and BibTeX manager"
  homepage "https://github.com/KokiMizuno531/Legra"

  depends_on :macos

  app "Legra.app"

  postflight_steps do
    run "/usr/bin/xattr",
        args: ["-dr", "com.apple.quarantine", "{{appdir}}/Legra.app"]
  end

  zap trash: [
    "~/Library/Application Support/Google/Chrome/NativeMessagingHosts/app.legra.importer.json",
    "~/Library/Application Support/Legra",
  ]
end
