# paper-manager 開発チェックポイント・テスト項目

## 1. 開発前提

- 対象は macOS 向けのデスクトップアプリとする。
- アプリ基盤は Tauri + React とし、ファイル操作・ローカルディレクトリ操作・永続化処理は Rust 側で担う。
- 永続化方式は JSON ファイルベースとする。
- PDF はアプリ内で表示せず、外部アプリで開く。
- 手動登録を必須の基本機能とし、DOI や外部 API からのメタデータ取得は補助機能として扱う。

## 2. フェーズ別チェックポイント

### Phase 0: 仕様確定

- [x] BibTeX 出力フォーマットを決定する。
  - DOI / arXiv から取得できる場合は、物理系ジャーナルで使われる BibTeX 形式をベースにする。
  - `title`, `author`, `journal`, `volume`, `issue`, `pages`, `numpages`, `year`, `month`, `publisher`, `doi`, `url` を扱えるようにする。
  - 欠損しているフィールドは空欄ではなく省略する。
  - 例:
  ```bibtex
  @article{PhysRevX.12.031042,
  title = {Beyond Conventional Ferromagnetism and Antiferromagnetism: A Phase with Nonrelativistic Spin and Crystal Rotation Symmetry},
  author = {\ifmmode \check{S}\else \v{S}\fi{}mejkal, Libor and Sinova, Jairo and Jungwirth, Tomas},
  journal = {Phys. Rev. X},
  volume = {12},
  issue = {3},
  pages = {031042},
  numpages = {16},
  year = {2022},
  month = {Sep},
  publisher = {American Physical Society},
  doi = {10.1103/PhysRevX.12.031042},
  url = {https://link.aps.org/doi/10.1103/PhysRevX.12.031042}
  }
  ```
- [x] 1 論文に対して複数ノートを許可するか決定する。
  - 複数ノートを許可する。
- [x] JSON ファイルの保存場所を決定する。
  - アプリを保存しているディレクトリに `setting/` ディレクトリを作成し、そこに JSON を保存する。
- [x] 外部 API 連携を必須機能にするか、任意補助機能にするか決定する。
  - arXiv API と DOI からのメタデータ取得を実装する。
  - オフライン時や取得失敗時も手動登録できるようにする。
- [x] PDF ファイル名の命名規則を決定する。
  - 基本形式は `{year}_{first_author}_{journal}.pdf` とする。
  - ファイル名に使えない文字は除去または `_` に置換する。
- [x] 管理対象ディレクトリの推奨構成を決定する。
  - 管理対象ディレクトリは設定画面から後で変更できるようにする。
  - 初回起動時は未設定を許可し、PDF 整理機能の利用時に設定を促す。

### Phase 1: アプリ基盤

- [x] Tauri + React のプロジェクトを作成する。
- [x] macOS で開発ビルドが起動する。
  - `npm run tauri -- build --debug --bundles app` で `.app` 生成を確認済み。
- [x] React 側から Rust command を呼び出せる。
  - `get_app_status`, `save_app_data`, `load_app_data` を React から呼び出す。
- [x] Rust 側から React 側へ正常系・異常系の結果を返せる。
  - command は `Result<T, String>` で成功値またはエラー文字列を返す。
- [x] Paper / Note / Settings の基本型を定義する。
- [x] JSON 読み書きの最小実装を作る。
  - アプリ実行ディレクトリ配下の `setting/app-data.json` に保存する。
- [x] エラー表示とログ出力の基本方針を実装する。
  - React 側で command 失敗時のエラーメッセージを画面表示する。

### Phase 2: 論文登録

- [x] PDF ファイルを選択して登録できる。
  - Tauri dialog plugin で PDF ファイルを選択し、`pdf_path` に反映する。
- [x] タイトル、著者、出版年、DOI、arXiv ID、URL、タグを手動入力できる。
- [x] DOI、タイトル、PDF パスを使って重複登録を検知できる。
  - Rust 側の `register_paper` command で保存前に検知する。
- [x] 登録した Paper レコードを JSON に保存できる。
  - `setting/app-data.json` に保存する。
- [x] アプリ再起動後に登録済みデータを復元できる。
  - 起動時に `load_app_data` で保存済み JSON を読み込む。
- [x] PDF が存在しない、読めない、権限がない場合にユーザーへエラーを返せる。
  - 選択された PDF パスが存在しない、ファイルでない、PDF でない場合は登録を拒否する。

### Phase 3: 一覧・検索・詳細編集

- [x] 登録済み論文を一覧表示できる。
- [x] タイトル、著者、タグ、読書状態、年で検索・絞り込みできる。
  - タイトル、著者、DOI、arXiv ID はキーワード検索対象にする。
- [x] ソート条件を指定できる。
  - 更新日時降順、出版年降順、出版年昇順、タイトル昇順を選択できる。
- [x] 一覧上で複数論文をチェックボックス選択できる。
  - 表示中の論文を一括トグルできる。
- [x] 論文詳細画面でメタデータを確認・編集できる。
  - 一覧の Edit から詳細編集フォームへ反映する。
- [x] 編集内容が JSON に保存され、再起動後も維持される。
  - `update_paper` command で保存前バリデーションと重複検知を行う。
- [x] データが 0 件の場合の空状態を表示できる。
  - 登録 0 件と検索結果 0 件を分けて表示する。

### Phase 4: ノート管理

- [x] 既存 Markdown ファイルを論文に紐付けできる。
  - 詳細画面の Notes から既存 `.md` / `.markdown` / `.txt` ファイルを選択して紐付ける。
- [x] 新規 Markdown ノートを作成して論文に紐付けできる。
  - `setting/notes/` に Markdown ファイルを作成し、Note レコードを保存する。
- [x] ノートを外部エディタで開ける。
  - Tauri opener plugin で実ファイルを外部アプリに渡す。
- [x] ノートのパスを JSON に保存できる。
  - Note の `file_path` と `file_type` を `setting/app-data.json` に保存する。
- [x] ノートファイルが削除・移動された場合に状態を検知できる。
  - `check_note_files` で存在確認し、欠落時は Missing として表示する。
- [x] 複数ノート対応の有無に応じて UI とデータ構造を統一する。
  - 1 論文に複数 Note を紐付ける設計で統一する。

### Phase 5: PDF 整理

- [x] 管理対象ディレクトリを設定できる。
  - Storage パネルの `Set managed directory` からディレクトリを選択し、Settings に保存する。
- [x] 命名規則に従って PDF をリネームできる。
  - `{year}_{first_author}_{journal}.pdf` を基本形式として生成する。
- [x] フォルダ分類に従って PDF を移動できる。
  - Paper detail の `Folder category` を管理対象ディレクトリ配下のサブフォルダ名として使う。
- [x] PDF 移動後に Paper の `pdf_path` を更新できる。
  - `organize_paper_pdf` command で移動後パスとカテゴリを JSON に保存する。
- [x] 同名ファイルが存在する場合の衝突処理を実装する。
  - 同名が存在する場合は `_2`, `_3` のように連番を付ける。
- [x] 移動・リネームに失敗した場合、元ファイルを破壊せずエラーを返せる。
  - `rename` 失敗時は copy/remove の順に fallback し、remove 失敗時はコピー先を削除してエラーにする。

### Phase 6: BibTeX 出力

- [x] チェックボックスで複数論文を選択できる。
  - Phase 3 の複数選択を BibTeX 出力対象として利用する。
- [x] 選択した論文だけを BibTeX 出力対象にできる。
  - `generate_bibtex` command に選択済み Paper ID を渡す。
- [x] 物理系論文で使いやすい最小限のフィールドに整形できる。
  - `title`, `author`, `journal`, `volume`, `issue`, `pages`, `numpages`, `year`, `month`, `publisher`, `doi`, `url`, `eprint` を扱う。
- [x] `bibtex_key` が未設定の場合の生成規則を適用できる。
  - DOI 末尾を優先し、なければ first author + year から生成する。
- [x] DOI、arXiv ID、journal、year、authors、title が期待通り反映される。
  - DOI がない場合は arXiv ID を `eprint` に出力する。
- [x] 欠損メタデータがある場合でも出力処理がクラッシュしない。
  - 空欄フィールドは省略し、選択なしの場合はエラー表示する。

### Phase 7: バックアップ・共有

- [x] JSON、PDF、ノートをディレクトリ単位でバックアップできる構成にする。
  - Storage パネルの `Backup` で `setting/` と管理対象ディレクトリを `paper-manager-backup-<timestamp>/` へコピーする。
- [x] Cloud Drive 配下で同期されても破綻しにくいパス管理にする。
  - 管理対象ディレクトリ単位で PDF と notes をまとめ、バックアップもディレクトリ単位で扱う。
- [x] 管理対象ディレクトリを別端末で開いたときにデータを復元できる。
  - `Restore` でバックアップ内の `setting/app-data.json` を読み込み、バックアップ内 `managed-directory/` を管理対象として復元する。
- [x] 絶対パスが変わった場合の復元・再リンク方針を用意する。
  - `Relink` で選択ディレクトリ以下をファイル名検索し、見つかった PDF / note パスへ更新する。

## 3. 機能別テスト項目

### 3.1 データモデル・永続化

- [ ] Paper / Note / Settings を JSON にシリアライズできる。
- [ ] JSON から Paper / Note / Settings を復元できる。
- [ ] 不正 JSON を読み込んだ場合にクラッシュしない。
- [ ] 保存前にバックアップまたは一時ファイルを使い、破損リスクを下げる。
- [ ] `created_at` と `updated_at` が期待通り更新される。

### 3.2 論文登録

- [ ] 必須項目が不足している場合に登録できない。
- [ ] 同一 DOI の論文を重複登録できない。
- [ ] 同一 PDF パスの論文を重複登録できない。
- [ ] DOI が空でも手動登録できる。
- [ ] PDF パスが未設定でもメタデータだけ登録できるか、仕様に従って拒否できる。

### 3.3 検索・フィルタ

- [ ] タイトルの部分一致で検索できる。
- [ ] 著者名の部分一致で検索できる。
- [ ] タグで絞り込みできる。
- [ ] 読書状態で絞り込みできる。
- [ ] 出版年で絞り込みできる。
- [ ] 複数条件を組み合わせた検索結果が正しい。
- [ ] 検索結果 0 件の表示が崩れない。

### 3.4 ノート

- [ ] 既存 Markdown ファイルを選択して紐付けできる。
- [ ] 新規ノート作成時にファイルが作られる。
- [ ] 外部エディタ起動に失敗した場合にエラーを表示する。
- [ ] 紐付け解除後も必要に応じてノートファイルを残す。
- [ ] ノートパスが存在しない場合に詳細画面で状態を表示する。

### 3.5 PDF 操作

- [ ] PDF を外部アプリで開ける。
- [ ] 存在しない PDF を開こうとした場合にエラーを表示する。
- [ ] 命名規則に従ったファイル名を生成できる。
- [ ] 使用できない文字をファイル名から除去または置換できる。
- [ ] 同名ファイル衝突時に上書きしない。
- [ ] 移動失敗時に Paper の `pdf_path` を不正更新しない。

### 3.6 BibTeX 出力

- [ ] 1 件の論文から正しい BibTeX を生成できる。
- [ ] 複数論文から連結された BibTeX を生成できる。
- [ ] 著者名の区切りが BibTeX 形式として正しい。
- [ ] 日本語・特殊文字・LaTeX 記号を含むタイトルで出力が壊れない。
- [ ] 欠損フィールドは仕様どおり省略または空欄処理される。
- [ ] 出力結果をクリップボードまたはファイルに渡せる。

## 4. 非機能テスト項目

- [ ] 数百件の Paper データで一覧表示が実用速度で動く。
- [ ] 数千件の Paper データで検索が極端に遅くならない。
- [ ] オフライン環境でも手動登録・検索・編集・BibTeX 出力が使える。
- [ ] 外部 API が失敗してもアプリ全体が停止しない。
- [ ] ファイル権限エラーをユーザーが理解できる形で表示する。
- [ ] macOS の通常ユーザー権限でビルド・起動・ファイル操作ができる。
- [ ] Cloud Drive 同期中の一時的なファイル不整合でクラッシュしない。

## 5. 未決事項・決定ゲート

| ID | 論点 | 決定タイミング | 未決のまま進めた場合のリスク |
| --- | --- | --- | --- |
| TBD-01 | BibTeX 出力フォーマット | 解決済み | 物理系ジャーナル向け形式をベースにする |
| TBD-02 | 1 論文に対するノート数 | 解決済み | 複数ノートを許可する |
| TBD-03 | JSON 保存場所 | 解決済み | アプリ保存ディレクトリ配下の `setting/` に保存する |
| TBD-04 | 外部 API 依存度 | 解決済み | arXiv API と DOI 取得を実装し、手動登録も維持する |
| TBD-05 | PDF 命名規則 | 解決済み | `{year}_{first_author}_{journal}.pdf` を基本形式にする |

## 6. MVP 完了条件

- [ ] PDF またはメタデータから論文を登録できる。
- [ ] 登録済み論文を一覧・検索・編集できる。
- [ ] Markdown ノートを論文に紐付けできる。
- [ ] 複数選択した論文から BibTeX を出力できる。
- [ ] JSON 保存により、再起動後も登録データが保持される。
- [ ] PDF を外部アプリで開ける。
- [ ] 主要操作でクラッシュせず、失敗時に原因が分かるエラーを表示できる。
