---
schema: task
status: completed
agent: gina
description: Independently establish native Codex app-server activation and bridge feasibility.
links: [./task.md, ../../.docs/codex-activation/README.md]
---

# Assignment

The user requires agentx to retain its autonomous message/parent/subagent activation capability when a generic client launches `agentx codex --stdio --go -- app-server`. Paseo is only one outer client. Its agent identities and scheduling must not become prerequisites for agentx scheduling. The parent is designing the shared activation contracts and state machine; your scope is the independent native protocol feasibility investigation.

Read root AGENTS.md and relevant guidance, `src/README.md#native-codex-stdio`, `src/runtime/codex_stdio.rs`, and the native local schemas already generated at `/tmp/zomi-codex-wake-schema`. Official OpenAI documentation retrieval directly returned HTTP 403 in this session; the parent is retrying through the configured proxy. Use exact installed native versions and declarations. Do not assume a method exists solely because it is plausible.

# Questions to resolve

1. Can an agentx-owned protocol bridge be the sole writer/reader to native Codex while forwarding an outer client's app-server protocol and issuing its own `turn/start` on a bound thread? Distinguish schema support from exercised behavior.
2. What response/event ordering, thread status, busy admission and hooks can safely arbitrate an external user turn versus an internal wake? Are `UserPromptSubmit`, `PostToolUse` and `Stop` hooks meaningful for app-server turns?
3. Does installed Codex offer a supported independent native control channel that is simpler than a stdio bridge and operates on the same live thread/process? Examine `app-server daemon/proxy`, WebSocket or Unix support without starting an extra production daemon or attaching to user sessions.
4. Must the bridge route server-initiated approval/tool requests and remap arbitrary string/number request ids? What would prevent native protocol transparency?
5. If safe and useful, make an isolated no-paid-model probe using temporary CODEX_HOME/project state, no inherited credentials and a loopback fake model provider on an OS-assigned non-default port. Native model calls must target only that fixture. Do not submit any request to a real model endpoint. A local fixed SSE response can test native turn lifecycle without model service use. If this is disproportionate or the fixture cannot be established safely, preserve the uncertainty instead of making unsupported claims.

# Scope and outputs

Write only `./.docs/codex-activation/protocol-feasibility.md` and, if needed, a focused disposable/reproducible probe under `./.docs/codex-activation/.tools/`. Do not edit parent-owned design/contracts, production source, installed tools, user/project configuration, credentials or Paseo. Do not spawn additional agents. Reuse current dependencies and Python standard library when possible. Temporary data belongs under `/tmp`; do not print secrets, full config or environment. Preserve unrelated edits.

Your report must include installed version/artifact identity, exact evidence sources and commands, observations versus assumptions, native control-channel comparison, compatibility/failure risks, a technically supported recommendation and remaining integration checks. Link any reproducible probe. Keep the primary result in the file and report its path and conclusion to zomi using agentx when finished, or in your final response if that route is unavailable. All agentx commands must use your assigned agent name. Do not report schema-only or fake-server checks as full end-to-end production proof.

# Delivered result

Completed on 2026-11-06. The [independent report](../../.docs/codex-activation/protocol-feasibility.md) and [reproducible probe](../../.docs/codex-activation/.tools/probe_native_protocol.py) retain the native artifact identity, scoped observations, commands and limits. A final targeted valid-steer case confirms same-turn delivery and clientId correlation without rerunning passed full scenarios. The parent integrated these observations into the proposed contracts; production bridge and unmodified-client end-to-end behavior remain explicitly unverified.
