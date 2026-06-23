cask "legra" do
  version "0.2.0"
  sha256 "84f2696acecb41f0ebd1518ba76715d177725c6b188ebb05d1a64b0b5770f2ab"

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
    "~/Library/Application Support/Google/Chrome/NativeMessagingHosts/app.legra.importer.json",
    "~/Library/Application Support/Legra",
  ]
end
