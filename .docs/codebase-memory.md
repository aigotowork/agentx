---
scope: AgentX checkout CBM runtime and indexing recovery
status: active
owner: AgentX project environment
related: [../.envrc, ../.cbmignore, ../.agent-guides/skills/codebase-memory/SKILL.md]
last_updated: 2026-11-17
---

# Codebase Memory in this checkout

Current configuration is owned by [global and longwork CBM operations](../../.docs/codebase-memory/README.md). The shared launcher now selects this checkout’s cache and a short hashed runtime directory. The incident and earlier project-only recovery below remain historical evidence; use the current operations guide for configuration and rollback.

## Runtime boundary

The project [directory environment](../.envrc) sets `CBM_CACHE_DIR=$HOME/.cache/codebase-memory-mcp-agentx` and `CBM_RUNTIME_DIR=$CBM_CACHE_DIR/runtime`. Both variables are required: changing the cache alone conflicts with the existing account daemon. The local ignored `.codex/config.toml` forwards both names through the CBM MCP server's `env_vars`. When recreating that local configuration, retain this forwarding. Existing Agent processes retain their startup environment; start new sessions from the loaded directory environment.

The isolated cache has `ui_enabled=false`, avoiding the existing account daemon's port 9749. This is CLI/MCP index isolation; it does not replace the existing graph UI or modify other projects' sessions and indexes. A project daemon still has the upstream limit of 64 simultaneous connections. Prefer one-shot CLI queries when many Agents are idle; isolation does not remove the upstream limit.

## Commands

The shell's direnv hook loads the environment at its next prompt. `direnv exec .` also makes it explicit for an already-running shell or automation:

```bash
direnv exec . codebase-memory-mcp cli index_repository --repo-path "$PWD"
direnv exec . codebase-memory-mcp cli list_projects
direnv exec . codebase-memory-mcp cli index_status --project home-long-works-longwork-agentx
direnv exec . codebase-memory-mcp cli search_graph --project home-long-works-longwork-agentx --name-pattern '.*' --limit 3
```

For another checkout, obtain the actual project name from `list_projects`. Native `index_repository` requires `--repo-path`; the shared wrapper described by the [Skill](../.agent-guides/skills/codebase-memory/SKILL.md) fills that argument automatically. Local cache logs are under `$CBM_CACHE_DIR/logs/`.

## Diagnosed incident

| Observation on 2026-11-17 | Meaning and correction |
| --- | --- |
| v0.12.1 account daemon PID 2528262 was alive, its socket connected successfully, and its UI returned HTTP 200. There were exactly 64 resident CBM stdio clients, each parented by Codex. | This was connection admission exhaustion, rather than a dead daemon or a repository parse failure. |
| The socket returned `CBM daemon connection capacity reached`; the CLI retried and ultimately reported `could not accept this client within 30000 ms`. The running daemon and installed executable had identical SHA-256 `6761236d038de87bf49da5695dd52d7928180d814624d4ceb8d8e849ba07f4c8`. | Increasing the timeout, deleting indexes, or replacing the same release does not resolve this capacity condition. Use the project runtime boundary above. |
| The first isolated full index reached business execution, then logged `pipeline.abort reason=semantic_inputs_changed` and returned `aborted_previous_preserved`. Its diagnostics included copied source/build outputs under `.artifacts/`. | CBM refuses to publish an index if its semantic inputs change during the run. The [CBM-only ignore rules](../.cbmignore) exclude runtime state, copied artifacts and verification/evidence subtrees. Maintained source, tests, task guides and designs remain discoverable. Changes to maintained source can still abort a run; retry after the affected source settles. |

The authoritative upstream v0.12.1 contracts are [host capacity](https://github.com/DeusData/codebase-memory-mcp/blob/v0.12.1/src/daemon/host.c#L37), [runtime hard limit](https://github.com/DeusData/codebase-memory-mcp/blob/v0.12.1/src/daemon/runtime.c#L124), [admission rejection](https://github.com/DeusData/codebase-memory-mcp/blob/v0.12.1/src/daemon/runtime.c#L2333), [runtime directory override](https://github.com/DeusData/codebase-memory-mcp/blob/v0.12.1/src/daemon/bootstrap.c#L227), and [ignore semantics](https://github.com/DeusData/codebase-memory-mcp/blob/v0.12.1/docs/cbmignore.md).

Graph readiness is distinct from freshness and parse coverage. Review the index result and coverage before relying on negative or exhaustive graph results; inspect excluded historical evidence directly when needed.

## Verified recovery

On 2026-11-17, a full index of the actual checkout using the isolated runtime and `.cbmignore` completed with `status=indexed`: 2,592 discovered files, 54,920 nodes, 210,955 edges, 15 partially parsed files, zero unusable parses and zero skipped files. The initial unfiltered run discovered 11,118 files. The successful coverage log is `$CBM_CACHE_DIR/logs/home-long-works-longwork-agentx-1794888468.log`.

A subsequent independent CLI session returned `index_status.status=ready` with `root_path=/home/long/works/longwork/agentx`, and `search_graph` returned stored symbols. The status explicitly reported `watched=false`, `reason=cli_session`; this proves persistent index/query availability, not automatic CLI refresh or adoption by already-running Agents. No upstream binary, account-wide MCP configuration or existing account daemon was replaced.
