cask "legra" do
  version "0.1.0"
  sha256 "3818f7e1b50210f64c8b84db4c034722b5e6d3bd2aad45774a684417cc095c91"

  url "https://github.com/KokiMizuno531/Legra/releases/download/v#{version}/Legra_#{version}_aarch64.app.zip"
  name "Legra"
  desc "Local-first paper, PDF, note, and BibTeX manager"
  homepage "https://github.com/KokiMizuno531/Legra"

  depends_on macos: ">= :big_sur"

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
