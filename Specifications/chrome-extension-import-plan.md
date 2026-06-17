# Chrome Extension Import Plan

## Goal

Chromeで開いているarXivページ、論文ページ、PDFタブから、DOI/arXiv ID/PDFをpaper-managerへ渡し、paper-manager側で登録、リネーム、フォルダ分けを行う。

v1ではChrome Native Messagingを使う。非OA PDFは、ユーザーがChrome上で正当に閲覧またはダウンロードできるPDFだけを対象にする。購読回避、認証突破、出版社サイトの無断スクレイピングは行わない。

## Architecture

- Chrome extension: `chrome-extension/`
  - 現在タブからDOI、arXiv ID、PDF URL、タイトル、著者、年、出版情報を検出する。
  - PDF URLがある場合は `chrome.downloads.download` で `Downloads/paper-manager-import/` に保存する。
  - 保存完了後、絶対PDFパスとメタデータをNative Messagingで送る。
  - PDF保存に失敗した場合もエラーで止めず、警告付きでメタデータのみをNative Messagingで送る。
  - PDFではないHTML等が保存された場合、拡張側で即削除してDownloadsに残さない。
- Native Messaging host: `src-tauri/src/bin/paper_manager_native_host.rs`
  - Host名: `com.mizuno.paper_manager`
  - Chromeから受け取ったJSONを `setting/extension-imports/pending/*.json` に保存する。
  - app-data.jsonは直接編集しない。
- paper-manager app:
  - `process_extension_imports` commandでpending JSONを読む。
  - DOI/arXivから不足メタデータを補完する。
  - 既存の登録処理を使ってPDFをmanaged directory/categoryへ移動し、命名規則でリネームする。
  - `suggested_category` が未作成の場合は、managed directory配下に新規ディレクトリを作成して保存する。
  - 成功したJSONは `processed/`、失敗したJSONは `failed/` に移動する。

## Setup

1. Native hostをビルドする。

```bash
cd src-tauri
cargo build --bin paper_manager_native_host
```

2. `chrome-extension/native-messaging/com.mizuno.paper_manager.json` を編集する。

- `path`: `target/debug/paper_manager_native_host` の絶対パスにする。
- `allowed_origins`: Chromeで読み込んだ拡張のIDに置き換える。

3. manifestをChromeのNative Messaging Hostディレクトリへ配置する。

```bash
mkdir -p "$HOME/Library/Application Support/Google/Chrome/NativeMessagingHosts"
cp chrome-extension/native-messaging/com.mizuno.paper_manager.json \
  "$HOME/Library/Application Support/Google/Chrome/NativeMessagingHosts/com.mizuno.paper_manager.json"
```

4. Chromeで `chrome://extensions` を開く。

- Developer modeを有効にする。
- `Load unpacked` で `chrome-extension/` を選ぶ。
- 表示された拡張IDをNative Messaging manifestの `allowed_origins` に反映する。

5. paper-managerを起動し、managed directoryを設定する。

## User Flow

1. ChromeでarXiv、論文ページ、またはPDFタブを開く。
2. 拡張アイコンを押す。
3. 検出されたTitle/DOI/arXiv/PDFを確認する。
4. 必要ならFolder categoryを入力する。
5. `Import to paper-manager` を押す。
6. paper-managerを開く、またはメイン画面の `Import inbox` を押す。
7. 登録済みカードでPDFパス、カテゴリ、BibTeX、Open PDFを確認する。

## Data Shape

Native Messaging request:

```json
{
  "action": "import_paper",
  "request": {
    "id": "chrome-import-1710000000000",
    "source_url": "https://example.org/paper",
    "doi": "10.xxxx/example",
    "arxiv_id": "2401.00000",
    "title": "Paper title",
    "authors": ["First Author", "Second Author"],
    "year": 2026,
    "publication": "Journal",
    "pdf_path": "/Users/.../Downloads/paper-manager-import/file.pdf",
    "suggested_category": "physics/spin",
    "tags": ["chrome-import"],
    "import_warnings": ["PDF download failed: ..."]
  }
}
```

## Test Checklist

- [ ] arXiv abstractページからImportし、PDFがDownloadsへ保存される。
- [ ] arXiv import後、paper-managerの `Import inbox` で登録、リネーム、カテゴリ分けが完了する。
- [ ] Chrome拡張で指定した新規Folder categoryが存在しない場合、自動作成されてPDFが保存される。
- [ ] arXiv PDFタブからImportして同じように登録できる。
- [ ] DOIページでDOIを検出し、Crossref/arXiv metadata補完で登録できる。
- [ ] ログイン済み出版社ページで、ユーザーがアクセスできるPDF URLだけが保存される。
- [ ] PDF URLがないページではメタデータだけpendingに入り、metadata-onlyで登録できる。
- [ ] PDF URLがあるが保存に失敗した場合も、metadata-onlyで登録され、`pdf-missing` タグが付く。
- [ ] PDFではないHTMLが返った場合、`Downloads/paper-manager-import/` にHTMLが残らない。
- [ ] Native Messagingやpaper-manager側のImport失敗時、`Downloads/paper-manager-import/` の一時PDFが削除される。
- [ ] Native Messaging manifest未設定時、拡張popupに明確なエラーが出る。
- [ ] managed directory未設定時、paper-manager側で失敗し、requestが `failed/` に移動する。
- [ ] 重複DOI、重複タイトル、重複PDFパスで既存バリデーションが効く。
- [ ] `npm run build` が成功する。
- [ ] `cargo check` が成功する。

## References

- Chrome Native Messaging: https://developer.chrome.com/docs/extensions/develop/concepts/native-messaging
- Chrome downloads API: https://developer.chrome.com/docs/extensions/reference/api/downloads
- Tauri Deep Linking: https://v2.tauri.app/plugin/deep-linking/
