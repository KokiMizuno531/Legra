# paper-manager

macOS 向けの論文管理デスクトップアプリです。Tauri + React を基盤に、論文メタデータ、Markdown ノート、PDF パス、BibTeX 出力用情報をローカル JSON で管理します。

## Development

```sh
npm install
npm run build
npm run tauri dev
```

## Phase 1 scope

- React UI から Rust command を呼び出す
- `Paper` / `Note` / `Settings` の基本型を Rust 側に定義する
- アプリ実行ディレクトリ配下の `setting/app-data.json` にサンプル JSON を保存・読み込みする


## check
npm run tauri dev