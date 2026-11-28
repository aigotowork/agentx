---
scope: AgentX repository compatibility paths
owner: AgentX maintenance
status: completed
---

# Compatibility retirement

Cloud v2 already rejects obsolete workspace IDs and the old cloud protocol. This follow-up removes branches whose old format supplies no current operation or authoritative data. Current codecs and UI owners are the sole entry points; historical receipts remain in Git. Source changes do not replace deployed binaries.

| Boundary | Decision |
| --- | --- |
| Relay pairing | Decode only the authenticated join format already emitted by `relay new`; reject old keyless invitations in the decoder. Stored peer configuration and MQTT transport are outside this change. |
| Native config | End silent deletion of the obsolete Claude and Copilot TOML delivery keys. The current loader already ignores them. Explicit config edits remain with the config owner. |
| Management selection | Restore only the exact selection saved with a matching generation. Selection is disposable UI state; original mutation receipts are independent. |
| Space appearance and components | Read the published `style` selector; use the canonical UIUX exports and remove internal forwarding files. Historical art review links no longer request a separate combination. |

## Follow-up order

The [local read contract delivery](local-contract-retirement.md) owns the subsequent Space/SDK and TUI source outcome. It also removes the Space `channel=notes` alias and stops interpreting the obsolete `layout` key. The table records the rationale at the time of this inventory; the config, database and peer decisions remain open.

| Priority | Compatibility still in source | Retirement decision |
| --- | --- | --- |
| 1 | The local [Space reader](../../web/agent-space/src/live/domain-office.ts) uses the global Agent scan when a Management listener omits native workspace membership, and switches to forward paging when it omits native history. The [Domain SDK](../../packages/agentx-client/src/domain/workspace.ts) exposes both member readers; its [example](../../packages/agentx-client/examples/scoped-send.mjs) still uses the projected one. The native [Management descriptor](../../src/management/service.rs) mirrors the selected daemon's support. | Make the new membership and backward history contract the minimum for current Space and its SDK. Update sample consumers, make older listeners report an explicit upgrade requirement, and remove the duplicate member reader, forward reader state, and matching UI. Keep ordinary forward message fetches where they serve current incremental updates. Verify on an upgraded local listener before adopting the new frontend. |
| 2 | The native [TUI data source](../../src/tui/db.rs) fabricates an Info reference only for snapshots from older native daemons; the [legacy Info reader](../../src/tui/info_data/legacy.rs) then reconstructs exact current or stopped records with extra requests. The [native transport](../../src/daemon/transport.rs) gates operations across builds. | Require the native Info reference and delete the reconstruction adapter when the CLI and local daemon have a common minimum contract. Keep scope, generation, exact reference, and OS process ownership checks, as well as explicit diagnostics for a refused peer. Resolve any existing managed hosts before removing recovery for the old daemon lifecycle. |
| 3 | User configuration still supports the [pre-split prompt source semantics](../../src/runtime/agent_context.rs) and [unpinned tool argument defaults](../../src/runtime/launch_profile.rs). Space can still interpret the old `channel=notes` alias and validate the no-op `layout` hint in the [route parser](../../web/agent-space/src/navigation/route.ts); the latter is still documented by [embedding](../../.docs/agent-management/web-context.md). | Convert existing settings to explicit instruction/context selectors and named profiles in one step before deleting config branches. Retire obsolete route values from current embedding examples and fixtures before removing the URL branches. |
| 4 | The [local database](../../src/db/mod.rs) upgrades and repairs older schemas in place; a version number alone does not prove that all historical migrations ran. Derived [message](../../src/db/conversation_routes.rs), [mailbox](../../src/db/mailbox_routes.rs) and [subscription](../../src/db/subscription_index.rs) indexes also have fallbacks when their coverage is incomplete. | First separate upgraders from ordinary startup, reject unsupported versions without archiving or writing, and verify the explicit upgrade on copies. Keep index fallbacks that protect current correctness until the index is a complete, verified prerequisite. Preserve events, receipts and unknown membership facts. |
| 5 | [Relay publish](../../src/relay/push.rs) still budgets for peers without packet limits, and [Relay control](../../src/relay/control.rs) reads peer capabilities. [Provider parsing](../../src/cloud/client/host.rs) also handles current Codex messages without a phase, independently of the Cloud version. | Set minimum peer and Provider contracts and measure actual traffic before cutting over. Reject unknown peer protocols or restart affected workers explicitly; keep Relay keys and exact delivery recovery. |

Rollback volumes and historical acceptance records are operational evidence. They are independent of the hot paths and do not justify adapters in the current runtime.

## Verification

All 24 targeted native tests passed, including the current Relay token round trips, rejection of the keyless formats, and exact preservation of the local config on reads. The fresh embedded Space and Management Web bundles passed their respective `cloud-space.browser.mjs` and `browser.mjs` fixture suites. Space overlay and Studio checks also passed. The rollout from [Cloud v2](cleanup.md#accepted-public-outcome-and-rollback) is separate.
