# Agent management delivery

Read [design](../../.docs/agent-management/design.md), [Delivery](delivery.md) and the assigned Task before editing. User authorized design and implementation on 2026-11-15. The target is the existing AgentX management TUI's Web counterpart, not a PTY viewer.

Delivery is completed for the [accepted local scope](delivery.md#accepted-local-outcome). The table below retains original ownership for history; there are no active assignments. Maintenance follows the native and Web module guides plus affected cases.

| Original owner | Delivered scope |
| --- | --- |
| gabi | Native contract, journal/query store, HTTP adapter, daemon/CLI/build composition, causal native binding repairs and final integration |
| toku | `src/management/worker.rs` handoff completed; `src/management/store/tests.rs` and `src/agents/hook_store/session_tests.rs` native invariant cases, no build execution |
| fava | `web/management/` excluding generated `api/native.ts`; UX design and React implementation |
| pula | `tests/agent_management/`, acceptance evidence and identified repairs by handoff |

The shared working tree contains substantial prior user work. Do not reset, stash, overwrite unrelated changes, replace installed programs, restart current services or touch the protected demonstrations. Only integration runs Cargo and package builds. Use isolated scopes and a separately named candidate binary. Send handoffs through AgentX; do not delegate to existing unrelated agents.
