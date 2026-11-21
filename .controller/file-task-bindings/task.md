---
schema: task
status: completed
description: Implement file-backed assignment, state normalization and rebuildable observation through existing native launch/query paths.
agent: naga
---

# Assignment

Parent: [Delivery](delivery.md). User wants a fast, lightweight implementation, not a task management engine. One coder owns the complete design-to-consumer change. Read project guidance and affected module entries; record a concise structural design before implementation and establish native signatures/boundaries before behavior. No delegation. Preserve the substantial pre-existing dirty workspace; no broad formatting, resets, unrelated repairs or commits.

## Accepted requirements

- Repeatable launch `--bind PATH` supports `schema: task` and `schema: delivery`, independent of filename. Binding is assignment and updates frontmatter Agent information as part of launch. Support multiple bindings per Agent and multiple participants per file without silently replacing an assignee. Preserve body, unknown metadata and formatting as far as feasible. Ordinary Agent work updates status directly in files; platform never manages work transitions.
- File is the sole durable source. No task/binding DB tables, columns, or hidden durable binding index. Daemon-owned memory is disposable and rebuildable from bounded workspace `.controller` discovery. Paths have explicit caller-relative resolution. Distinguish exact participant identity from reusable names; use existing native identities. Preserve compatibility with existing scalar `agent` metadata and document the chosen minimal additive assignment representation.
- Small cohesive reusable file reader/binder library: recognize schema, description, raw status, normalized status, declared relations and assignment. Tolerate unrelated/unknown fields. Preserve unknown status words. Normalize casing and separators; provide generous explicit aliases. `planning` is ongoing work, `planned` is not started. Do not conflate paused, failed, blocked or cancelled. Keep document state separate from missing/unreadable/malformed-source diagnostics and freshness.
- Initial/discovery and binding refresh; useful Agent lifecycle refresh; explicit refresh/query entry. Avoid per-tool-call scans, synchronous filesystem work in a DB write transaction, broad disk scanning, or file watchers. Coalesce refresh if necessary. A retained prior successful snapshot must be distinguishable from a current read failure.
- Expose associations/status through actual native/CLI query consumers and existing management projection where appropriate; do not stop at a disconnected helper library. Delivery links may be shown but do not recursively assign children or infer Delivery acceptance. Keep `work` unchanged.
- Define errors honestly: failed file writes are failed assignments, not successful binds. Validate before avoidable launch effects. If launch fails after writes, provide a coherent rollback or explicit recoverable outcome. Document supported local/remote launch boundary rather than silently ignoring flags.

## Ownership and completion

Coder may edit the affected Rust/module/CLI contracts, scoped tests, dependency declarations only if needed, usage docs, and this Task plus a concise linked design/working record under this directory. Root AGENTS.md and Delivery are controller-owned; ask for a brief shared-file handoff only if necessary. Record ambiguities and their dispositions in the design/working record. Read existing frontmatter schemas rather than inventing an unrelated format. Avoid broad Web redesign; consume existing projections with the smallest necessary adaptation.

Verify meaningful scenarios: real frontmatter variants and aliases, non-destructive multi-agent writes and deduplication, unknown/malformed/missing files, restart/index rebuild, name reuse, cwd/path behavior, launch write failure, actual launch/query integration in disposable resources. Use `just --list` and project recipes. Run relevant checks once after coherent edits; repeat only for affected failures/changes. Complete `just package` and version check if applicable without replacing installed/default binaries or restarting user services. Return implementation paths, usage, executed evidence, material limits and cleanup facts. Do not claim support from unexecuted tests.

## Runtime binding

Fresh coder naga, role `coder`, profile `codex-gpt-6-1-sol-high`, batch `2b122cab`, runtime ready. Controller vase owns coordination; naga owns this cohesive implementation and its verification. Assignment continues through ordinary integration fixes and verification; no per-phase redispatch is needed.

## Delivered local result

The cohesive implementation is usable through fresh local launch, native query/CLI and existing management HTTP projections. [Usage and exact limits](../../.docs/file-task-bindings.md), [module ownership](../../src/file_bindings/AGENTS.md) and [design/decisions](design.md) are the maintenance entry points. Delivery acceptance remains with vase; this Task closes its authorized implementation and scoped verification.

| Responsibility | Implemented source |
| --- | --- |
| YAML reader, raw/normalized work state, relations and exact file assignment | `src/file_bindings/document.rs` |
| Non-destructive additive writes, locks, preflight and assignment-only compensation | `src/file_bindings/write.rs` |
| Bounded discovery, disposable memory and current-read versus retained-success observations | `src/file_bindings/index.rs` |
| Native reservation-to-spawn integration and unsupported-path rejection | `src/runtime/{launcher,launch_options,resume_client}.rs`, `src/cli/commands/{launch,resume}.rs`, additive default at `src/relay/control_host.rs` |
| Application-owned index, useful lifecycle refresh and native query | `src/service/{application,queries,queries_store}.rs` |
| Actual CLI query/help/opaque-value registration | `src/cli/commands/bindings.rs`, command/router/help registration and `src/runtime/environment.rs` |
| Existing management projection | Additive `FileBindingView` / `AgentView.file_bindings` in `src/management/contract.rs`, empty store initialization, application enrichment; office owner regenerated native TS while preserving fields |
| Dependency / source discovery | `serde_yaml_ng 0.10.0` plus `unsafe-libyaml` lock entries; `src/main.rs`, `src/README.md` |

No task/binding DB tables, columns, KV index, work integration, watcher, live service restart, installed/default executable replacement or unrelated repair was introduced. All substantial pre-existing/concurrent edits remain in place.

## Executed evidence

| Check | Executed result | Evidence |
| --- | --- | --- |
| `just test --bin agentx file_bindings::` | 10 passed; 0 failed; 3278 unrelated tests filtered | [Native log](evidence/native-tests-r1.log) |
| `just package .bin/agentx-file-bindings` with established functional profile overrides | Passed, candidate reports `agentx 0.7.31`; 127.4 seconds | [Package receipt](evidence/package-r1-receipt.json), [log](evidence/package-r1.log) |
| `python3 tests/file_task_bindings.py --binary .bin/agentx-file-bindings ...` | 12 scenarios passed against the packaged SHA | [CLI/HTTP cases and command receipts](evidence/cli-cases-r2.json), [run receipt](evidence/cli-r2-receipt.json), [concise log](evidence/cli-r2.log) |
| Public help discovery and launch/query flags | Root help, launch --help, bindings --help and recursive bindings help passed | [Help receipts](evidence/help-checks.json) |
| Exact artifact and cleanup | SHA matches tested binary; disposable fixture removed and no matching scoped processes survive | [Outcome](evidence/outcome.json) |

Native evidence covers aliases and unknown values, frontmatter variants, scalar compatibility, multi-agent deduplication, eight simultaneous writers, compensation with intervening edits, flow-root rewriting, symlink scope, missing/malformed/busy reads, retained success and disposable rebuild. Executed CLI/HTTP scenarios cover caller-relative paths versus child cwd, dry-run, unsupported local/remote/stdio/resume paths, native reservation/launch and explicit query, two assignees, management HTTP projection, real lifecycle refresh, disposable daemon restart, malformed/missing recovery, synchronous launch failure, actual write failure after preflight with multi-file compensation, absent binding DB/sidecar state and same-name native reincarnation isolation.

## Attempts, scope and limitations

The first crate attempts were blocked by another team's in-flight office-management skeleton/Web errors. Those files were not repaired here; coordination with their owners preserved additive binding projections. After their fix, scoped native compilation/tests and package succeeded. Initial CLI attempt r1 stopped on a test expectation: Clap correctly rejected an opaque `--env-file` token as an unexpected argument, rather than naming the missing `--bind` value. The assertion was corrected to the actual required no-dotenv-consumption behavior; r2 passed against unchanged product binary. [Original failing run](evidence/cli-r1.log) remains auditable.

The packaged candidate uses `CARGO_PROFILE_RELEASE_LTO=false`, `CARGO_PROFILE_RELEASE_CODEGEN_UNITS=256`, `CARGO_PROFILE_RELEASE_OPT_LEVEL=0`; default optimized release behavior is not established by this functional build. Execution covers Linux local fake-Provider/native hooks, not paid Provider interoperability, Windows/macOS or live user services. Fresh local terminal/PTY/background launches support binding; remote, stdio/shared app-server, resume/fork reject it. External files outside `.controller` require explicit paths after restart. File writes use advisory inode locks with compensating multi-file rollback, not crash-atomic transactions; incomplete restoration/rollback is actionable and explicitly reported. File declarations grant no runtime control authority.

The unique candidate `.bin/agentx-file-bindings` remains for controller consumption. All integration fixture processes/files were removed; no protected demo or user service was mutated. No commit was made.
