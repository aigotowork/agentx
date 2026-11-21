# Bounded work delivery

Read the [native Task](../../.controller/collaboration-workbench/tasks/collaboration-service.md), [capability meaning](../../.docs/collaboration-workbench/capabilities.md) and [native usage](../../.docs/collaboration-workbench/native.md). Rust declarations in `service.rs` are canonical; `web/api.rs` closes the operator-only browser surface and `web/workbench/api/derive.py` derives that closure.

| Owner | Responsibility / dependency |
| --- | --- |
| `service.rs` | Native values, finite operations, bounds and failure/effect meaning; no HTTP or storage policy. |
| `model.rs` | Private aggregate transitions and eligibility; no DB, Provider or adapter imports. |
| `store.rs` | Daemon-only composition of pure rules, exact participant/actor validation, persistence and existing native message admission in one write scope. |
| `web/` | Fixed project/scope/mode entry and finite HTTP mapping through typed Client; never opens DB or exposes report impersonation. |
| `../db/collaboration.rs` | Aggregate and command receipt SQL only, including additive schema/reset. |
| `../cli/commands/work.rs` | Actor-bound report grammar and finite reads; no domain rules. |

Create retains valid distinct initial roles without sending. Dispatch/check/answer/change/note commit message admission with the aggregate and receipt. Reports are explicit exact-assignment statements, never runtime inference. Acceptance closes writes. Reported checks are attributed evidence; they do not certify current filesystem bytes. Keep historical evidence, generation fences, command recovery and native capacity guards.

Independent acceptance alone executes tools/tests/builds. Authored cases in `cases.md` and Rust tests are unexecuted until its actual receipt. No live demonstration or installed binary ownership belongs here.

Workbench launch defaults and local acquisition are owned in `web/mod.rs::launch_paths` and `run`: omitted scope follows normal originating `AGENTX_DIR`/home resolution, omitted project is invoking cwd, explicit paths resolve from that cwd, and both local modes acquire a missing native daemon once. Every HTTP call still uses existing service in the fixed canonical scope and actor-cleared invocation; finite writes require `--collaboration`. [Native usage](../../.docs/collaboration-workbench/native.md#workbench-startup-and-defaults) and [startup Task](../../.controller/domain-daemon/tasks/startup-native.md) carry the bounded change and independent evidence route.

`web/openapi.rs` derives the separate workbench OpenAPI from the real connection/read/apply HTTP bindings. `web/api.rs` remains the public input closure, reusing native Read and OperatorCommand; no AgentReport input or generic CommandCall is exported. Native reachable types derive ToSchema without changing serde. Shared `browser/openapi.rs` projects every native integer to the actual `$integer` wire object, retaining source width metadata for SDK bigint mapping. Offline export uses `agentx __export-workbench-openapi`; see [three-listener projection guide](../../.docs/api-panorama/openapi/README.md) and [owned HTTP Task](../../.controller/contract-service-projections/tasks/full-http.task.md) for current evidence. Source authoring alone does not establish full listener/client acceptance.

Current browser access and embedding use [shared open-origin policy](../../.docs/api-panorama/openapi/README.md#browser-access-policy) through `browser/admission.rs`; [the native Task](../../.controller/web-sdk-adoption/http.task.md) owns source/acceptance status. OPTIONS is side-effect-free. Every actual route retains native authority, identity, input/size bounds and unknown-outcome recovery.
