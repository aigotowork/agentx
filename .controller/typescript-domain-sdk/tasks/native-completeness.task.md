---
schema: task
status: completed
links: ["../../../.docs/proposals/typescript-domain-sdk.md#complete-the-object-model", "../../../.docs/model/typescript-domain-sdk-review.md", "contract-completeness.task.md"]
agents: [{"scope":"/home/long/works/longwork/agentx/.agentx","generation":"a6195a5b-1dd4-4a6d-87d4-1f18f1b664fb","participant":{"canonical_name":"hozira","incarnation_bits":4745316440240220710}}]
---

# Supply missing native membership and launch identity facts

Own the native responsibility for the full application path. A Workspace needs bounded pagination over its adopted members, and exact conversation navigation after launch is possible only when the launch itself supplies the new exact identity. The SDK cannot infer either fact by filtering a global page or querying by name. Trace current native execution and proofs first. Design and implement the narrow compatible native Service/OpenAPI contracts that make these guarantees possible. Work out the established owner for the filtering source, scoping/current/stopped semantics, explicit global versus no adoption, pagination order, and the exact launch provenance available at journal creation. Prefer additive behavior and preserve current route inputs/outputs and recoverability of older journal entries. If a premise does not hold, identify the missing native authority and evidence before designing around it; when reasonable keep independent native work moving.

Read the linked model and audit, native Management/service guides and actual HTTP chain. Own the affected native source, focused small native cases, upstream contract documentation and this Task. Coordinate with the TypeScript owner through Controller on the declaration before public binding; do not mutate generated SDK sources or run codegen before single ownership is arranged. Preserve unrelated source, existing history-performance changes and live services. No delegation. No load, large data, release build, install, or daemon/service changes. Author only tiny source fixtures and avoid native compilation until Controller coordinates one bounded check; evidence may initially be a source handoff plus exact commands.

Decisive cases include one matching member behind a few nonmembers at a tiny limit, valid global versus unsupported/legacy, selector-bound cursor, and a launch result that identifies the accepted instance even when its name is later reused. Older results without an exact identity must remain readable but cannot gain one by guessing.

## Native model and handoff declaration

[`checked_collaboration`](../../../src/agents/workspaces.rs) is the authority for adopted membership; the CLI/label selector intentionally treats missing membership as globally visible, while [`AdoptedWorkspaces`](../../../src/management/store.rs) is a capped discovery projection. The new read filters valid adoption before `LIMIT`, from local current rows and exact stopped event snapshots. In particular an explicit `workspace: null` inside a valid snapshot is global; missing, malformed, unsupported, remote, and neighboring-workspace rows are excluded. The key is literal `global` or the exact saved absolute path, validated without filesystem I/O or normalization against present files.

The additive declaration is `GET /api/v1/workspaces/members` (`managementListWorkspaceMembers`), with required `generation`, `workspace` and optional `after`, `limit`, `current_only` (defaults false). It returns `AgentPage`; current names ascend, then stopped event IDs descend, with `current_complete` and `next` scoped to matching members. The native opaque cursor binds dataset generation, workspace and current-only mode. The descriptor advertises the selected native peer's capability in additive `supports_workspace_members` (missing on an older listener). The [same-connection handshake](../../../src/daemon/transport.rs) rejects the new request as Unsupported before sending it to an older peer. Existing list, label and catalog routes keep their meanings.

[`launcher`](../../../src/runtime/launcher.rs) obtains the exact `Reservation.participant` from native reserve or reserve-resume; Management uses the non-stdio local path for launch, fork and resume. Keep exact successful reservations separately from public CLI/Relay display handles. The common operation result `Launched` retains existing `agents`, `failed` and `errors`, adding optional `agent_refs: AgentRef[]` with exact newly launched incarnations. Omission means the worker did not retain an exact result (including historical journal data); no name resolution, and no guarantee a successful launch's incarnation is still current when the receipt is read. Finish continues to record only the authorized worker's original result under its original operation identity.

## Source handoff and evidence

The assigned source and tiny cases are complete. Follow [consumer semantics](../../../.docs/api-panorama/service-contract-design.md#workspace-membership-and-launch-identity), [HTTP binding](../../../src/management/web.rs), [Service](../../../src/management/service.rs), [store](../../../src/management/store.rs), and [worker](../../../src/management/worker.rs). The existing current workspace expression index selects candidates, and the checked snapshot predicate and stopped-event selection run inside the bounded WAL read lane. Existing schema and persisted rows require no migration. Cursor pages observe the current dataset independently, and no scale measurements are claimed.

Source checks passed: scoped `git diff --check` and Rust syntax parsing via `rustfmt --edition 2024 --emit stdout --config skip_children=true` (stdout discarded). Four authored native cases share the `native_completeness` filter: member selection on a disposable WAL reader (explicit global, sparse local/stopped results, cursor scope), worker projection/journal recovery after name reuse and old result shape, old peer refusal before native Call, and emitted OpenAPI shape. These cases have not been compiled/executed; the launch case injects a successful reservation result and does not launch a Provider.

Controller kula owns the native build lane, actual emission and SDK generation. When the shared browser source is compatible with the pre-generation SDK, use the same approved isolated debug target for both commands below (replace `/absolute/path/to/isolated-target`). The native Cargo build invokes existing browser asset compilation through `build.rs`; do not build while concurrent source requires as-yet ungenerated types. After successful emission, the existing package generator consumes the canonical OpenAPI directory; keep ownership with kula and coordinate the package adapter with fele.

```sh
CARGO_TARGET_DIR=/absolute/path/to/isolated-target CARGO_BUILD_JOBS=1 cargo test --locked --bin agentx native_completeness -- --test-threads=1
CARGO_TARGET_DIR=/absolute/path/to/isolated-target CARGO_BUILD_JOBS=1 cargo build --locked --bin agentx
just management-openapi /absolute/path/to/isolated-target/debug/agentx > /absolute/path/to/isolated-target/management.json
```

No native build, codegen, release, load, installation or running service changes were performed for this handoff.
