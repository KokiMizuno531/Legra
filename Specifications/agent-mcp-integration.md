# Codex / Claude MCP Integration

## Goal

Codex AppまたはClaude Desktopが見つけた論文を、DOIまたはarXiv IDからLegraへ安全に
インポートできるようにする。エージェントはインポートジョブを投入し、Legra本体が書誌の
解決、PDF取得、重複判定、命名、保存、カタログ更新を行う。

対象クライアントは次のとおり。

- Codex App: ローカルSTDIO MCP serverを絶対パスで登録する。
- Claude Desktop (macOS / Windows): リリースのMCPBを第一選択とする。
- Claude Desktop開発時: Developer configへSTDIO serverのJSONを手動で追加する。

## Architecture and writer boundary

```text
Codex App / Claude Desktop
          |
          | stdio MCP
          v
      legra_mcp
          |
          | validated, versioned job
          v
 pending -> processing -> done
                    \----> failed
          |
          v
 Legra desktop app (sole catalog/PDF writer)
```

`legra_mcp` はLegraのカタログやmanaged directoryを直接更新しない。MCP serverが書き込める
のはagent import queueとそのstatusだけであり、`app-data.json`、managed library catalog、
PDF、noteの唯一のwriterはLegra本体とする。

Legraのメインwindowは起動時と15秒ごとにagent queueを処理する。Legra停止中もジョブ投入は
可能で、pendingのまま次回起動を待つ。したがって `import_paper` の成功は「登録完了」ではなく
「キュー投入完了」を表す。

## MCP tools

| Tool | Mode | Purpose |
|---|---|---|
| `resolve_paper` | read-only | DOI/arXiv IDを正規化し、Legra内に同じidentifierの登録があるかをローカルで確認する。 |
| `search_library` | read-only | タイトル、著者、DOI、arXiv ID等でローカルlibraryを検索する。 |
| `list_categories` | read-only | 現在のfolder categoryを返し、保存先の候補選択に使う。 |
| `import_paper` | write, idempotent | 検証済みidentifierからagent import jobを投入し、`job_id`を返す。 |
| `get_import_status` | read-only | `job_id`のpending/processing/done/failedと結果を返す。 |

MCP annotationでは前三者と `get_import_status` にread-onlyを付ける。`import_paper` は
非破壊・idempotentとして宣言し、Codex側ではwrite toolだけ承認を求める設定を推奨する。

`import_paper` の論理入力は次の形とする。

```json
{
  "identifier": "10.xxxx/example",
  "category": "physics/spin",
  "tags": ["agent-import"],
  "download_policy": "open_access_only",
  "idempotency_key": "client-generated-stable-key"
}
```

- `identifier` はDOI、doi.org URL、arXiv ID、arxiv.orgのabs/pdf URLのみ。queueには正規化後の
  `{ "kind": "doi|arxiv", "value": "..." }` を保存する。
- `category` と `tags` は任意であり、path traversalや不正な文字をLegra側でも再検証する。
- `download_policy` の既定値は `open_access_only`。取得不可の場合はmetadata-onlyを許容する。
- 同じclient、同じ `idempotency_key`、同じ引数の再送は既存jobを返し、二重登録を作らない。
  同じkeyを異なる引数へ再利用した場合は、別の論文を誤って成功扱いにしないよう拒否する。
- 任意の `pdf_url`、HTTP URL、ローカル `pdf_path`、認証情報は受け付けない。

`get_import_status` のdone resultには最低限 `job_id`、`paper_id`、title、PDFの有無、warningを
含める。PDF本体や任意のローカルファイル内容をMCP responseへ含めない。

## Job lifecycle

1. `legra_mcp` がrequest schema、identifier、category、tagsを検証する。
2. 同じclientと `idempotency_key` の既存jobを確認する。key省略時は正規化済み引数から安定した
   keyを生成する。
3. 一時ファイルへ完全なJSONを書き、atomic renameで `pending` として公開する。
4. Legra本体がatomic renameで `processing` へclaimする。他プロセスはclaim済みjobを処理しない。
5. DOI/arXiv metadataを解決し、libraryのDOI/arXiv/fingerprint重複を確認する。
6. 許可されたopen-access PDFがあれば検証して一時保存し、命名・category配置を行う。
7. catalog保存後にresultを `done` へ移す。metadata-only完了もdoneで、warningに理由を残す。
8. 検証・解決・保存に失敗した場合は、再試行可能性とmessageを持つ `failed` resultへ移す。
9. `get_import_status` は各状態を同じ `job_id` で参照できる。

クラッシュ後もpending/processing jobを消失させない。catalog保存前のjobをdoneとして公開しては
ならない。claim後10分以上更新されていないprocessing jobは再開時にrequeueする。catalog保存後に
result公開だけが中断したjobは再importせずterminal stateへ修復する。複数のLegra processが起動しても、
新しいclaimを1つのjobへ同時に取得しない。

## Download and security policy

- DOI/arXiv ID以外をimport sourceとして受け入れない。
- arXivまたは公開・許可されたsourceからのPDF取得だけをbest effortで行う。
- `open_access_only` は認証・Cookie・paywallを使わない取得ポリシーであり、再利用licenseの法的な
  判定を意味しない。
- paywall、購読、ログイン、Cookie、browser sessionを迂回・転用しない。
- PDF取得URLはHTTPS、redirectごとの再検証、private/loopback/link-local拒否を必須とする。
- response size上限と `%PDF-` magicを検証する。
- PDF取得失敗は安全にmetadata-onlyへ縮退できる。HTMLや部分ファイルをlibraryに残さない。
- MCPから任意のローカルpathを読み書きできるtoolを公開しない。

## Connector distribution

### Legra app resource

各OSのrelease buildで `paper_manager_native_host` と `legra_mcp` をrelease buildし、両方を
Tauri resourceへ同梱する。Settingsの `Install connector` は `legra_mcp` をLegraのuser data
配下へcopy/updateし、絶対server pathと設定snippetを表示する。

Settingsは以下を自動編集しない。

- Codexの `config.toml`
- Claude Desktopのdeveloper config
- Claude Desktopへinstall済みのMCPB設定

`Remove connector` はLegraがcopyしたbinaryだけを削除する。クライアント側server entryの削除は
ユーザーが行う。

### Claude Desktop MCPB

macOS arm64とWindows x86_64 releaseでは、platform別 `legra_mcp`、`manifest.json`、iconを
含むMCPBを生成する。manifestとserver binaryはarchive rootから相対参照できなければならない。
Linux MCPBはClaude Desktopの公式対象外のため生成しない。

## Acceptance checklist

### MCP and queue

- [ ] Codex Appから `list_categories` と `search_library` を実行できる。
- [ ] Claude Desktopから `resolve_paper` を実行できる。
- [ ] DOI jobとarXiv jobを投入し、一意な `job_id` が返る。
- [ ] 同じ `idempotency_key` の再送でjobやpaperが二重作成されない。
- [ ] Legra停止中のjobがpendingに残り、次回メインwindow起動時に処理される。
- [ ] Legra起動中のjobが15秒以内のpollでclaimされる。
- [ ] `get_import_status` でpending、processing、done、failedを区別できる。
- [ ] Chrome importとagent importのUI messageが混同されない。
- [ ] 多重pollでも同じagent jobが同時処理されない。

### Library and downloads

- [ ] DOI/arXivの正規化と重複判定が機能する。
- [ ] open-access PDFはmanaged directoryへ命名・category配置される。
- [ ] PDF取得不可はmetadata-onlyで完了し、warningが残る。
- [ ] HTTP、localhost、private IP、redirect先private IP、oversize、非PDFが拒否される。
- [ ] 任意URL、任意local path、認証情報をMCP inputへ渡せない。
- [ ] catalog保存前のcrashでjobがdoneにならず、復旧後に再処理できる。

### UI and packaging

- [ ] SettingsでCheck、Install/Update、Removeとversion/path/statusを確認できる。
- [ ] Codex TOMLとClaude Desktop JSONが選択・copy可能な形で表示される。
- [ ] unpackaged development buildで `legra_mcp` resource不足が明確なerrorになる。
- [ ] Codex Appで絶対server pathをSTDIO commandとして登録できる。
- [ ] macOS/Windows MCPBをClaude Desktopへinstallしてtoolsが表示される。
- [ ] `cargo test` がagent MCP integration testを実行する。
- [ ] `cargo check --bin legra_mcp`、`node --check scripts/prepare-mcpb.mjs`、`npm run build` が成功する。
