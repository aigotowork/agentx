---
schema: task
status: completed
description: Simplify launch/config/runtime and CLI inputs
---

Read [entry](AGENTS.md) and [overall outcome](cleanup.task.md). You own `src/runtime/**`, `src/cli/**`, `src/config.rs`, `src/config/**`, `src/integrations/**`, `src/shared/constants.rs`, and `src/events/**`, including their tests and module docs. Completely remove obsolete Agent metadata: launch/resume/fork flags, env/config loading, injected guidance, process tracking, formatted name prefixes, dedicated group behavior, helpers and feature tests. Update native callers to contracts with obsolete fields/arguments removed by the identity owner. Preserve unrelated XML/HTML tags, AEAD authentication tags and terminal display-mode label semantics. Do not implement labels. Use another existing config value for generic precedence/environment tests. No files outside ownership, no Cargo/build checks during concurrent source edits, no commits/live service changes. Report changed signatures and integration needs to maho through AgentX. Work directly in shared checkout; use source-first removal with no deprecated aliases.

## Source handoff

suki source release 39259 and bali assigned integration/events release 39508; native runtime, config and CLI inputs are available for integration. Final executed guarantees belong to [verification](verification.md); source handoff does not imply live adoption.
