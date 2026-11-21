---
schema: delivery
status: completed
description: Establish evidence-led, independently operable target designs for AgentX Management and Space.
stages:
  redesign:
    description: Research and design both products, compose usable prototypes with shared foundations, evaluate the journeys and derive an engineering sequence.
    tasks: ["./tasks/research.task.md", "./tasks/management-experience.task.md", "./tasks/space-experience.task.md", "./tasks/design-system.task.md", "./tasks/management-prototype.task.md", "./tasks/space-prototype.task.md", "./tasks/evaluation.task.md"]
---

# Management and Space product redesign

The user wants a repeatable way to improve both products from actual user tasks and browser evidence. Develop coherent experience models, sitemap and page structures for each product, a reusable interface foundation, and executable prototypes. Management should help operators understand and act on AgentX daemon resources. Space should help humans find, understand and talk to agents; its conversation list and office canvas need deep design as distinct presentations of coherent work. Design the conversation reading, composing, disclosure and return experience as part of that work. Observability is deferred; useful operational evidence in Management can be proposed when governed by a concrete operator decision, an actual data source and an honest coverage limit.

This delivery covers research, design, local prototypes, evaluation and an engineering handoff. Preserve native authority/identity, published route compatibility, existing source changes and running services. Use realistic synthetic data only in the new preview. Separate present source support, target product choices, prototype simulation and missing contract capabilities. Do not ship changes to production apps. Do not invent interviews or imply user acceptance from expert review.

| Owner | Consumed contribution | Real dependency and authority |
| --- | --- | --- |
| Research | Current daemon resources, actions, state semantics, product surfaces and baseline evidence | Both experience owners use findings as factual sources; researchers do not select future IA. |
| Management experience | Operator goals, resource relationships, IA/sitemap, page and interaction states | Feeds Management prototyper; owns which operator decisions and actions belong together. |
| Space experience | Actor journeys, IA/sitemap, office and conversation list, unified chat and disclosure system | Feeds Space prototyper; owns what the user needs and the continuity semantics. |
| Design system | Shared usable foundations with Management and Space variants | Owns common primitives/tokens and system contract; both prototypers bind to it and own product-specific components. |
| Management and Space prototypers | Two complete reviewable explorable products; maintained page/region/state and fixture bindings | Consume usable increment of product/system contracts as available; own UI composition and local prototype behavior. |
| Independent evaluator | Screenshots and task walkthroughs, concrete problems and impacts, regression evidence after material fixes | Starts final walkthrough when a coherent snapshot is operable. Authors repair their defects in a batch. Controller consumes evaluation and assembles rollout sequence. |

Domain design and system work can run together. Exact Control names, interaction behavior and IA are specialist decisions. Candidate hypotheses: prioritize Management diagnosis-to-action and operation recovery; prioritize Space switching modes mid-conversation and finding the same exact target through distinct list/office affordances. Validate or revise the hypotheses and record why. Different products, common interaction semantics; avoid turning both into one app or forcing identical styling.

Final package: target models with intentional page/disclosure inventory, usable browser previews, screen-by-screen evidence from representative scenarios, flow/scenario and responsive checks, important fixes or explicit residual limits, and implementation priorities mapped to current modules and API constraints. Cover common and adverse states including some combination of empty/error/stale/unknown, contextual return and small-screen behavior as relevant. Compare baseline with candidate when the state/viewport is comparable. Charts earn space by supporting a decision; do not imply unmeasured history. Evaluate actual actions in a browser before interpreting screenshots. Record expert evaluation as distinct from user validation. Use original experience and native records as references rather than duplicating contract definitions.

## Preview

Static synthetic review site, hosted only from this Delivery's preview subtree. Port 18984 (separate from deployed AgentX). Start from repository root with `python3 -m http.server 18984 --bind 192.168.2.178 --directory .controller/web-product-design/preview`. Candidate entries: `/management/`, `/space/`, `/system/`. The same URLs receive new content on manual reload. No runtime API/credentials. Use review-only browser sessions and dispose of test processes. The continuous preview process can remain to allow user inspection.

## Runtime bindings and next decisions

Controller: kula. No additional delegation by assigned owners. Runtime resolved the following Codex assignments; adoption and work progress are maintained in their bound Tasks. The cohort routing message enables direct design/prototype/system exchanges.

| Agent | Adopted role | Bound Task |
| --- | --- | --- |
| buro | browser-explorer | [Baseline research](tasks/research.task.md) |
| silo | product-experience | [Management experience](tasks/management-experience.task.md) |
| nida | product-experience | [Space experience](tasks/space-experience.task.md) |
| toto | design-system | [Shared system](tasks/design-system.task.md) |
| zena | prototyper | [Management candidate](tasks/management-prototype.task.md) |
| gobi | prototyper | [Space candidate](tasks/space-prototype.task.md) |
| peso | product-experience | [Independent evaluation](tasks/evaluation.task.md) |

Both prototype owners have the first usable contracts: [Management operator paths](../../.docs/web-product-design/management.md), [Space paths SP-01–06](../../.docs/web-product-design/space.md#first-usable-path-contract), and [shared system assignment](tasks/design-system.task.md). Both prototypes consume the [shared system](../../.docs/web-product-design/system.md); the [system evidence](evidence/system/) covers both actual preview consumers. The [baseline research](../../.docs/web-product-design/baseline.md) and [matched screenshots](evidence/baseline/README.md) are available: 12 captures at 1440×900 and 390×844, with synthetic inputs and read-only browser paths.

The [independent evaluation](evidence/candidate/README.md) is complete for [Management](http://192.168.2.178:18984/management/) and [Space](http://192.168.2.178:18984/space/), including targeted repairs and final pinned replays. Keep the preview available for inspection; evidence is fixed per product, even if the preview changes.

## Engineering handoff decisions

Space workspace continuity and conversation history pagination are separate deliverable slices. The [Space design](../../.docs/web-product-design/space.md) maps existing route, identity, reader and composer behavior to the target. A first UI slice can share exact discovery/reader state across the two modes while retaining the actual history defaults. The target "open at latest, then load older" depends on a separately owned bounded read: [native Messages](../../src/management/store.rs#L265) currently defaults to cursor zero and queries oldest-first; [the Service](../../src/management/service/resources.rs#L34) exposes the same after-only shape. Native/SDK must define a bounded latest window and backward cursor with an exact-reference boundary, tests for same-name reuse/generation and page continuity, and backward compatibility for existing after-only callers. The prototype may simulate this view; the production UI must not drain the forward cursor to impersonate the latest window. Treat a recent-conversation index and richer Provider blocks as separate supply choices, conditional on demand and trustworthy data. No production implementation in this Delivery.

Management should first pair exact Agents selection (MG-01) with durable contextual recovery (MG-03). Keep read/UI selection and the operation journal distinct while sharing the actual endpoint/scope/generation boundary. The present [browser journal](../../web/management/model.ts) and [consumer controller](../../web/management/use-management.ts) are the starting owners; the [SDK endpoint adapter](../../web/management/api/client.ts) is the source of the actual target rather than a label inferred by the UI. Migrate old receipts conservatively: never assign unproven endpoint/scope to a retry, never send under a new generation, and only explicitly replay a natively eligible absent request with the same ID/body. Test a lost reply, listener change, generation change, stopped namesake and focus return against an isolated native fixture. Follow with Workspaces and lifecycle (MG-02/04), requiring path invalidation, preview/actual recipient distinction, truthful capability gates, and result-to-finder links whenever the receipt does not contain an exact identity. See [Management capability disposition](../../.docs/web-product-design/management.md#capability-disposition-and-engineering-handoff) for current coverage and separate native gaps.

## Controller screenshot reading

In the [Management directory result](evidence/candidate/management-r2/run-2026-11-17T22-48-28.332Z/mg02-routing-actual-recipient.png), the receipt gives the actual recipient local context while leaving its workspace visible. In the [narrow Unknown receipt](evidence/candidate/management-r2/run-2026-11-17T22-48-28.332Z/mg03-narrow-native-unknown.png), the decision cue ("may have taken effect; cannot retry") is visibly separated from raw request details. For production information density, keep this cue and the actual recipient ahead of raw identities while keeping identifiers copyable.

The [Space office with an open conversation](evidence/candidate/space-v1-final/run-2026-11-17T22-48-18.296Z/sp01-list-continuity-same-reader-in-office.png) shows selection and active conversation together; the [unselected conversation list](evidence/space-author/list-1440.png) gives discovery a separate, scan-friendly shape. On repeat use, the large unselected welcome region may be less valuable than an intentional work summary or recent activity. Treat that as a user validation question; do not populate it with unsupported unread/recency claims or silently open an inferred target. Screenshot reading supports these composition judgments; the independent walkthrough owns interaction results.


## Accepted design and prototype outcome

Use the two maintained product designs and their reviewed synthetic candidates as the engineering reference. The [independent evidence](evidence/candidate/README.md) owns the exact pins, screenshots and limits: Management 9/9 cases across 25 observed checkpoints; Space 15/15 across 28, including three iframe sizes. The review exercised all named criteria with explicitly partial coverage of some variants. Both final replays report no unhandled page errors or unexpected requests. Material findings were repaired and rechecked within their owning prototype. The Controller accepts the scoped design and walkthrough result for engineering handoff; user acceptance, native validation, user research, production deployment and installed extension adoption remain separate.

For subsequent implementation, first extract only the exercised shared primitives and interaction rules into a reusable boundary, leaving business state and spatial art with each product. Then implement the first Management and Space cuts described above independently, with one isolated native integration check per coherent cut. Extend the Management operator paths, Space reader history supply and page/capability parity in order of evidence and dependencies. Use the same scenario IDs to connect actual native/SDK behavior, UI result and new browser evidence. Before broad rollout, observe real operators and Space users attempting the jobs with representative data; acceptance must distinguish speed/comprehension and observed user preference from browser assertion success. Office high-population discoverability, default view, and recurrent empty-list use are concrete research questions. Preserve existing routes/data/services until an explicitly selected adoption step. Observability remains deferred.
