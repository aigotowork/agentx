---
schema: delivery
description: Configurable shared context generation and compaction injection.
status: completed
stages:
  implementation: {description: Shared composition and provider adapters}
  acceptance: {description: Independent bounded verification}
---

# Agent context delivery

User authorized implementation on 2026-11-14. [Proposal](../../.docs/proposals/agent-context.md) owns semantics. Mika owns implementation; a new Paseo verifier owns independent cases and execution. Preserve all concurrent checkout edits and live daemons. Source and disposable binary acceptance do not authorize service replacement.

## Selected first slice

- `context.sources` TOML array; `context_sources` CLI JSON / `AGENTX_CONTEXT_SOURCES` JSON. Absent/empty-string selector keeps legacy bootstrap; explicit `[]` disables AgentX context. Sources have unique IDs and exactly one `builtin`, `text`, `file` field. Builtin ID `agentx` only initially.
- Typed internal generator interface receives reason and current participant facts. Ordered composition, literal files/text, bounded UTF-8 file reads and aggregate 64 KiB limit. Current caller cwd resolves relative files. Errors fail generation without fallback. Existing notes/exclusions belong only to the builtin.
- Bootstrap and compact share the composer. Codex compact SessionStart must preserve mid-turn status and actual thread attachment. Claude/Qoder recover via existing native hooks. OpenCode/Pi-family get fresh context via native plugin RPC, not cached text. Observation-only hooks are not advertised as postcompact delivery.
- External command hooks deferred. Do not replace existing developer instructions or execute shell templates.

## Verification handoff

Verifier writes only new files under tests/agent_context/ and this directory's acceptance.md; coordinate any suggested source changes with implementer. Build output /tmp/agentx-context-candidate (implementer builds after edits). Do not concurrently launch Cargo builds before candidate handoff. Early work: design independent black-box cases and inspect native Codex/Claude feasibility. Test config default/replacement/empty, ordering, changed file, invalid source/file/oversize, compact refresh with independent thread/state fencing, native developer instruction retention where feasible. Do not infer native execution from a hook fixture. No changes to installed tools, live daemon scopes, credentials or user sessions.

## Progress

Design accepted and source implemented. Public contract is src/runtime/agent_context.rs; config integrates the native key registry. Codex/Claude/Qoder and OpenCode/Pi/OMP adapters are connected. Cargo check, plugin typecheck and scoped formatting passed. Final candidate package passed; independent binary/native verification is accepted for the documented Linux local scope. Verifier: Paseo srv_yDbm9zuqIMiM / 2c68046a-9134-4993-b64c-22a9c00d281c.

Build note: the initial default release/LTO build was explicitly stopped because subsequent source repairs made it stale and LTO dominated delivery time. Final local candidate uses `CARGO_PROFILE_RELEASE_LTO=false CARGO_PROFILE_RELEASE_OPT_LEVEL=1 CARGO_PROFILE_RELEASE_CODEGEN_UNITS=16 just package /tmp/agentx-context-candidate`. This is a local behavior-acceptance build, not default-profile performance or distribution evidence.

Independent early result: the isolated composer crate passed all three scenario tests; Pi/OMP/OpenCode v1 mocked-RPC event cases passed. [Acceptance](acceptance.md) owns commands/receipts and remaining native/binary scope. No further plugin behavior edits followed this run.

Final strict Clippy attempt (`cargo clippy --all-targets --locked -- -D warnings`) reported existing/concurrent diagnostics in lifecycle_service, cli/router, runtime/hosts, runtime/launcher, daemon/transport and relay/control_host; none was in this feature's changed context/composition/provider code. Full log: `/tmp/agentx-context-clippy.log`. Do not describe this check as passed. The final candidate package is built separately after that diagnostic run.

Final candidate handoff: `/tmp/agentx-context-candidate`, AgentX 0.7.31, SHA-256 `245ad1f22467ac3978002b4c309231b628aa0ef1b23f9c1f9f85a2da1a4784bc`. Built after the final Codex home/profile preservation change; package log `/tmp/agentx-context-package-verified.log`. The verifier is explicitly authorized to run the binary and native hook cases. No installed binary or running user service was replaced.

Candidate CLI/launch verification: all 20 cases passed, including replacement/disable/environment precedence, fresh relative files, invalid sources, size bounds, native home/profile retention and explicit argv precedence. Exact independent receipt: [candidate launch log](../../tests/agent_context/evidence/candidate-launch-corrected.log). Hook/native compact checks remain with the verifier.

Candidate hook verification: independent Codex and Claude repeated compact refreshes passed with active state preserved. Ordinary Codex missing/empty/wrong/retired session guards passed without context output or rebinding. Receipt: [candidate hooks](../../tests/agent_context/evidence/candidate-hooks.log). Native model transport compact and shared-sibling cases are separate evidence.

Native Codex 0.167.0 manual and automatic compact runs passed against a deterministic local protocol fixture: fresh context and retained native developer instructions appeared in subsequent real native requests, with exactly the requested user messages. Evidence: [manual](../../tests/agent_context/evidence/native-manual-correlated.log), [automatic](../../tests/agent_context/evidence/native-auto.log). Five configuration persistence/rejection cases also passed: [config](../../tests/agent_context/evidence/candidate-config.log). These establish native request placement rather than hosted-model semantic compliance.

## Accepted outcome

Completed and independently [accepted](acceptance.md#verdict) for the documented Linux local scope. Shared context sources and typed generation callbacks now serve bootstrap and supported compact adapters. Candidate CLI/config, ordinary Codex/Claude hooks and actual Codex manual/automatic request construction passed; plugin evidence is mock-backed and other unexecuted Provider/shared-thread/platform limits remain listed in acceptance. External command execution is intentionally deferred. Installed programs and user services remain unchanged.
