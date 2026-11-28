---
scope: Local Domain SDK, Space and native Agent Info contracts
status: completed
---

# One current local read contract

This is the implementation follow-up to the [compatibility inventory](compatibility-retirement.md#follow-up-order). The selected scope is duplicate application read paths. No installed or live instance is selected for replacement by this task.

| Boundary | Current contract and responsibility |
| --- | --- |
| Native Management | Owns exact member selection before pagination, latest/older message selection, and the live watermark. Existing descriptor flags declare the minimum contract; do not add another protocol or change the HTTP schema. |
| Domain SDK | `selectAgentXScope` requires both native member and history capabilities at admission and re-verification. A missing capability is an upgrade error. `Workspace.memberPage` is the sole workspace membership read; `scope.agents` has a separate purpose, the scope-wide overview including unassigned and historical records. Cursor fences remain in the SDK. |
| Space | Selected offices use native member pages. Each visible exact conversation loads latest, independently prepends older pages and polls forward only from the live watermark. Parking and restoring preserve exact history and viewport. Cloud and local adapters produce a single presentation state; their different wire identities remain at the adapter boundary. |
| Native TUI | Use only the daemon-supplied Info reference and its exact AgentInfo query. A daemon without the reference can still supply dashboard rows; the TUI displays the requirement for a current daemon rather than deriving an address. Remote and genuinely unavailable exact data remain distinct. |
| Space URLs and contract Canvas | Publish only `channel=messages\|model`; reject `notes` and treat `layout` as an unrelated query key. Remove the retired SDK projection from the Canvas model, static entry and verification routes. |

Do not infer persisted historical membership, rebind receipts, or change dataset/process fences.

## Sequencing the storage, config and peer work

1. Inventory schema versions and required objects on read-only snapshots of supported scopes. Separate the storage-owned upgrader from the daemon's ordinary database open. A fresh file bootstraps head, a complete head opens normally, and any other file gets an actionable refusal. An explicit offline upgrade should acquire the scope's existing owner lock, transform inside a transaction, and preserve database generation, events, receipts and exact participant references. Verify old, partially stamped and future schemas before retiring startup repair. Keep lifecycle recovery for already-running owners.
2. Define one persisted input per configuration responsibility: named profiles for launch arguments and explicit instruction/runtime sources. Produce a deterministic one-time conversion for legacy inputs, with clear precedence for already-explicit values and proof that argv, ordering, and frozen instructions stay equivalent. Keep this out of ordinary status/read paths. Resolve caller-provided legacy environment keys and deployed configuration before removing their old interpretation.
3. Inventory deployed Relay peer versions and Provider capabilities. Retire obsolete negotiation only after the current contracts are present across the selected cohort; retain exact process, delivery and receipt fences. These are independent rollout decisions and do not belong in the application read adapters.

## Verification

The locked native build passed, along with 36 SDK tests and 19 Space reader, exact relation, transcript and route tests. The [Contract Canvas checker](../../.docs/object-atlas-v2/model/check.mjs) verified source anchors, static wrapper equality and 52 navigation paths. Real Chrome passed the [conversation workbench](/tmp/agentx-local-read-browser.XImuoR/focused.json), [workspace](/tmp/agentx-local-read-workspace.FEE1Pl/report.json) and Cloud Space fixtures. The [tmux reader](/tmp/agentx-local-read-info.puJxqh) verified adopted Role/Profile, frozen Bootstrap and Task, exact current/stopped relationships, and the same daemon before and after.

The [native HTTP and Chrome run](/tmp/agentx-local-read-native.7xpeNb/result.json) used a disposable daemon with a local Provider stand-in. It confirmed native member selection and distinct latest/older cursors, opened the exact stopped record in the embedded Space using only the canonical reads, and verified that a descriptor missing history produced an upgrade message and no resource traffic.
