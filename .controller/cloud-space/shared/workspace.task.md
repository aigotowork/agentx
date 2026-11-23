---
schema: task
status: active
agents: [{"scope":"/home/long/works/longwork/agentx/.agentx","generation":"a6195a5b-1dd4-4a6d-87d4-1f18f1b664fb","participant":{"canonical_name":"mamuva","incarnation_bits":4745317378358057615}},{"scope":"/home/long/works/longwork/agentx/.agentx","generation":"a6195a5b-1dd4-4a6d-87d4-1f18f1b664fb","participant":{"canonical_name":"laloza","incarnation_bits":4745317378419909487}}]
---
# Online workspace registration
Cloud scope identifies the shared namespace. Local runtime scope selects the daemon bound to that URL. Existing exports (default = connect cwd) are the allowed parent roots. No per-project launch-time enrollment. Allow an operator to register an existing project or create a missing leaf below a published host root. Keep project membership and launch cwd distinct. Reuse exact admin control receipts. Choose command {kind:"add_workspace", parent:string, directory:string, alias:string, create_directory:bool}. Succeeded workspace outcome {status:"workspace_added", alias:string}; receipt.status remains succeeded. HostCapabilities.workspace_management optional boolean gated by host support. Verify restart persistence, alias conflict, confinement and stale host. Use the same Space launch dialog and explicitly select newly added project. No new daemon or token.

## Deployment control design

The [shared wire contract](../../cloud-daemon/wire.md) owns the request headers, command and receipt. [Control HTTP](../../../src/cloud/control/http.rs) reads the optional feature and configured root advertisements on the existing scope lease PUT. The [control store](../../../src/cloud/control/store.rs) stores the bit on the exact lease and derives the read-only `managed_parent` catalogue marker from the root aliases supplied in the header. Admission and fresh claims check that the lease supports registration, the published parent is marked, and the new alias is unused. Local path and original export authority stay with the host, since the deployment sees aliases only. The host publishes the new alias in its lease catalogue before reporting success; receipt validation checks that alias against both the original command and the current catalogue. Exact replay uses the existing request and claim fences.

Keep the existing lease query for directory launch and the existing launch/stop DTOs. A separate endpoint would add a compatibility boundary without changing the effect. The optional headers let a new host keep renewing against an old deployment, which rejects unknown fields nested in the JSON catalogue.

## Host contribution

The uplink resolves registration only from a connect-configured alias in its last accepted catalogue. It uses the same descriptor-based directory walk as launch, permits creating only the final component, and stores the selected parent alias, canonical parent and relative directory with the exact operation. The private control journal commits the registration and successful outcome together under the current deployment generation and installation. Recovery never repeats directory creation; an interruption before the commit can leave a directory without registering an alias. The next renewal publishes the updated catalogue before the worker reports success; after restart the initial renewal does the same. Registered directories are rechecked before advertising or launch and are never allowed parents.

The [control wire](../../cloud-daemon/wire.md) distinguishes eligible configured roots via `managed_parent`. The host advertises them in the renew headers, leaving the strict JSON body compatible with earlier deployments, and rechecks their eligibility at execution.

## Consumer and end-to-end evidence

The [operator guide](../../../.docs/architecture/cloud-daemon.md#在云端添加项目工作区) describes the Space and HTTP CLI entry points. The [shared HTTPS fixture](../../../tests/cloud_scope/shared_workbench.py) and its browser script created one new project, registered a second existing project, launched a real Codex supervisor with a role defined only in the second project, verified its subdirectory via procfs, and recovered both projects on host restart. The evidence is in `/tmp/cloud-workspace-accept-1/result.json`. The SDK's receipt test checks mismatched aliases, paths, creation flags and command kinds.
