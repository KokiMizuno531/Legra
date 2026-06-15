# arXiv ID / DOI による論文メタデータ自動取得 実装計画

## Summary

- 登録ウィンドウで arXiv ID または DOI だけを入力して、論文メタデータを自動取得できるようにする。
- `Register paper` の初回押下ではメタデータを取得してフォームへ反映し、ユーザー確認後の再押下で保存する。
- arXiv は公式 API の `id_list` Atom XML、DOI は Crossref REST API の `/works/{doi}` JSON を利用する。
- 取得失敗時も手動登録できる状態を維持する。

## Key Changes

- Rust 側に `fetch_paper_metadata` command を追加する。
  - 入力は `doi` と `arxiv_id`。
  - DOI が入力されている場合は Crossref を優先する。
  - DOI が空で arXiv ID がある場合は arXiv API を使う。
- 登録フォームを DOI / arXiv ID 起点の流れに変更する。
  - DOI / arXiv ID をフォーム上部に配置する。
  - `Fetch metadata` で明示的に取得できる。
  - `Register paper` でも未取得の DOI / arXiv ID があれば、先に取得して保存はしない。
- `Paper` に既に存在する `volume`, `issue`, `pages`, `numpages`, `month`, `publisher`, `abstract_text` を登録・編集フォームと保存 DTO に通す。
- 追加依存関係は HTTP クライアント、JSON、XML parsing の最小構成にする。

## Data Mapping

- Crossref:
  - `title[0]` -> `title`
  - `author[].given/family` -> `authors`
  - `published-print` / `published-online` / `published` / `issued` -> `year`, `month`
  - `container-title[0]` -> `publication`
  - `volume`, `issue`, `page`, `publisher`, `DOI`, `URL` -> 同名または対応フィールド
- arXiv:
  - `<title>` -> `title`
  - `<author><name>` -> `authors`
  - `<published>` -> `year`, `month`
  - `<id>` -> `url`
  - `<summary>` -> `abstract_text`
  - `<arxiv:doi>` -> `doi`
  - `<arxiv:journal_ref>` -> `publication`

## Test Plan

- DOI のみ入力して `Register paper` を押すと、Crossref から取得してフォームが埋まる。
- arXiv ID のみ入力して `Register paper` を押すと、arXiv から取得してフォームが埋まる。
- 自動入力後、再度 `Register paper` を押すと保存される。
- `Fetch metadata` ボタンで明示的に再取得できる。
- API が 404、オフライン、パース失敗の場合、入力済みフォームを壊さずエラーを表示する。
- DOI と arXiv ID の両方がある場合は DOI を優先する。
- 手動登録、PDF 選択、重複検知、編集ウィンドウ、BibTeX 出力が壊れていないことを確認する。

## References

- arXiv API User's Manual: https://info.arxiv.org/help/api/user-manual.html
- Crossref REST API: https://www.crossref.org/documentation/retrieve-metadata/rest-api/
