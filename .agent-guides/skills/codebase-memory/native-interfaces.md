
# Native CLI and MCP usage

Use the local code knowledge graph to locate structural evidence, then confirm
material conclusions against current source. Read [runtime and CLI usage](README.md)
for interface selection, MCP connection and version-qualified command examples.

## Select the useful path

| Question | Tool |
| --- | --- |
| Which index belongs to this checkout? | `list_projects`, matching `root_path` to the working directory |
| What symbols or modules match a name? | `search_graph` |
| Who calls a symbol, or what does it call? | `trace_path` with the relevant direction |
| What is the exact indexed source? | `get_code_snippet`, followed by current-file inspection when freshness is uncertain |
| What relationships and query syntax exist? | `get_graph_schema`, then a bounded `query_graph` |
| What could a working-tree change affect? | `detect_changes` and targeted source/consumer checks |
| Are the evidence files indexed and current? | `check_index_coverage` |
| Is this a text lookup or known-file change? | `rg`, direct reads, or `search_code` when its graph enrichment helps |

## Structural investigation

1. Call `list_projects` before the first structural investigation in a session,
   or recover a still-valid exact project binding from the task. Match the actual
   checkout root; pass the returned `name` as `project`. Do not guess it from the
   folder name. If no index exists, index the authorized repository or continue
   with explicit source-only evidence.
2. Check `index_status` when an index may be running or its state is unknown.
   Discover exact qualified symbols with `search_graph`; use `trace_path` for
   relevant caller/callee paths and read the source behind material claims.
3. Check `check_index_coverage` once for the collected evidence paths. Include
   the relevant scope when making an absence or exhaustive claim. For stale,
   partial, skipped, excluded, pending or unknown coverage, refresh the index or
   inspect the affected files/ranges directly.
4. Follow relevant pagination and output-limit indicators. Stop when the task's
   question is answered. A quick positive lookup may remain provisional; complete
   impact, unused-code and absence claims need the corresponding source scope and
   explicit limitations. A clean coverage result means no recorded gap, not proof
   that static analysis found every runtime relationship.

## Boundaries and recovery

- Source declarations, repository guidance and current design records remain
  authoritative. Keep durable project decisions there; do not move them into
  CBM's `manage_adr` store as a second editable source.
- Tool names and inputs come from the installed version's MCP schema or
  `codebase-memory-mcp cli <tool> --help`. The CLI exposes the same native tools
  to Agents without direct MCP access. Never claim tools inherited by a child
  unless that runtime actually provides them.
- Reuse existing query evidence in an authorized handoff; this skill does not
  require spawning graph specialists, repeated queries or an extra review tier.
- CLI indexing is one-shot and does not establish automatic watch coverage.
  Recheck or refresh relevant evidence after edits instead of assuming it is live.
- Do not delete shared indexes, alter account-wide CBM configuration or ingest
  unrelated repositories as part of an ordinary code lookup. Diagnose an actual
  tool failure, then use source fallback while independent work continues.

This is an adaptation of the MIT-licensed upstream embedded skill from
`DeusData/codebase-memory-mcp` `v0.12.1`. See [provenance](README.md#provenance),
the [upstream snapshot](references/upstream-skill.md), and [license](LICENSE).
