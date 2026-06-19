cask "legra" do
  version "0.1.0"
  sha256 "fec90b57bc454f76872a68004f205750f948f48130d40276519845d8d0bf2e8d"

  url "https://github.com/KokiMizuno531/Legra/releases/download/v#{version}/Legra_#{version}_aarch64.app.zip"
  name "Legra"
  desc "Local-first paper, PDF, note, and BibTeX manager"
  homepage "https://github.com/KokiMizuno531/Legra"

  depends_on macos: ">= :big_sur"

  app "Legra.app"

  zap trash: [
    "~/Library/Application Support/Legra",
    "~/Library/Application Support/Google/Chrome/NativeMessagingHosts/app.legra.importer.json",
  ]
end
