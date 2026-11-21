---
schema: scenario
description: Read original sources and reconcile independent scenario, domain and UI inventories with explicit discovery and coverage limits.
tags: [interaction-modeling, provenance, coverage, discovery]
---

# Read sources and assess coverage

Use when a claim needs its original source, or a completeness question extends beyond a selected [interaction slice](method.md). Reuse the consumer's existing inventory/model/review. These working formats do not establish another registry or require a particular scanner or viewer.

## Read the source behind a claim

For each consequential object, edge, action or coverage assertion, retain authoritative source, locator, interpretation, evidence type and source basis. Registration proves a reference exists; opening it proves readability; neither proves its meaning or implementation. Separate verified, inferred, proposed, unavailable and unexamined assertions.

| Source | Locate and read | Limit |
| --- | --- | --- |
| Document | Path/URL, heading and passage/range with surrounding context; identify current fact, selected target or proposal | A written promise is not execution; a heading is not criterion coverage |
| Code | File, symbol, registration/call site and version-specific range; inspect resolved handler and branch | A definition alone proves neither active binding nor effect |
| Schema/type | Native export, path/variant and version; distinguish input, parsed output and wire projection | Shape does not prove supply, authorization, transitions or UI behavior |
| Image/media | Resource identity/version, caption, frame/time/region, environment and capture origin | A visible state does not prove hidden calls, completion, motion or understanding |
| External page | Exact page/fragment, publisher, access date, version where available and passage read | An unread/mutable/inaccessible URL is not inspected evidence or local support |

Show readable title, kind, locator and context. Format code/schema as data, preserve document structure, and provide useful media descriptions. Label excerpts and truncation. A bare path or raw JSON is insufficient explanation. Link summaries to originals and label interpretation. Preserve scenario/object/filter/selection and return position when opening sources. Follow the project's image-reading rule before inspecting images; capture/linking and visual interpretation are different actions.

For fixed reports, retain selected source content or a retrievable immutable version and the report association. For live views, show the current basis and last verification time. Git HEAD alone does not identify uncommitted content; old line numbers can drift. On change, mark stale/drift/unchecked claims, preserve old results and choose affected mappings/checks to refresh. Missing or denied sources remain evidence/coverage gaps. Disclose incoherent mixed-version selections.

If a source viewer is used, follow its explicit permitted resource contract. Resource IDs must not grant arbitrary filesystem traversal or URL fetching. Its implementation owner must enforce permitted roots/files, resolved symlink boundaries, private-path exclusions, MIME/size limits and safe failure. Model references do not grant access to credentials, private configuration or recovery archives.

Render sources without executing them: code/schema/JSON as data, sanitized Markdown with scripts and unsafe HTML inert, supported media under the viewer policy. Do not import source modules, run embedded commands, activate documents or fetch external assets to make a preview work. Identify external navigation. Unsupported/blocked/oversized material needs an honest status and an authorized text/original-link fallback if available. A “read-only” label does not prove these controls exist.

## Build three independent inventories

Declare product/module, actor/role, current versus proposed behavior, source basis, included surfaces/packages, extraction method and omissions before counting. Obtain the denominators independently of the interaction model, then join by reviewed semantic references.

| Denominator | Independent source | Preserve before joining |
| --- | --- | --- |
| Scenario/task criteria | Canonical requirements, scenario entry and accepted experience decisions | Criterion/outcome, actor, prerequisite and branch; proposed discoveries in a separate candidate set |
| Domain responsibilities/capabilities | Native objects, rules, state, schema/procedure variants and public exports | Object role, variant, inputs/results/effects, lifecycle/constraint owner; internal capabilities need no forced UI exposure |
| UI operations/entries | Actual control/handler registrations, event delegation, keyboard/context/gesture paths and dynamic factories | Located occurrence, trigger, resolved handler, facet/scope, local/service behavior and unresolved discovery sites |

Keep source occurrences and semantic actions as different units with an explicit normalization mapping. Several controls may expose one action; a handler may dispatch multiple actions; an action may compose calls or invoke none. Preserve aliases, variants, repeated entries and exclusions. Procedure, handler, control, action and criterion counts are not interchangeable.

Reconcile in both directions:

1. For each criterion, locate required domain facts/capabilities, UI entries, outcomes and decisive branches. A linked ID/title alone is insufficient.
2. For each domain capability/variant, locate scenario/caller use and UI binding where applicable. Explain internal, unsupported or deliberately unexposed capabilities.
3. For each UI action/entry, resolve object/facet/scope, canonical criterion or candidate use, actual call/local effect, result and return. Retain ambiguities and unmapped items with reason and owner.
4. Inspect contradictions and unmatched rows, then challenge the scenario set with the case matrix. Do not remove inaccessible sources or dynamic sites to improve coverage.

The existing scenario list is a catalog, not an exhaustive task universe. Missing roles, responsibilities, lifecycle steps and UI operations can suggest candidate scenarios. Only the product/scenario owner can accept, merge, defer or reject them. A button or technically possible operation does not create a requirement. Do not copy one project's counts or handler/action ratios into general thresholds.

## Challenge scenarios with a case matrix

Apply dimensions that affect the promise and relevant `D/U/A/E` slice. Record not-applicable with a task/domain reason. This matrix exposes blind spots; it requires neither every Cartesian combination nor a feature for every cell.

| Dimension | Consequential cases |
| --- | --- |
| Lifecycle | Entry/create/import, read/select, edit/preview/apply, observe completion, reuse/history, end/removal where supported; usable empty, waiting and terminal states |
| Batch/scope | Single/selected set, mixed eligibility, empty selection, partial outcomes and remaining work; item/use-site/all-reference distinction |
| Conflict/authority | Changed baseline, permission/lock, moved/deleted target, stale capability/input and concurrent edit; submission rechecks |
| Recovery/effects | Refusal before effect, partial commit, lost reply/unknown effect, duplicate/late event, reconnect and supported retry/undo/cancel; original work identity and completed items |
| Return/continuity | Close/back/navigate/reopen during draft, comparison or pending work; retained selection/scope/version/position/focus and invalid-context handling |
| Roles/supply | Relevant actors/capabilities, configuration/input providers and version agreement; facts needed to start and continue |

Link applicable cases to criterion, object role, entry/action, guard/result, feedback, return/recovery and property/trace. Use the [finding categories](checks-and-evidence.md#counterexamples-and-gap-owners). Unknown expectations stay unknown; a candidate promise and an implementation defect need different dispositions.

## Respect dynamic discovery limits

Static extraction is bounded by syntax, imports and selected sources. Record extractor/version, files, supported patterns and unresolved sites. Where relevant inspect computed names, aliases, generated registrations, event delegation, factories, flags, lazy surfaces, keyboard/gesture paths and embedded/canvas controls. Do not claim discovery of unsupported patterns.

Separate literal extraction, inferred candidate links, manually confirmed bindings and runtime observations. Inspect registration → selected event/target → handler branch → actual local effect or procedure/variant/input → result/feedback. Matching labels, names or nearby calls are only candidate links. Check reused action IDs, stale aliases, wrong facets, no-op placeholders and fallback dispatches that can appear bound while doing the wrong thing.

When names cannot be enumerated, retain a located discovery site with unknown cardinality. Report known items and unresolved sites separately; unknown is not a zero-sized exclusion or a 100% discovery rate. Excluded sources need reason, owner and investigation route.

For runtime evidence, select explicit authorized paths and record build/data/role/flags/viewport, actions, actual calls/results and assertions. Use the existing observation/instrumentation method. Do not crawl arbitrary clickable elements or invoke unknown/destructive/paid actions to fill counts. Observations cover visited configurations only; simulated calls keep their Mock boundary. Cataloging does not execute handlers or prove registration activation.

## Report coverage with owners and dispositions

These are separate predicates with their own evidence, not one status enum or a replacement for [evidence types](checks-and-evidence.md#keep-evidence-types-separate).

| Predicate | Required basis | Does not imply |
| --- | --- | --- |
| `catalogued` | Located item in a scoped inventory, extraction/verification status | Mapping, reachability or implementation |
| `bound` | Reviewed criterion–domain–UI links and ambiguity handling | Dynamic legality, model execution or working UI |
| `modelled` | Relevant state, guards/results, cases/properties and omissions represented | Checker support or implementation execution |
| `executed` | Identified run, observations/assertions and explicit model-check, UI/Mock, real-service or production boundary | All paths, business effects from a condition trace, or user validation |
| `user-validated` | Relevant participant/task observations, environment, assistance, results and limits | All users, complete implementation or production guarantees |

Implementation support needs located current evidence and remaining obligations of its own. Full classification can include unimplemented, deferred, unsupported, candidate and unknown items. It is not “all implemented.” A finite check does not execute a business action; user task evidence does not validate every handler.

Adapt this join record inside the existing inventory/review:

```text
Scope / source basis / extraction limits:
Canonical or candidate criterion:
Object role / operation+variant / UI occurrence+semantic action:
Source locators / confirmed mapping or ambiguity:
Applicable lifecycle/batch/conflict/recovery/return case:
Catalogued / bound / modelled / executed(boundary) / user-validated: evidence or gap
Current implementation support / remaining obligations:
Unmapped or not-applicable reason / consequence:
Disposition / receiving owner / next source, decision or check:
```

Report each numerator against matching axis, unit, scope and predicate, with exclusions and unknown sites. No combined score or fixed count establishes exhaustive scenarios. Send priority/requirement choices to the product owner, semantics/state/supply to the domain/contract owner, entries/bindings/feedback to UI, and unsupported extraction/preview to the tool owner. Record accept/merge/fix/defer/reject/out-of-scope/needs-decision with reason. Unknown ownership goes to the responsible parent for resolution rather than ending as an anonymous “unmapped.”

Close the inventory contribution when independent lists are reconciled, applicable matrix gaps and discovery limits are visible, and unmatched/candidate items have disposition, reason and owner. Implementation and user/service evidence can remain open with explicit receiving owners. Do not wait for unrelated tooling or claim universal completeness.

## Apply the inventory to product work

Choose a consequential finding and return to the original criterion, native contract and running UI. Inventory rows are investigation leads: a procedure may supply facts through an aggregate read, and a service preview may run without ever presenting a review to the user. Trace those joins before adding calls or declaring missing capability.

Keep one linked application record: trigger, object/version/scope, expected result, observed call/effect, category, owner and disposition. A finding can affect several responsibilities:

- Tool gap: the selected extractor/checker cannot express or evaluate the question. Fix only what the slice needs and retain unsupported obligations.
- Model gap: a relationship, guard, input source, action binding or step context misrepresents the product. Repair the binding; missing native semantics go to that owner.
- Product gap: a promised capability's entry, review, scope, feedback or recovery is absent or wrong. Correct the original implementation and observe the consumer through its existing client.

When conditions change between steps, use sourced step contexts or an actual transition executor. One initial world does not prove later steps. A saved new configuration does not silently replace an existing task's pinned version. Unknown effects remain unknown after local coordination stops.

Before changing implementation, fix the acceptance slice: normal path, decisive refusal/conflict/unknown branch, return continuity, allowed changes and preserved facts. After correction, execute only affected checks and consumer paths; preserve before/after evidence and failed attempts. Refresh inventories/reports after their inputs stabilize. A green classification dashboard cannot replace product verification or justify spending the iteration extending a console.
