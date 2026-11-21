---
schema: scenario
description: Establish and evolve a source-grounded design map without duplicating its authoritative facts.
tags: [design-management, architecture, product-evolution]
---

# Maintain a connected design model

## Enter at the current question

| Starting condition | Next action |
| --- | --- |
| Existing product, scattered designs | Identify its current entry, intended users, full scenario outline and fact owners; reconstruct only the necessary relationships from source and evidence |
| New product or uncertain outcome | Use product-design discovery; mark hypotheses and proposed structures before connecting views |
| Change to architecture, command or behavior | Locate affected scenario criteria, subjects and edges; record intended changes against the relevant baseline |
| Source changed after a captured view | Check the selected source basis and affected promises; label stale/unexamined claims until reconciled |
| Small correction with unchanged semantics | Update the local fact and references; reuse the existing model |

Read the product's actual scope and accepted direction first. Distinguish independently maintained applications, packages, source candidates and installed products. A historical controller, design or successful run does not supply current implementation authority.

## Establish a usable baseline

1. Locate the narrowest owner and its normal README/AGENTS entry and artifact catalog. Reuse its current model home; otherwise create the needed entry at `.docs/model/README.md` following [placement rules](../../documentation.md#artifact-placement-rules). Explain scope, core language and relationships, then connect canonical scenario, topic/module design, native declaration, protocol, technology-decision and evidence sources. Keep them with their owners; a small model can stay on one page.
2. Identify the selected user outcome and a branch that can change the design decision. For substantial reconstruction, outline wider intended outcomes and mark what remains unexamined; a local change need not reconstruct the whole product.
3. Connect the relevant concepts, rules, responsibilities and behavior using the [view contracts](views.md). Use existing names and heading links; introduce stable IDs only when cross-view references need them.
4. Distinguish important claims by their actual basis: source/contract, accepted decision, observation or unresolved assumption. Canonical links and the affected diff normally suffice; retain a fixed source identity only when the selected claim or report needs it, following [basis selection](../../scenarios/collaborative-modeling.md#retain-the-needed-basis).
5. Keep precise APIs, schemas and command grammar in native declarations that implementation can consume directly, including declarations written before implementation. Domain documentation or existing adjacent comments own meaning and rules; the original scenario owns the user promise. Generated catalogs name their source and omissions.

Make each selected view's scope, current/target/alternative meaning and important relationships understandable through its surrounding explanation and source links. Explain relevant evidence limits beside the claim. Shared context can serve several views; do not add a source register or repeat metadata for every node and edge.

Map existing nondefault homes through actual `model`, `roadmap`, `proposal` or `delivery` artifacts when useful; do not duplicate their content or status, move accepted history, or register future files as readable resources. The entry must let a reader find the meaning of a rule, its owning module and exact contract without reconstructing past Tasks. A file inventory alone does not provide that model.

When source and model disagree, record the expected behavior, observed behavior and affected scope in the existing work record. Determine whether this is an implementation defect, outdated documentation, mistaken interpretation or an intentional change, then repair the owning source. Keep an unresolved difference explicit; neither source code nor an accepted design alone proves the intended behavior is delivered.

## Maintain a feature's current design

For a [continuing product feature](../product-design/requirements.md#identify-the-continuing-feature), keep a readable home for its purpose/scope, representative scenarios, key rule and experience sources, actual support, remaining targets and change rationale. Reuse an existing section or local specification; create a separate entry only when independent reading and maintenance need it. Follow [placement](../../documentation.md#artifact-placement-rules). The feature connects its design; domain, interface, prototype and evidence owners retain their original facts.

Use [complete design questions](../../scenarios/product-design.md#organize-a-complete-feature-design) to expose missing meaning before adding files. A feature entry may link an original scenario, native declaration and prototype; it need not repeat their fields, criteria or status. A heading link is enough until another consumer needs a stable local identity. An internal helper need not become a product feature.

Keep a cross-feature scene with its complete-outcome owner. For example, receiving a shared report uses export, sharing and read authority; the report feature does not acquire ownership of authorization. Explain the necessary handovers and shared rules. Feature decomposition, scenario order, module composition and work assignments have separate meanings, even when their names overlap.

Use the [relationship meanings](views.md#relationships-between-design-expressions) only where they help a reader judge or change the design. A prototype records its adopted or exploratory treatment and simulation limits; a check records the criterion and basis it examines. Neither becomes a second definition of the product rule. The [worked example](feature-example.md) shows these sources and a later change without a new registry.

## Represent a planned change

Record the initiating user outcome or problem and the deciding example. Show affected nodes/relationships/steps as added, modified, replaced or removed. Explain changes to visible results, identities, authority, ordering, resource lifetime and recovery where relevant.

Keep current behavior readable while a target is proposed or implemented. Separate accepted target decisions from unresolved alternatives; record the decision source and rationale. Do not retroactively rewrite a historical witness to match the new target.

Carry the same model through the [Roadmap → Proposal → Delivery → Task relationships](../../scenarios/feature-planning.md#carry-a-model-change-through-the-work). Planning owns the selected change and its delivery scope; the design topic owns the meaning and native declarations own exact contracts. The Proposal references affected model sources and records its internal design decisions in body sections or a short table. An accepted topic may be refined here as an explicit target before implementation; link the Proposal's reason and retain current support separately. Formal aggregation stays upstream, while reverse views are derived with their query coverage limits.

A Task names the design question or behavior it owns and the model/contract destinations it will update. Use [Task records](../../scenarios/task-records.md) for execution state; the view references it rather than maintaining another tracker. Routine implementation choices within authorized scope proceed without an extra approval ritual.

### Determine the affected updates

Start with the original fact or promise and its intended difference. Distinguish a behavior change, an implementation defect, an outdated interpretation and a presentation-only correction. Follow its direct uses, refinements and presentations, then the compositions and checks needed for the selected outcome. For shared or public rules, also inspect known channels and consumers; absent links do not establish absent use.

Record the useful conclusions in the existing Proposal or Task: which sources must change, which need review but can stay, which are unrelated and which remain unexamined. Name each changed source's owner and required check. Explain preservation when it avoids unnecessary work; do not create a permanent impact ledger or mechanically redo every reachable document and test.

For concurrent changes, reconcile the latest shared rule and its actual overlap before merging. A new name or file move keeps the original feature identity when meaning is unchanged. A split, replacement or retirement names the retained/removed promises, successor and existing-consumer treatment, updates current navigation and preserves original evidence. A cancelled Proposal does not remove support that was already delivered.

## Return implementation and evidence to the model

After implementation, map changed promises to actual entry points and collaborating owners. Record support with separate claims:

| Claim | Needed basis |
| --- | --- |
| Defined | Canonical declaration/design and the selected semantic scope |
| Implemented | Source entry and relevant implementation basis |
| Connected | Actual caller/adapter/composition path |
| Verified | Named check or observed scenario, selected version/environment and exclusions |
| Distributed/installed | Exact candidate/release/installation identity and applicable evidence |

These are evidence dimensions, not a compulsory linear workflow or a percentage. An implemented feature may be intentionally frozen; a verified library may have no installed application. Use explicit unknown, unsupported or not-applicable when appropriate.

Read them against the [selected scenario coverage](../product-design/requirements.md#write-promises-that-can-be-challenged). State which actor/outcome/channel/version is supported and which necessary path or handover remains missing. A narrow completed batch leaves the rest of the continuing feature's target explicit; overall usefulness requires its own product judgment.

Check edges as well as endpoints. Two existing modules do not prove integration; successful persistence does not prove notification, recipient reading or model action. Return failure, partial and unknown outcomes to the same user scenario.

Merge established semantics into the relevant current topic, update actual consumer/environment/version support and remove target differences that are now resolved. Link the original Proposal for rationale and work records for evidence. Keep outstanding targets explicit. The Proposal retains its decision history without maintaining a second copy of the current rules. Do not leave the current model as a chain of appended change notes that readers must reconcile themselves. Preserve fixed historical captures and their original claims.

Perform this update under the responsibility assigned by the Delivery/Task. For a staged change, integrate only the established part and retain the remaining target with its decision and delivery links. Domain owners update their own rules; the named integration owner reconciles the product scenario and cross-domain relationships. If code exposed an incompatible promise, resolve it with the proposal owner before merging a different meaning. A Task cannot claim its required model update is complete merely by linking an unmerged design.

## Keep changes discoverable

When a declaration, source, scenario or accepted decision changes, inspect its referenced consumers and update affected views or retain an explicit unresolved drift marker. Automate extractable facts and structural checks where worthwhile. A hash change requests inspection; it neither proves a semantic defect nor updates the meaning automatically.

Use existing change/Task workflow to record this work. No background watcher, new database or mandatory repository-wide CI gate is required. Tools derive indexes and views from the original documents and declarations; authors should not repeat domain facts in a projection script or object registry. Keep meaning a tool cannot extract in the readable source. Edit that source and regenerate disposable views as needed; a hand-authored semantic diagram remains part of the owning design. Check changed links and rendering using available tools; state if only syntax/source was inspected.

## Judge the result

From the real entry, perform a bounded reading walkthrough:

- Find the relevant scenario and its visible outcome.
- Explain the involved subjects and key ownership/lifetime distinction.
- Distinguish current behavior, accepted target and an unverified or proposed relationship.
- Locate the exact design step or edge a user would change.
- Follow one claim to its source/evidence and interpret its limits.

Record actual findings and fixes in the owning Task. Static walkthrough, automated structural validation, actual Agent use and human usability feedback are different evidence. Do not claim measured reading speed or user acceptance without an actual observation.

Stop at the assigned outcome, leaving remaining substantive work with an explicit owner and dependency. A complete baseline can be useful even when the product itself still has planned features.

