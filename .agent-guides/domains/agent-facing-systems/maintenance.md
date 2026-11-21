---
schema: scenario
status: active
description: Route changes, verify real consumers and recover without contract drift
tags: [agents, contracts, maintenance, evidence]
---

# Maintenance, evidence and recovery

Use when a public promise changes, generated artifacts drift, a consumer fails or delivery needs a completion decision. Keep current decisions and results in the existing owning task/design; do not create a parallel integration ledger.

## Route a change to its owner

| Changed fact | Change first | Then update or inspect |
| --- | --- | --- |
| Identity, lifecycle, invariants or success meaning | Domain model/owner and linked public semantics | Native operations, effects/errors, affected adapters, recipes and state/effect cases |
| Fields, defaults, validation or absence | Native input/output schemas | Derived service types, generated projections and real consumers; inspect create/patch differences |
| Stable error or recovery promise | Native error contract and owning behavior | HTTP/SDK/CLI/MCP mappings, caller recovery and actual failure branch |
| Surface name, grammar, route or envelope | Owned metadata or adapter policy | Generated descriptors, capability map, help/config examples, public compatibility and recipes |
| Provider/runtime/schema dialect | Package/lock/config and selection decision | Generation, adapter behavior, installed/built entry and a decisive actual-client case |
| Installation, public URL or executable path | Runtime/distribution declarations | llms links, README, Skill packaging, MCP examples and built publication |
| Recipe or discovery content | Its canonical document/Skill source | Aliases, distributed references and selecting entries; no unrelated contract rewrite |

Preserve existing consumers deliberately. A field retaining its name can still change meaning through new defaults, path resolution, empty/null handling, effect timing or error classification. Record a changed promise and its compatibility choice rather than treating a clean typecheck as proof of compatibility.

Keep detailed operation reference, recipe and discovery index distinct. Update the fact in its owner, then only its affected summaries/routes. If copied or generated content is necessary for distribution, document its source and refresh/check command.

## Match checks to claims

Before execution, name the changed boundary and what evidence can settle it. Follow the project's existing execution authority and shared test limits; documentation-only work usually needs route/example checks rather than runtime tests.

| Claim | Small useful evidence | Limit |
| --- | --- | --- |
| Generated outputs follow current native declarations | Repeatable generation plus a non-mutating stale-output check | Does not establish domain behavior |
| Types and package entries compose | Required type/build check and consumer import | Does not establish runtime validation or effects |
| Every intended operation is exposed | Compare native declarations to actual CLI/API/SDK/MCP descriptor coverage, names and policies | Descriptor presence does not prove invocation works |
| Tool result/error conforms | Actual selected SDK/client validates a representative success, domain failure and invalid input without dispatch | Does not establish all failures or provider versions |
| Effects and lifetime match the promise | Actual provider/caller through the relevant boundary; observe identity, state and cleanup | A mock or exit status alone cannot prove an external effect |
| Discovery is usable | Start at the published entry or supported host path; follow a selected recipe and inspect distributed references | Local links do not prove deployed publication or host activation |

Record source revision, relevant versions, target/fixture, commands, observed result, artifact path and remaining limits. Distinguish authored cases, source/help inspection, executed checks and unavailable scenarios. Reuse passed evidence unless a subsequent relevant change invalidates it. Do not broaden a successful focused check into a universal support claim.

For stateful integrations, use isolated data and the product's real ownership rules. Observe that the intended object is used, important state persists when promised, attachment shutdown behaves as documented and manager cleanup reaches the expected final state. Retain only necessary receipts; do not publish credentials or personal state in evidence.

## Recover at the owning boundary

| Symptom | Discriminating next action | Correction / continuation |
| --- | --- | --- |
| Generated file is stale | Compare changed native declarations and generator inputs | Regenerate through the maintained command; never patch generated output directly |
| A field silently changes on update | Inspect input versus output defaults and presence/null mapping | Correct the native patch schema or input adapter; verify omitted and explicit values |
| MCP lists tools but call/error validation fails | Check selected dialect, native validation and declared result envelope using a real client | Fix the owned projection; preserve native domain meaning and error codes |
| CLI output breaks machine parsing | Capture stdout, stderr and exit status together | Keep data framing stable and move diagnostics to their declared stream |
| Skill works only inside its author checkout | Resolve references from an external installed layout | Materialize/package required files and declare application prerequisites |
| Consumer loses connection after restart | Query current domain state/endpoint and identify attachment ownership | Reconnect using supported flow; do not create a second resource silently |
| Timeout after dispatch leaves outcome unclear | Query/reconcile through existing domain identity where available | Retry only under the declared idempotence policy; do not infer rollback |
| Integration or schema provider is incompatible | Check exact peers/runtime and adapter requirements | Choose a compatible provider or a narrow supported adapter; reopen stack migration only for a concrete requirement |

If repeated local changes stop reducing uncertainty, use shared [system diagnosis](../../scenarios/systematic-troubleshooting.md). A workaround must not silently become a second schema, an undocumented operation or a weaker public guarantee. Record substantive friction in the existing owner's feedback/task and resolve or explicitly defer it.

## Completion criteria

Delivery is complete for its declared scope when:

- A fresh caller can select and finish the intended task through the real entry and supported interface, with explicit prerequisites and target identity.
- Native contracts own public values/operations, important effects/errors/lifetime semantics are stated, and projections have deliberate mappings without independent schema drift.
- Required derivation/build checks are current, and the decisive success/failure/effect behavior has evidence from the actual consumer boundary appropriate to the claim.
- Published docs, Skill references and MCP setup match the distributed/built artifacts; unsupported paths and evidence limits are explicit.
- Cleanup/recovery is usable, and a fresh maintainer can find the source of truth, change route and remaining work from the project's normal context entry.

These criteria apply to selected surfaces. They do not require every project to ship CLI, HTTP, SDK, MCP and Skills together.
