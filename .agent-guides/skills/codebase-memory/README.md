# Codebase-memory interfaces and resources

The canonical [Skill](SKILL.md) supports the `cbm` wrapper. Use
[native CLI and MCP usage](native-interfaces.md) when using the engine directly.
Choose the interface actually available; copying guidance installs no engine,
wrapper, client integration or account configuration.

## Runtime and client connection

The imported native-interface notes were written against `codebase-memory-mcp
0.12.1`; the existing wrapper method retains its own supported contract. Locate
the actual executable and check its version/help or MCP schema before use.
Installation location, client configuration and cache selection belong to the
consuming environment; no author-machine path is required.

Check the current project's actual `CODEX_HOME` and MCP configuration before
claiming direct tool availability. When its `config.toml` registers the stdio
server as `codebase-memory-mcp`, new sessions using that configuration can
discover its tools. Existing sessions retain their loaded connections and role
snapshots. Without an MCP connection, use the CLI below or targeted source reads;
copying this skill does not install or configure a client integration.

The default per-account cache is `~/.cache/codebase-memory-mcp`; indexes are
identified by their actual checkout roots. Keep this cache root consistent across
concurrent CBM clients, as upstream coordinates exact builds and cache roots
through a shared daemon. Indexes are derived local state, not project source or a
canonical decision log. No UI or permanent daemon is needed for CLI queries.

## CLI workflow

Run commands from the repository being examined:

```sh
codebase-memory-mcp --version
codebase-memory-mcp cli list_projects --format json
codebase-memory-mcp cli index_repository --repo-path "$PWD" --mode fast
```

Select the returned index whose `root_path` matches this checkout. Copy its exact
`name` into `--project`; do not assume that it equals the checkout's basename.
Fast mode supplies an initial structural index without optional semantic work.
Use the native indexing help to choose broader modes when the task needs them.

Discover the index for the current checkout with `list_projects` and match its
actual root. Never reuse an index name copied from another project's guide.

```sh
codebase-memory-mcp cli search_graph --help
codebase-memory-mcp cli search_graph --project '<returned-name>' --name-pattern '.*Handler.*'
codebase-memory-mcp cli trace_path --project '<returned-name>' --function-name '<exact-symbol>' --direction both
codebase-memory-mcp cli check_index_coverage --project '<returned-name>' --paths '["src/handler.ts"]' --format json
```

The final paths and symbol above are examples: substitute evidence from the actual
repository. Use `get_graph_schema` before writing relationship queries and
`cli <tool> --help` for the installed version's flags. Read tools default to compact
trees; `--format json` requests machine-readable output where supported. Normal
CLI indexing waits for completion; do not use asynchronous indexing from a lone
one-shot CLI session, whose temporary daemon cannot keep that job alive.

An empty result can reflect exclusions, stale indexing or missing language/runtime
relationships. Check coverage and inspect source before concluding that code,
callers or dependencies do not exist. Indexed document entities do not establish application-code coverage. Inspect
the current index's own exclusions and collected source paths before making
coverage claims; runtime/cache paths are not application-source evidence.

## Provenance

- Upstream: <https://github.com/DeusData/codebase-memory-mcp>
- Release: `v0.12.1`; commit: `a7158ac2f48ca646b4527341b76f930f2aeff87e`.
- Native archive: `codebase-memory-mcp-linux-amd64.tar.gz`.
- Verified archive SHA-256: `8453015145f072c1b9b019cbdd02cea8cc6f18cc5e116036dd20992631c638c7`.
- [Native skill snapshot](references/upstream-skill.md): decoded from
  [`skill_content` in `src/cli/cli.c`](https://github.com/DeusData/codebase-memory-mcp/blob/v0.12.1/src/cli/cli.c).
  Upstream publishes the skill inside its installer rather than as a standalone
  installable `SKILL.md` directory.
- [MIT license](LICENSE). The snapshot is reference material; the project adapter
  owns local usage and avoids importing unsupported performance claims or
  mandatory delegation/review workflows.

For updates, inspect a chosen upstream release and its actual CLI/schema, verify
the artifact digest, update the machine binary deliberately, and reconcile the
local adapter and snapshot. The ordinary multi-client installer would also change
global skills and client integrations, so select its configuration behavior deliberately; ordinary queries do not authorize account-wide setup.
