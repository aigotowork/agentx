---
name: codebase-memory
description: "Use cbm on demand for the current repository: code architecture, symbol search, callers, dependencies, call chains, change impact, and dead-code analysis."
---

# Codebase Memory

Run `cbm <tool> [flags]` from the current repository or a subdirectory. The wrapper fills the repository for indexing and the project for single-project tools. Read `cbm <tool> --help` when parameters are needed. `cbm` is a Ruby adapter requiring Ruby 2.6+, Git and the separate `codebase-memory-mcp` native engine; finding the adapter alone does not prove the engine is installed. The portable kit supplies the adapter through its tool owner, not through this Skill. Use the consuming project's setup diagnostics and the engine's official distribution; if unavailable, continue with source reads and `rg`.

Use the graph to discover locations and relationships. For a known TS/TSX component's props or JSX, use [tsxctx](../tsxctx/SKILL.md); for exact text or a known source range, use `rg` or a focused read. Once a function's full source is available, do not fetch it again through another tool unless information is missing.

For direct engine CLI or MCP access without `cbm`, read [interface selection and resources](README.md) and [native usage](native-interfaces.md). The wrapper fills repository/project arguments; direct tools require the actual project binding. Do not mix their flags or infer availability across sessions.

## Start with a usable index

Call `cbm index_status` before relying on the graph. `ready` means populated, not fresh or fully parsed. If missing, stale, or freshness is uncertain, run `cbm index_repository`; reuse it until relevant source changes. CLI does not watch files. If unavailable, use source reads/`rg` and state the limit. Check result errors even when the command exits 0.

## Choose the workflow

Use only the steps needed to answer the question; stop once the relevant behavior is established.

| Scenario | Short workflow |
| --- | --- |
| Understand an unfamiliar repository/module | `get_architecture` (optionally `--path`) → `search_graph` for relevant symbols → trace an entry point → read the key implementation with `get_code_snippet`. |
| Diagnose a bug or follow a call chain | Find the exact symbol with `search_graph` → `trace_path --direction inbound` for callers or `outbound` for dependencies → read conditions and behavior along the relevant edges. Tracing starts from one function, not two endpoints. |
| Review changes or choose regression tests | `detect_changes --base-branch <ref>` → inspect changed symbols and affected callers → trace with `--include-tests true` when tests matter → verify behavior and select tests. The default base is `main`; `--since` overrides it with merge-base semantics. Worktree changes are included, so check whether they belong to the review. |
| Find dead-code candidates | Check scope coverage → `get_graph_schema` → `query_graph` for functions without incoming calls → inspect candidates and their consumers. Rule out exported APIs, framework entry points, callbacks, reflection and tests before proposing removal. |
| Follow service or parameter relationships | Locate the handler/function → `trace_path --mode cross_service` for service edges, or `--mode data_flow` with optional `--parameter-name` for argument flow → verify the relevant source/configuration. These are indexed relationships, not a complete runtime trace. |

For a file's declarations use `get_file_outline`; for indexed text use `search_code`. For other structural questions, inspect `get_graph_schema` before writing `query_graph`. `compare_graphs` requires explicit `--base-project` and `--target-project`.

```bash
cbm search_graph --name-pattern '.*Handler.*' --limit 10
cbm trace_path --function-name '<exact-name>' --direction inbound --include-tests true
cbm check_index_coverage --paths 'src/handler.ts' --scopes 'src'
```

## Verify before concluding

- Reuse qualified names from search results. Keep queries narrow; inspect depth, truncation and continuation fields before claiming completeness. `trace_path` excludes tests by default; `detect_changes` pages changed files, impacted symbols and modules independently.
- Batch evidence paths in `check_index_coverage`; include relevant scopes for negative/exhaustive claims. Freshness and parse coverage are separate. Read flagged or uncertain files directly; an empty graph result alone does not prove absence.
- Verify material conclusions against source. Use `trace_path --include-evidence true` for surprising edges; resolver confidence is not proof. Report candidate impact/dead code as candidates until verified.

Keep architecture decisions in the repository's existing authoritative records; `manage_adr` is optional and must not create a competing source. In upstream 0.11.0, `ingest_traces` counts submissions but does not create runtime edges.
