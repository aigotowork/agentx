# Plugin task

Read AGENTS.md and design.md here, plus root/plugin guidance. Implement lifecycle split ONLY in src/integrations/plugins/pi/agentx.ts, omp/agentx.ts, opencode/agentx.ts. You may add tests under tests/agent_context_lifecycle/plugins* and append implementation notes here. Do not edit Rust/config, old tests, or other providers. Existing acceptance test author will independently verify.

Rust owner will supply start JSON {name,session_id,instructions,context,bootstrap}; compact JSON {instructions,context}, current-session fenced. instructions is frozen; context is formatted dynamic snapshot. Old bootstrap fallback is allowed for compatibility when fields are missing, not when explicitly empty. Separate fixed instructions and current dynamic snapshot in plugin state. Use native additional instruction APIs for fixed text when actually supported; otherwise carry frozen text in existing hidden context channel. Never replace host base prompt, enqueue a user message, alter mailbox receipts or status during context refresh. Keep reply generation/session fences, and prevent concurrent late compact replies replacing newer snapshots. Read actual installed SDK types before deciding event channels. Keep no-op/empty refresh behavior explicit and no stale bootstrap leakage. Run TypeScript check (no Cargo) once changes are coherent. Return evidence and limits, no historical controller messaging.

## Plugin implementation plan

Owner: assigned plugin worker (this session). Scope remains the three named adapters and new plugin tests. Reuse the selected lifecycle model: one frozen instruction string, one replaceable dynamic string, and a monotonic compact request sequence inside each existing binding generation. Explicit empty values clear dynamic state; only a legacy reply with both split fields absent may use combined bootstrap. A failed refresh clears dynamic state but retains fixed instructions. Foreign sessions and superseded replies have no effects.

SDK evidence: Pi 1.1.0 exposes mutable `systemPromptOptions.appendSystemPrompt` in `before_agent_start`; OMP 18.10.2 offers only full `systemPrompt` replacement, so fixed instructions use its hidden context channel. OpenCode 1.19.0 exposes `experimental.chat.system.transform` and additional compaction context; OpenCode 2.0.27 exposes mutable `SessionContext.system` / `SessionCompaction.system` arrays. Preserve all existing base prompt entries. Dynamic Pi/OMP messages are request-local hidden custom messages; OpenCode 1 uses a replaceable synthetic part on an existing user message, and OpenCode 2 appends to request-local context. No context refresh calls mailbox/status operations.

Verification cases: split initial fields; explicit empty versus legacy missing fields; fixed text survives refresh/failure; replacement removes stale dynamic state; repeated request projection; foreign/reset session fencing; reverse-order compact replies; base system preservation and no new user turns. Native executable/provider acceptance remains with the independent verifier.

## CHECKPOINT_RELEASED

Status: CHECKPOINT_RELEASED. Released on user instruction; the AgentX successor owns continuation. No Paseo orchestration was used. No further implementation, tests, builds, or review were performed after the release instruction. All three named plugin files and the new plugin test file are released for successor edits. This is a checkpoint, not final acceptance.

Current implementation:

- `src/integrations/plugins/pi/agentx.ts`: split frozen `instructionsText` and replaceable `contextText`; native append via `systemPromptOptions.appendSystemPrompt`, with hidden-context fallback when an existing forced system prompt prevents append; request-local hidden dynamic projection replaces prior owned bootstrap/recovery/context messages. Compact refresh uses generation, session identity and request revision fences. Explicit empty snapshots clear; refresh failures keep frozen instructions and clear dynamic text.
- `src/integrations/plugins/omp/agentx.ts`: same split and compact fencing; both frozen instructions and dynamic snapshot use request-local hidden context because the installed extension API offers only full system-prompt replacement. Existing main-agent guard remains.
- `src/integrations/plugins/opencode/agentx.ts`: split state; OpenCode 1 appends fixed text through `experimental.chat.system.transform`, replacing only its own marked dynamic synthetic part on existing user messages. OpenCode 2 appends fixed/dynamic text through native request context system arrays. Both compact paths restore fixed text with fresh dynamic content. Generation/revision/disposal checks reject stale responses. Startup rejects explicitly mismatched returned session IDs. Legacy combined bootstrap is used only when both split fields are absent; explicit empty split fields never fall back.
- `tests/agent_context_lifecycle/plugins.mjs`: new source-only mocked RPC/event harness; covers Pi, OMP, OpenCode 1 and OpenCode 2. No Rust/native/config/old test files were edited by this worker. Pre-existing changes throughout the workspace must be preserved.

Executed evidence before release:

| Command | Result | Scope |
| --- | --- | --- |
| `npm run typecheck` | PASS, exit 0 | Installed SDK compatibility for the configured TypeScript project; no source changes after this check, only new harness coverage |
| `npm run test:plugins` | PASS, 27/27, exit 0 | Existing plugin mailbox admission, receipt and OpenCode lifecycle regressions |
| `node tests/agent_context_lifecycle/plugins.mjs` | PASS, exit 0, latest run includes all four plugin paths | Mocked lifecycle projection/refresh, empty/failure behavior, reverse-order replies, session/disposal fences and base-prompt preservation; Pi/OMP reset and legacy fallback; no live provider/native acceptance |

The first focused harness run covered three paths; a second run was justified by adding distinct OpenCode 2 coverage and passed all four paths. Typecheck and existing regression suite were each run once. No Cargo/build was run. The harness creates and removes its own temporary directory.

Remaining work for AgentX successor:

1. Inspect the current source diff and this checkpoint against `design.md`; implementation is present but no final handoff/acceptance claim has been made.
2. Native integration and independent acceptance remain with the native owner/verifier. The current tests mock RPC and provider events, so they do not establish actual provider compaction execution or end-to-end persisted instruction recovery.
3. Optional targeted coverage gaps to assess before adding checks: OpenCode 1 explicit-empty/legacy startup and same-ID reset paths; Pi's forced-system-prompt fallback is implemented but not directly covered by the new harness. Avoid rerunning already passed checks without a relevant change or requirement.
4. Document final accepted outcome and evidence after successor review/integration. Preserve the user's correction: use AgentX for any continuation/orchestration, not Paseo.
