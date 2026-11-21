# Credential-free browser adapter

Read the [LAN follow-up](../../../.controller/observability/lan-usability.md) for the credential-free LAN launch contract; it supersedes earlier bootstrap/loopback restrictions. Read the [root entry](../../../AGENTS.md), [native observation guide](../AGENTS.md), [original Task](../../../.controller/observability/implementation-tasks/local-adapter.md) and [browser contract](../../../.docs/observability/browser-api.md). This module owns only explicit launch, HTTP framing/CORS, finite native read mapping and safe static bundle delivery. Native contracts and daemon authentication retain authority; never deserialize an InspectionRequest, RuntimeRequest or business request from the browser.

| Home | Responsibility |
| --- | --- |
| `mod.rs` | Passive launch/HTTP route, HTTP framing, open-origin CORS, bounded connection/native lifetime and cancellation |
| `api.rs` | Closed read operations, selected scope fences and native leaf conversion |
| `../../browser/{wire.rs,admission.rs}` | Shared lossless integer/strict JSON and HTTP framing and open-origin CORS mechanics; route/policy stays here |
| `assets.rs` / `../../browser/assets.rs` | Diagnostic embedding selection / shared preloaded finite manifest; no browser path-to-file mapping |
| `tests.rs` | Owner-authored meaningful state/browser-access/failure cases; independent consumer executes |
| `../../../../web/observability/api/` | Native-derived browser types and handwritten transport client; views stay Web-owned |

Keep native errors, references, evidence clocks, loss, coverage and unavailable/stale results exact. Update the contract and regenerate the browser closure when public fields change. `derive.py` reads native sources without compiling; unsupported syntax/custom serde requires explicit adaptation before emission. Never hand-edit native.ts, introduce arbitrary report/mutation/file/environment tunnels, or add a success page in this module. Compiler/formatter/test/package/browser execution belongs exclusively to the assigned consumer integration; no installed/live/default scope work.

`openapi.rs` derives the separate observation OpenAPI from real scope/read HTTP bindings and closed ReadCall/BrowserRead/ReadResult types. It exports through `agentx __export-observation-openapi` before environment/runtime startup; export never resolves or acquires a daemon. Native reachable declarations derive ToSchema, while shared `browser/openapi.rs` maps integer schemas to actual `$integer` objects and preserves source-width metadata. Custom UUID identities explicitly reuse their real string serializer schema. No InspectionRequest, runtime report or native mutation input is exported. See [projection guide](../../../.docs/api-panorama/openapi/README.md) and [HTTP Task](../../../.controller/contract-service-projections/tasks/full-http.task.md) for actual generation/integration status.

Current browser access and embedding policy supersedes historical origin/frame restrictions: see [shared policy](../../../.docs/api-panorama/openapi/README.md#browser-access-policy) and [active native Task](../../../.controller/web-sdk-adoption/http.task.md). Observation remains passive and scope-fixed; OPTIONS does not call native reads.
