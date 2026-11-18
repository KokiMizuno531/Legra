cask "legra" do
  version "0.3.3"
  sha256 "c394783e840640c8ed5a220fd96ba29140a076dc9f9c6c8b5829b43862495f86"

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
