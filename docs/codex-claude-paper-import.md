# Import papers from Codex App or Claude Desktop

Legra 0.3.0 and later include a local MCP server that lets Codex App or Claude Desktop search your
Legra library and queue DOI, Nature article URL, or arXiv papers for import. Legra remains the only process that changes
the catalog or managed PDF directory.

## Install or upgrade Legra on macOS

Install Legra for the first time:

```sh
brew tap KokiMizuno531/legra
brew install --cask legra
```

Upgrade an existing installation:

```sh
brew update
brew upgrade --cask legra
open -a Legra
```

Confirm the installed version with `brew info --cask legra` or
`brew list --versions --cask legra`.

If the MCP connector was already installed, opening the upgraded Legra app refreshes its private
connector copy when the version changes. First-time users must install the connector from Legra
Settings. Restart Codex after an upgrade so that it launches the refreshed binary.

## Connect Codex App

1. Open the main Legra window and choose `Legra -> Settings...`.
2. Under `Codex / Claude connector`, select `Install connector`.
3. Select `Check connector` and confirm that the connector is installed and current.
4. Select `Copy TOML`.
5. Merge the copied table into `~/.codex/config.toml`. Keep every unrelated setting and MCP server
   already in the file.
6. Restart Codex App. Open a new task and use `/mcp` to confirm that `legra` is connected.

The generated configuration has this shape. Use the exact paths shown by Legra instead of copying
the placeholders below:

```toml
[mcp_servers.legra]
command = "/absolute/path/to/legra_mcp"
env = { LEGRA_SETTING_DIR = "/absolute/path/to/Legra/data" }
default_tools_approval_mode = "writes"
```

`default_tools_approval_mode = "writes"` lets read-only library tools run normally while asking
for approval before `import_paper` queues a write. The Codex desktop app, CLI, and IDE extension
share this MCP configuration on the same host. See the
[official OpenAI MCP documentation](https://developers.openai.com/codex/mcp).

As an alternative for a standard Legra data directory, use `Settings -> MCP servers -> Add server`
in Codex App, name the server `Legra`, select `STDIO`, and enter the Server path displayed by Legra.
The TOML route is recommended because it also preserves the exact data directory and write approval
policy.

## Verify the Codex connection

Start with a read-only request:

```text
Use Legra to list my categories and search the library for "Attention Is All You Need".
```

Then test an import. This creates a real library record, so use a paper you want to keep or delete
the test record afterward:

```text
Import arXiv:1706.03762 into Legra with the tag "codex-import". Check the import status until it is done or failed.
```

Approve the `import_paper` call when Codex asks. A typical successful sequence is:

1. `resolve_paper` normalizes the DOI, Nature article URL, or arXiv ID and checks for an existing record.
2. `import_paper` returns a stable `job_id` and an initial `pending` state.
3. Legra claims the job from its main window at startup or during its 15-second poll.
4. `get_import_status` returns `done` or `failed`.
5. `search_library` finds the imported record.

Re-sending the same idempotent request returns the same job instead of creating another paper.
`done` with `has_pdf: false` is a valid metadata-only result when an unauthenticated PDF is not
available.

## Queue and download behavior

- Keep the main Legra window open for imports to run immediately. When Legra is closed, jobs remain
  pending until the next launch.
- Imports accept DOI, `doi.org`, `nature.com/articles`, arXiv ID, and `arxiv.org` identifiers. Other publisher URLs and local
  paths are rejected intentionally.
- PDF retrieval is unauthenticated and best effort. Legra does not reuse browser cookies, bypass a
  paywall, or determine the legal reuse license of a paper.
- MCP responses report whether a PDF exists but never expose its local filesystem path.

## Claude Desktop

On macOS or Windows, download the platform-specific `.mcpb` file from the matching Legra GitHub
Release and open it with Claude Desktop. Restart Claude Desktop after installation.

For a custom or legacy Legra data directory, use Legra's `Copy JSON` configuration instead of the
MCPB default, merge it into Claude Desktop's developer configuration, and keep the generated
`LEGRA_SETTING_DIR` value. Linux is not currently an official Claude Desktop target, so Legra does
not publish a Linux MCPB.

## Troubleshooting

- **Legra is missing from `/mcp`:** check that the connector is current, verify the `legra` table in
  `~/.codex/config.toml`, then restart Codex App.
- **Codex sees an empty or different library:** replace `LEGRA_SETTING_DIR` with the exact value from
  Legra's `Copy TOML` output.
- **A job remains pending:** open the main Legra window and wait at least 15 seconds.
- **An import is done without a PDF:** the paper was registered as metadata-only because no direct,
  unauthenticated PDF passed Legra's checks.
- **An import fails:** inspect `get_import_status`, then verify that the source is an unambiguous DOI
  or arXiv identifier.
- **Homebrew still shows an older release:** run `brew update`, inspect `brew info --cask legra`, and
  retry `brew upgrade --cask legra`.

The connector transport and end-to-end import flow were tested with Codex CLI 0.150.0, including
resolve, queue, status, search, PDF download, and idempotent retry. Codex App uses the same host MCP
configuration, but its Settings UI was not part of that automated live test.
