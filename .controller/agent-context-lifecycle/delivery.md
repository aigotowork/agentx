# Delivery

Status: active. User authorized full implementation across adapters. Mika: core interfaces, configuration, native persistence/hooks, docs and final build. Plugin worker: native TS plugin adapters. Verifier: independent lifecycle tests, candidate integration and acceptance. Current design is [design](design.md); previous [acceptance](../agent-context/acceptance.md) remains historical.

Candidate target: /tmp/agentx-context-lifecycle-candidate. Build owner Mika; use local non-LTO release overrides as in previous Delivery if necessary. Never replace installed binaries, project daemons or live user services.

Risks tracked: native roles differ; startup may precede session binding; same logical Agent resume requires retained fixed text; same-name different incarnation must not accidentally share state except exact native-session resume; old source lists must remain meaningful; hook receipt is not model-following evidence.

Historical bindings (released): plugin worker Paseo d28dd29a-15bc-4f41-8cdc-e400a5ca525c; independent verifier 095e734d-ba5a-44fc-9a50-57d227ab3f82, server srv_yDbm9zuqIMiM. The historical default profile was codex/gpt-6.1-sol, full-access, max. Initial all-target compile caught a concurrent missing model_request test initializer in agents/identity.rs; preserve owner changes and distinguish from this feature.

Orchestration correction: the user removed the external-platform requirement. The two provisional external workers released their files in plugins.md and acceptance.md. Their IDs above are historical cleanup evidence only. Continue through AgentX; no new external-platform migration workflow is required.

Implementation checkpoint: core ports/config/native KV persistence and all Provider entry points wired. Codex fixed launch content is separate from dynamic SessionStart/compact output. Pi/OMP/OpenCode worker reported TypeScript and 27 plugin regressions plus four-adapter lifecycle mock coverage passed. Core independent source tests: 3 composition and 5 store cases passed after fixing duplicate outstanding initial grants. Main binary check passed at the earlier snapshot; final candidate package in progress. All-target check previously hit concurrent model_request initializer drift.
