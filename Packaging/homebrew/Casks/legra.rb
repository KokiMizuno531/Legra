cask "legra" do
  version "0.1.0"
  sha256 "REPLACE_WITH_RELEASE_SHA256"

  url "https://github.com/OWNER/legra/releases/download/v#{version}/Legra_#{version}_aarch64.app.zip"
  name "Legra"
  desc "Local-first paper, PDF, note, and BibTeX manager"
  homepage "https://github.com/OWNER/legra"

  depends_on macos: ">= :big_sur"

  app "Legra.app"

  zap trash: [
    "~/Library/Application Support/Legra",
    "~/Library/Application Support/Google/Chrome/NativeMessagingHosts/app.legra.importer.json",
  ]
end
