# UI 調整時の機能回帰チェックリスト

UI の見た目や配置を変更した後、主要機能が壊れていないか確認するためのチェックリスト。

## 1. 起動・保存状態

- [ ] `npm run tauri dev` でアプリが起動する。
- [ ] 起動時に既存の `setting/app-data.json` が読み込まれ、登録済み論文数が表示される。
- [ ] `Reload` を押すと保存済みデータが再読み込みされる。
- [ ] エラー表示が出ても画面操作が継続できる。

## 2. Storage 操作

- [ ] `Set managed directory` で管理対象ディレクトリを選択できる。
- [ ] 選択後、Storage 表示の managed directory が更新される。
- [ ] `Backup` でバックアップディレクトリを作成できる。
- [ ] `Restore` でバックアップからデータを復元できる。
- [ ] `Relink` で PDF / note のパスを再リンクできる。

## 3. 論文登録ウィンドウ

- [ ] `Register paper` ボタンで登録用の別ウィンドウが開く。
- [ ] DOI を入力して `Fetch metadata` を押すと Crossref からタイトル・著者・年・journal 等が埋まる。
- [ ] arXiv ID を入力して `Fetch metadata` を押すと arXiv からタイトル・著者・年・URL 等が埋まる。
- [ ] `Fetch metadata` が失敗しても手動入力を続けられる。
- [ ] `PDF path` の `Select` で PDF を選べる。
- [ ] `Folder category` の `Select` は managed directory 配下から選択できる。
- [ ] `Register paper` 1回で登録と PDF organization が実行される。
- [ ] PDF なしでもメタデータのみ登録できる。
- [ ] 同一 DOI / title / PDF path の重複登録は拒否される。

## 4. メイン画面の一覧・選択

- [ ] 左カラムに `BibTeX` と `Registered papers` の操作が表示される。
- [ ] 左カラムの論文一覧は checkbox と title のみで表示される。
- [ ] title をクリックすると右カラムに該当論文のカードが表示される。
- [ ] checkbox の ON/OFF は BibTeX 出力対象だけを変更し、右カラムの表示対象は変えない。
- [ ] `Toggle visible` で表示中の論文を一括選択 / 解除できる。
- [ ] `Clear selection` で BibTeX 選択が解除される。
- [ ] 検索欄で title / author / DOI / arXiv ID を検索できる。
- [ ] tag / category / status / year フィルタが動く。
- [ ] sort を変更すると左カラムの一覧順が変わる。
- [ ] フィルタ変更で右カラムの表示対象が除外された場合、未選択表示に戻る。

## 5. 右カラムの論文カード

- [ ] 選択中論文の title, authors, status, tags, category, DOI, PDF path が表示される。
- [ ] `Edit` で編集用の別ウィンドウが開く。
- [ ] `Open PDF` で PDF が Preview で開く。
- [ ] PDF path がない論文では `Open PDF` が無効になる。
- [ ] 紐付け済み note の `Open: ...` ボタンが表示される。
- [ ] Markdown note は MarkText で開く。
- [ ] note がない場合は `No notes` と表示される。

## 6. 編集ウィンドウ

- [ ] `Edit` で開いたウィンドウに選択論文の詳細が読み込まれる。
- [ ] title / authors / year / status / tags / DOI / arXiv ID / URL を編集できる。
- [ ] volume / issue / pages / numpages / month / publisher / abstract を編集できる。
- [ ] PDF path を変更できる。
- [ ] Folder category を managed directory 配下から選択できる。
- [ ] `Save changes` で JSON に保存され、メイン画面へ反映される。
- [ ] `Organize PDF` で PDF が命名規則に従って移動され、`pdf_path` が更新される。

## 7. BibTeX 出力

- [ ] 論文を1件選択して `Generate BibTeX` すると出力欄に BibTeX が表示される。
- [ ] 複数選択時は複数 entry が出力される。
- [ ] arXiv ID がある論文は `@misc` 形式で出力される。
- [ ] DOI / journal 情報がある論文は `@article` 形式で出力される。
- [ ] `Copy` でクリップボードにコピーできる。
- [ ] `Save .bib` で `.bib` ファイルを保存できる。
- [ ] 選択なしで `Generate BibTeX` した場合、エラー表示になる。

## 8. レスポンシブ・表示崩れ

- [ ] 通常幅のデスクトップ画面で左カラムと右カラムが重ならない。
- [ ] ウィンドウ幅を狭めても sort / filter / buttons が枠外にはみ出さない。
- [ ] 長い title / DOI / PDF path / category が表示領域を壊さない。
- [ ] ボタン内テキストが切れたり重なったりしない。
- [ ] empty state が登録0件、検索結果0件、右カラム未選択で適切に表示される。

## 9. 最低限のコマンド確認

- [ ] `npm run build`
- [ ] `cargo check`
- [ ] `npm run tauri -- build --debug --bundles app`

## 10. UI 変更完了条件

- [ ] 上記チェックで主要機能に回帰がない。
- [ ] 変更した画面の主操作を少なくとも1回ずつ手動確認した。
- [ ] 失敗した項目がある場合、原因・再現手順・未対応理由を記録した。
