---
schema: scenario
description: Connect product needs, experience models, prototypes, contracts and feedback into a verifiable delivery
tags: [product-design, discovery, prototyping, feedback]
---

# Product design from needs to evidence

Use for a new product, a substantial feature or a redesign whose user problem, experience or delivery boundaries remain uncertain. For a bounded change with an established goal and contract, go directly to the affected professional method. This scenario owns cross-stage decisions and handoffs; it does not replace [feature planning](feature-planning.md), [web design](../domains/webdevelopment/AGENTS.md) or [engineering design](engineering-design.md).

For professional discovery, requirements, prototype design, evaluation and delivery depth, select the [product-design methods](../domains/product-design/AGENTS.md). Existing Web and engineering methods keep their original ownership.

When the product includes a user interface, first read the required [product and interface design requirements](interface-design.md). They apply to its experience, layout and realization, including tools, management products and mobile screens.

For simple content, express it directly. For new models, complex relationships, interacting states or multiple dimensions, apply [visual model communication](interactive-design-communication.md): derive a polished panoramic HTML view from the domain model, compose professional visual forms such as Mermaid, SVG, Canvas, chart libraries or 3D renderers, deliver the portable componentized HTML file early; when shared access is needed, serve that same file through LAN HTTP and update it with the canonical model. This also applies to systems without a frontend.

## Enter at the current question

For an evolving product whose scenarios, architecture and interfaces need a connected current/target reading entry, compose [design management](../domains/design-management/AGENTS.md). It owns view maintenance and source/evidence links; this guide continues to own discovery and scenario promises. A bounded change can reuse the existing entry.

| Starting point | Next useful result |
| --- | --- |
| An uncertain idea | A concrete actor, situation, obstacle and desired outcome; distinguish observed needs, assumptions and proposed solutions |
| A known need or feature | A deciding use case, important failure and observable success conditions |
| Conflicting product concepts | Examples that distinguish meaning, identity, ownership and lifecycle before choosing interfaces |
| Several connected functions | A complete outcome slice, its required capabilities and shared facts |
| A precise module or protocol | The affected contract; use [collaborative modeling](collaborative-modeling.md) when responsibility or behavior still needs derivation |
| A page or visual concern | The matching [web/experience method](../domains/webdevelopment/AGENTS.md); no automatic domain-modeling pass |
| An existing system to reconstruct or evolve | Its whole-scenario map, relevant source-grounded baseline, current assumptions and intended semantic changes |

Enter at the matching question without restarting completed discovery. Features, domains, pages, packages and processes need not correspond one-to-one. Reading a route selects a method; it does not authorize implementation, delegation or installation.

Use this working loop: select a user outcome and the next uncertainty, derive the affected model, challenge it with a deciding example or check, then return the result to the same scenario and revise its consumers. Follow the branches needed for that question; the sections below are not mandatory delivery stages. Keep the current conclusion, unresolved dependency and next action in the existing work record.

For a feature selected from a roadmap or proposal, recover its [model-change relationship](feature-planning.md#carry-a-model-change-through-the-work): current scenario/model, intended and retained promises, selected scope and open choices. Record new design results as refinements of that change in the owning topic and native contracts. If discovery changes the product promise, return the difference to its original proposal/decision rather than silently replacing it in a local design. A new idea can begin here and acquire those planning links when selected.

Planned capability work uses [Roadmap → Proposal → Delivery → Task](feature-planning.md#planning-levels). A Proposal owns the change's design decision and records its internal accepted/open/deferred choices in body sections or a short table. Keep detailed meaning in the distributed model and exact contracts in native declarations. Follow [placement rules](../documentation.md#artifact-placement-rules) for new homes, retain real existing entries, and omit parent artifacts for bounded direct changes. Planning choice, design acceptance and current support remain separate facts.

For substantial existing-system reconstruction, retain a whole-scenario map before selecting deep slices. Cover intended outcomes, including future or unexamined scenarios, and record each scenario's fact owner, actual implementation entry, source era and evidence limits. Distinguish declared capability, implemented behavior, installed composition and actually exercised behavior; link evidence for each supported claim and leave unknowns explicit. One successful slice does not establish the other scenarios, and future scope is not automatically a current defect. Reuse this map for a bounded change; a clear small contract edit does not require a new full inventory.

Reuse the user's answers and available context. Advance one consequential decision with a concrete interpretation or a few relevant alternatives and their tradeoffs. Ask only for missing information that can change the result; do not infer agreement from silence or require an exhaustive questionnaire. Retain useful understanding, decisions, open questions and the next action in the existing owner.

## Establish the problem and the design question

Identify the actor, situation, desired outcome, current obstacle, available evidence and material constraints. Separate an observed need from an assumed need and a proposed solution. A user instruction can establish direction and authority without supplying research evidence for every assumption.

Build only the relationships needed to select a useful slice:

`user problem -> desired outcome -> capabilities/requirements -> representative journey -> acceptance evidence`

Record scope, non-goals, unresolved assumptions and the evidence that would change the design. Where requirements depend on one another, show those relationships rather than treating a feature list as a needs map. Use [planning](feature-planning.md) for priorities and Delivery/Task ownership. Do not fabricate interviews, personas, metric baselines or user preferences.

For the deciding use case, clarify its trigger, preconditions, meaningful actions, alternatives, observable end state, failure residue and what must remain unchanged. Derive related concepts and rules from these distinctions; an action list is not automatically a list of API methods. Use cases, concepts and needs can refine each other before any technical declaration exists.

Choose the next artifact by the uncertainty it must resolve:

| Uncertainty | Useful next action | Evidence sought |
| --- | --- | --- |
| Whether the problem matters | Inspect existing use, ask a focused question or conduct relevant user research | Actual situation, obstacle and outcome; assumptions remain labeled |
| Whether people can find and complete the task | Model the journey and make a navigable prototype | Entry, comprehension, actions, return and recovery |
| Whether a necessary capability is feasible | Examine its contract or run a bounded technical experiment | Capability, limits, failure behavior and deployment constraints |
| Which visual or interaction treatment fits | Render a few comparable treatments of the same scene | Hierarchy, clarity, task fit, behavior and user preference |
| Whether implementation delivers the outcome | Integrate the real participants and exercise the journey | Actual effects and relevant failure/recovery behavior |

Collect useful feedback early. Do not require a complete prototype before discussing a risky assumption, or a complete architecture before testing a local experience. Existing authorization covers ordinary progress; ask only for a material missing choice or authority.

## Choose precision and a complete slice

Use prose, examples and small tables while meaning or boundaries remain disputed. Preserve alternatives and unresolved questions rather than filling types with guessed answers. When a relationship is understood and precision serves a current decision, express that selected part as a declaration, transition specification or data mapping. A clear existing contract can be edited directly; one bounded part may be ready while the wider product remains uncertain. A design question left open is not automatically a runtime unknown-result state.

Keep intent, rationale and alternatives beside the model. Once a declaration owns exact fields or transitions, reference it instead of maintaining a second editable copy in prose or diagrams. A product conversation may finish with a useful textual model; machine-readable syntax is neither a mandatory phase nor evidence of product value.

Select a slice that reaches a user outcome, including its deciding failure and recovery. Map the required capabilities, fact owners and dependencies; choose module boundaries by responsibility, consistency and actual change needs. Do not decompose every noun into a service or every function into a page. Use [collaborative modeling](collaborative-modeling.md) for a selected module or cross-module behavior that needs deeper derivation, and [composition](engineering-design.md#domain-module-and-product-composition) for system boundaries. Experience work can proceed with known results and explicit assumptions while private technical details remain open.

## Organize a complete feature design

Use the [continuing feature](../domains/product-design/requirements.md#identify-the-continuing-feature) to connect a product outcome across successive changes. Reuse its current design and scenario sources. Select the unresolved questions below; their answers can share one section or remain in existing specialist sources. They are not a required document set.

| Design question | Establish or locate |
| --- | --- |
| Why and for whom? | Situation, need, evidence or assumption, desired outcome, constraints and exclusions |
| What must work together? | Feature scope, representative scenarios, decisive alternatives/failures and role handovers |
| What must remain true? | Domain meaning, identity, rules, state, permission and relevant quality guarantees with their owners |
| How does the user understand and act? | Applicable channels, information, entry, choices, feedback, waiting, interruption, return and recovery |
| Where do necessary facts come from? | Meaning, source, grain, version, derivation, read authority and absent/unavailable semantics |
| What can uphold the promise? | Capability and native contract owners, composition, important technical choices and resource responsibilities |
| What material answers the next uncertainty? | Selected prototype, fixture, diagram or other expression, its source, purpose and simulation limits |
| How will the outcome be judged? | Criteria, checks, observations and remaining evidence obligations at the promised boundary |
| What is current or still intended? | Actual support, selected targets, open choices and references to the changes that explain them |

A design is usable for its selected scope when the result and decisive branches are coherent, necessary facts and guarantees have owners, related expressions agree, and remaining unknowns have an explicit next action. A feasibility question that could invalidate the result remains a design dependency. A design-only outcome can stop with that question owned; it must not claim a settled implementation promise.

An API-only capability can express the task through calls and results without pages or prototypes. A small internal function can retain its native contract and examples without a product feature record. An unchanged bounded edit reuses the established answers. Keep the current reading entry and affected sources through [design management](../domains/design-management/workflow.md#maintain-a-features-current-design); see the [portable example](../domains/design-management/feature-example.md) when a worked relationship helps.

## Maintain one scenario source

Keep the product's existing scenario document as the authoring source. Describe meaningful user nodes, their observable criteria and important alternatives there; introduce stable keys only where another artifact refers to them. A node may involve several operations or no user action, such as waiting for a result. Derive the facts, decisions, effects and invariant owners needed for each criterion, then return success, rejection and recovery semantics to that same node. A node does not prescribe one API or one module.

For a design-only task, a short Markdown section can be enough. This illustrative scenario assumes the product supports background work; it is not a universal requirement:

| Node | Criterion key | Product promise |
| --- | --- | --- |
| Leave a running export | export/leave/C1 | Closing the view leaves the selected export running |
| Return to it | export/return/C1 | Show the original export's current outcome, including failure or an unknown result |

A downstream contract or verification note references `export/leave/C1` and names the required lifetime owner and observation; it does not restate an independently editable promise. This example needs neither a UI prototype nor a Scene/Plan/Run: its current question is the separation of view and work lifetimes. Record such a scope choice briefly when it prevents ambiguity; do not add a separate tailoring checklist.

Extract a structured Scene or another projection when an actual consumer needs its supported format. Retain source/node/criterion identity, selected basis, transformation and omitted coverage. A manual extraction needs an explicit mapping check when its source changes; it is not automatically synchronized. If a structured source becomes canonical, make the former prose reference it. Do not hand-maintain the same criteria in prose, Scene and mapping. A design without an executor can finish with clear criteria and verification obligations; it needs no placeholder execution artifacts.

## Keep complementary models distinct

| Model | Owns | Professional method |
| --- | --- | --- |
| Needs and outcome map | Why the work matters, scope and success | [Discovery](../domains/product-design/discovery.md) and [requirements](../domains/product-design/requirements.md); [planning](feature-planning.md) owns selection and delivery |
| Navigation and interface map | Where users find objects and capabilities; available surfaces | [Experience design](../domains/webdevelopment/experience-design.md) |
| Journey and behavior | Actions, state transitions, guards, effects and recovery | [Interaction/state](../domains/webdevelopment/interaction-and-state.md), [data/state modeling](data-state-modeling.md) |
| Product and module architecture | Domain owners, service boundaries, hosts and deployments | [Composition and contracts](engineering-design.md#domain-module-and-product-composition) |
| UI composition | Page, Screen, Frame, Block and Component responsibilities | [Interface structure](../domains/webdevelopment/interface-structure.md) |
| Shared and local presentation | Tokens, components, patterns, motion, assets and scoped treatments | [Design system](../domains/webdevelopment/design-system.md) |

A sitemap does not express every action or modal transition. A C4 view does not specify every entity or method. A UI state model is not automatically a service DTO or a database record. Keep each source authoritative for its meaning and link explicit mappings across boundaries.

Consider content, accessibility, devices, data authority, privacy, permissions, performance and operational recovery when they affect this outcome. Establish foundational constraints early; add detail as risk and actual consumers require it. Do not make every concern a mandatory document or build every private class before implementation.

## Deepen modules and return their results

For separate module work or authorized delegation, provide the parent outcome and use cases, baseline/source references, owned facts and decisions, neighboring capabilities, shared terms, constraints, current choices and consequential unknowns. Link existing artifacts; [task handoff](task-handoff.md) owns assignment mechanics. Let the module owner choose internal design within the contract; keep cross-module guarantees with their shared owner. An ambiguous boundary is a reason to clarify the model, not to create more assignments.

Name which Proposal change and Delivery portion the module design refines, the original model it changes, where its detailed design and native contract will live, and who returns established results to the current model and parent scenario. Keep ordinary design/refinement inside the cohesive Task; explicitly assign a separate cross-domain decision only when consumers depend on its result. Producing a design document does not itself update the current model or complete implementation.

Feed service success, rejection, partial and unknown outcomes into the experience. Return missing data, recovery needs or infeasible guarantees to the owning contract or product decision. Then replay the same user scenario across the resulting parts: every required fact and action needs an owner, identities and outcomes must agree, and the failure path must remain coherent. A locally usable contract can unblock dependent work but does not prove integration or authorize replay after unknown effects.

Where revision, lifetime or callable-key agreement affects the result, use the [conditional supply check](collaborative-modeling.md#trace-collaboration-and-data-supply) to determine whether the parts can actually compose. Provider existence alone is insufficient.

Before handing a consequential guarantee to implementation, [assign its implementation obligations](collaborative-modeling.md#assign-implementation-obligations): connect the original criterion to the operation or boundary that can enforce it, and identify the evidence still needed. Generated bindings can release consumer work while handwritten behavior remains unverified; keep both conclusions visible.

## Make a representative slice operable

Apply this section when an interactive prototype or implementation is part of the requested outcome. A design-only assignment can instead hand off a scenario, contract or isolated verification model with explicit limits; it need not build a UI or product service to complete its design question.

Select a core journey and the failure or interruption most likely to invalidate it. Give the prototype representative fixtures, explicit state ownership, usable actions and observable outcomes. Placeholder components can expose composition and behavior; simulated services must declare what they do and do not prove.

Use [Storybook](../domains/webdevelopment/design-system.storybook.md) for isolated states, component/Screen treatments and bounded interactive compositions. Exercise a real application host when URL/history, cross-route behavior, authentication, persistence or other host concerns are part of the question. Neither action logging nor a set of disconnected stories establishes a complete journey.

Configuration may describe navigation and fixtures. Required transitions, validation, asynchronous effects and recovery still need an executable behavior model. Start with ordinary components and state logic; create a reusable prototype engine only when repeated, demonstrated needs justify its contract and maintenance cost.

Resolve high-risk integration contracts while the prototype is still cheap to revise. Use [service contracts](engineering-design.md#service-contracts-and-dependency-inversion) and [API/SDK tooling](api-contracts-and-sdks.md) for the actual boundary. Stable contracts can release parallel implementation; mock-only success cannot close real integration.

## Turn feedback into owned changes

Present a task the user can attempt, and a small number of meaningful alternatives when a choice is needed. Hold the scene and data steady when comparing treatments. Explain the consequential tradeoff in ordinary language; do not require the user to speak in design tokens or animation parameters.

Retain enough context to reproduce a finding:

`artifact/version + scene + state + environment + user action + observation/expectation + evidence`

Use the existing feedback mechanism and record. Distinguish observed behavior, reported difficulty, preference, business constraint and suggested implementation. They may inform the same decision but are different evidence. Screenshots support visible-state discussion; temporal behavior may require an interaction trace, recording or actual user operation.

Route each finding to its owner:

| Finding | Revisit |
| --- | --- |
| Wrong goal, scope or success measure | Product problem and requirements |
| Cannot find an action or understand the next step | Journey, navigation, content or visible-state hierarchy |
| Invalid transition, lost context or misleading completion | Behavior or service contract |
| Repeated visual/interaction inconsistency | Shared design rule or reusable component |
| Local treatment or asset does not fit | The consuming Screen/component's scoped design |
| Prototype contradicts real capability | Contract, adapter and affected design assumptions |

Record the decision and reason, changed artifact and recheck result with the existing owner. Keep unresolved or rejected feedback visible with its disposition. Link fixed evidence to current discussion without overwriting what was originally observed. A feedback export proves retained context, not that a shared service received or accepted it.

For a changed criterion or contract, inspect its referenced consumers and state the search scope. Decide which remain valid, need revision or a mapping check, or should be retired. Preserve old observations under their original basis; a source digest change signals drift, not automatic semantic failure. New evidence supports the revised claim and links back to the finding. For choosing links, fixed inputs or an offline handoff, use [source retention](collaborative-modeling.md#retain-the-needed-basis).

A promise can change while fields and signatures stay identical. For changes to recovery, permissions or effect meaning, [compare the affected promises](collaborative-modeling.md#compare-changed-promises), including old consumers and work already in progress. A wording or layout change that preserves meaning stays with its experience owner; changing “stop waiting” to “cancel execution” requires checking the actual cancellation contract.

## Consolidate and verify the outcome

Move accepted prototype behavior and design choices into the owning contracts, components and presentation rules. Reuse production components in current examples when applicable; label any retained experimental alternatives. [Component registries](../domains/webdevelopment/registry.md) distribute reusable source, while stories demonstrate usage; neither replaces the authoritative contract.

Preserve a traceable path from a selected need to its journey, behavior, contract, implementation and evidence. Ordinary links and local identifiers are sufficient. Keep execution status in the Task; keep research, decisions, design resources and runtime contracts with their respective owners under [placement guidance](../documentation.md#artifact-placement-rules).

Return established meaning to the current model, update actual support and retain accepted but undelivered targets explicitly. Preserve the Proposal's rationale and decision history by reference. Judge each Delivery against its promised portion; completing the listed Tasks or Deliveries does not itself close the full product target or change Proposal acceptance.

Verify at the promised boundary using [experience review](../domains/webdevelopment/review.md), contract/integration checks and actual user or operating evidence as applicable. State separately what was designed, built, exercised and observed. For production, include the relevant deployment, recovery and observation conditions; for an experiment, state which hypotheses and environments were covered.

Finish when the selected outcome is supported by its evidence and the next owner can use the result without reconstructing the conversation. Defer unrelated capabilities explicitly rather than inflating this slice. New evidence may revise an earlier model; it does not require repeating all prior work.

For baseline evolution, distinguish changed source bytes, intended product changes and later implementation conformity. Record retained, changed, added or removed behavior with its reason and affected scenarios/contracts; preserve historical evidence. Finish at the requested outcome: clarified needs, a scoped design, an observed prototype or a verified implementation. State what was understood, designed, checked and observed separately. Independent reconstruction or review is used when the task or substantive risk calls for it, not as an automatic extra participant in every design.
