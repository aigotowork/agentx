---
schema: delivery
status: completed
description: Adopt the public generated HTTP SDK across appropriate existing Web consumers and enable cross-origin browser, extension and iframe use.
stages:
  complete:
    description: Cohesive implementation, concentrated browser/system verification, repair and current-support documentation.
    tasks: [./controller.task.md]
---

# Web consumers on one maintained SDK

## Background and actual foundation

The [accepted Service/OpenAPI/SDK/CLI delivery](../contract-service-projections/implementation.delivery.md) implemented three native-derived HTTP specifications, generated clients, browser-safe `@agentx/client` and separate Node `agentx-http`. [Package usage](../../packages/agentx-client/README.md) and [OpenAPI/codec policy](../../.docs/api-panorama/openapi/README.md) own current contracts. Its Linux local installed-package/native integration passed; it did not migrate existing Web applications, establish browser interaction acceptance, or change cross-origin admission. Historical instructions preserving same-origin behavior were a boundary of that completed delivery, not this user's desired future policy.

Existing `web/` applications have their own request/type/codec layers. The desired user outcome is one maintained contract-derived access foundation: Web features consume public SDK methods rather than maintaining duplicate wire DTOs, endpoint assembly, integer decoding, receipt semantics or transport error rules. UI-specific state, query/cache orchestration, presentation and application view mapping stay with Web owners. Do not replace existing scenes, chat, Inspector scope or user flows merely to adopt a client package.

## Explicit user choices

- The user requested a new Controller Agent to own task planning and advancement, with recoverable background/goals and execution handoff.
- The user explicitly says same-origin restrictions are unnecessary and inappropriate for browser plugins and iframe use. Enable actual cross-origin browser access without origin allowlists, new auth/handshake gates or mandatory proxy setup. Own CORS/preflight/Origin/Sec-Fetch/Host interactions proportionately from actual source and browser behavior. Retain unrelated native authority/identity/validation guarantees.
- Scope is existing applicable `web/` consumers and shared HTTP/SDK surfaces required for this goal, including direct browser use and maintained extension/iframe paths. Audit actual access paths before choosing migration boundaries; not every visual/demo/scene module has an API client to replace.
- Source implementation and isolated acceptance are authorized. Replacing installed executables, restarting user daemons/PTYs, registry publication and live LAN rollout are excluded.

## Required result and acceptance

The assigned Controller derives a concise current-to-target model and cohesive responsibilities from actual consumers, then advances implementation through real browser/native integration. Inventory the existing management, Workbench, Observation, Agent Space and Inspector consumers and any extension/demo API calls; identify shared adapters and redundant generated/manual access sources. Explain deliberate exclusions and remaining application-specific state rather than assuming every file must import the package.

Use `@agentx/client` as the actual public access boundary where appropriate. Add or repair missing SDK supply at its canonical owner rather than duplicating logic in pages. Keep browser-safe package imports and actual bundler/dependency compatibility; no Node filesystem/process modules in browser bundles. Preserve all three independent endpoints and existing deployed/embedded route semantics.

Final verification must cover representative connected Web flows using actual SDK→HTTP→native paths plus cross-origin Fetch/preflight from a different browser origin, iframe use and relevant extension host behavior. Distinguish these paths: an iframe's own-origin API traffic does not prove parent/extension direct cross-origin access. Exercise relevant errors, exact-reference/receipt recovery and integer mapping where migration affects them. Existing unaffected checks are reused, not repeated by default. No paid Provider/model claims from finite fixtures. Record actual browser/platform limits and any host/browser constraints that server changes cannot remove.

Complete with discoverable source/module usage, build/generation instructions, current-support status, exact evidence and clean owned resources. One complete implementation batch and concentrated integration/repair are preferred; a focused early browser/tool probe is justified only for an actual architectural uncertainty. No per-file workers, per-task installations or repeated package cycles.

## Accepted local outcome

Controller movi accepts the complete selected source/isolated Linux browser outcome on 2026-11-18, consuming [independent final acceptance](integration.task.md#final-independent-acceptance-handoff), [complete Web contribution](web.task.md#complete-websdk-contribution-handoff) and [native HTTP result](http.task.md). No selected production defect remains open. Exact execution, failures, artifact identities and cleanup remain at the independent Task rather than a second evidence ledger.

| Delivered capability | Accepted boundary |
| --- | --- |
| One maintained Web access foundation | Management, Space, Inspector, Workbench and Observation use public named `@agentx/client` methods and emitted schema-derived types; redundant request/DTO/integer codecs replaced. UI state, exact drafts/journals and original outcome recovery remain application-owned. Demo/extension embed full Space and have no separate AgentX HTTP transport to replace. |
| Unrestricted browser origins | Three listeners accept cross-origin browser/extension/null Origin and cross-site metadata with wildcard credential-free CORS, real OPTIONS and readable success/error/binary responses. Host syntax, exact scopes/generations/actors, revisions, body limits and passive Observation remain enforced. No origin allowlist, auth/handshake or proxy gate added. |
| Real connected browser use | Three independent SDK/native endpoints and root pages; actual admitted-response loss/original lookup/replay/conflict; long identity/bigint and binary equality; desktop/narrow pointer sends, separate parent Fetch/full iframe, maintained genuine MV3 extension capture/attachment/send/service-change paths; Inspector collection/recovery/lifecycle and exact successor/reset fences. |
| Build and retained UI | Five consumers typecheck and embed current SDK; affected cancellation/removals-only adapters pass. Cargo tracks/builds SDK inputs. Production output removes only optional vendor inline sourcemap with provenance receipt; public inspection open/suspend/dismiss preserves discovery and prevents message-button obstruction. |
| Resource closure | Independent final resource receipt reports zero remaining processes for20 exact owned fixture markers; fixture manifests record no cleanup errors and unchanged candidates. Execution lane released; original services and installed programs preserved. |

Final immutable candidate: `/tmp/agentx-web-sdk-movi/candidates/agentx-sdk-adoption-r3`, SHA-256 `935172a4ca3a5f6c7fc4d369bd65a768b0e7baef1c0d795b098dd0167afd1950`. Final artifact/resource receipts and WSA-01..07 mapping are linked in the independent Task. Compatible earlier evidence is explicitly reused; no claim of a single all-passing historical run.

## Limits and maintenance

Subsequent user-authorized cleanup, causal diagnosis and native/test repairs are [completed and accepted](../source-hygiene-diagnosis/controller.task.md#accepted-completed-repair-outcome), with 15 targeted native cases and one actual CLI case passing independently. This follow-up fixes the demonstrated stale-worker admission defect and historical test-contract/fixture causes; it does not alter the original broad-suite receipt or prove its exact reset attribution. No full-suite rerun or installed/live transition is claimed.

Evidence is local Linux Chrome147/156 with finite native/Hook fixtures, no paid Provider/model claim. Host Node24.22.2 is outside declared root Node<23; Node22 execution is not established. Three existing moderate plugin audit findings remain; newly introduced js-yaml exposure is repaired. Broader Rust suite reported3328 pass/20 ignored/8 failures, with path/migration/kill/inbox limits and one unavailable reset-test attribution retained; assigned19 native HTTP cases pass. No blanket full-suite pass is claimed.

The candidate retains its recorded older DancingAI vendor snapshot; current shared feedback-feature vendor differs. Both fit the map-strip/size contract by source assessment, but newer feedback behavior is outside this browser evidence. Browser mixed-content/local-network permissions and host CSP/sandbox/extension permissions remain platform constraints. No store/Firefox/Edge/LAN-PNA or arbitrary-host coverage is implied.

No installed binary replacement, live restart/adoption, registry publication or paid Provider transition occurred. Current package/module guides and [browser policy](../../.docs/api-panorama/openapi/README.md#browser-access-policy) own maintenance. Original failed attempts and workflow corrections remain historical records; do not revive the completed team or rerun unrelated passing checks by default.
