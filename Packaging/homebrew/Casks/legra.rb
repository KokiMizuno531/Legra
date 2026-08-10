cask "legra" do
  version "0.2.5"
  sha256 "c1b1dc171f29d8e5b55f21b5472bfa35ab299dd68c1c78a1a392353f74cb48b5"

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
