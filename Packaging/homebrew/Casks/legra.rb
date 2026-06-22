cask "legra" do
  version "0.1.3"
  sha256 "REPLACE_WITH_RELEASE_SHA256"

  url "https://github.com/KokiMizuno531/Legra/releases/download/v#{version}/Legra_#{version}_aarch64.app.zip"
  name "Legra"
  desc "Local-first paper, PDF, note, and BibTeX manager"
  homepage "https://github.com/KokiMizuno531/Legra"

  depends_on macos: :big_sur

  app "Legra.app"

  postflight do
    system_command "/usr/bin/xattr",
                   args: ["-dr", "com.apple.quarantine", "#{appdir}/Legra.app"],
                   sudo: false
  end

  zap trash: [
    "~/Library/Application Support/Legra",
    "~/Library/Application Support/Google/Chrome/NativeMessagingHosts/app.legra.importer.json",
  ]
end
