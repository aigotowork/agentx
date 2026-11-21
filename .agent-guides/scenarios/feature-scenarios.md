---
schema: scenario
description: Maintain feature usage scenarios and explain why step evidence supports product understanding, feedback and verification.
tags: [scenarios, product-development, feedback, evidence]
---

# Maintain a feature's scenarios and evidence

Use when designing, implementing or changing a feature, or diagnosing a defect in its behavior. Start from the affected feature's existing design, scenarios and contracts. Maintain the changed usage flow there, whether execution uses the existing native runner, manual work or focused tests. A small fix can update an existing scenario and link its test; it does not require another document or a full UI campaign.

## Why this is the foundation

A feature has value because someone can achieve a goal under real conditions. Code structure, a successful command and a finished screenshot each reveal part of that behavior. A maintained scenario connects the intended value to the sequence a user actually follows, including the conditions that make an action meaningful.

The purpose of retaining evidence is to let another person or Agent understand and question what happened after the original runtime and working memory are gone. They should be able to find the expectation, see the relevant situation, explain a discrepancy and verify a correction. Treat an observation as support for a stated claim, not as a verdict in itself. A complete set of files can still describe the wrong target, an obsolete expectation or a failed experience.

For a UI node, semantic layout explains the objects and their roles; relevant state explains how those objects are currently expressed; actual rendering shows the output available to the user. State is central to interpreting the other two, but a captured subset does not necessarily contain every cause. These views are complementary. Their relationship, timing and omissions determine what can be concluded. External effects, such as saving a file or sending a request, need their own evidence.

Reusable domain tools preserve this reasoning while removing repeated infrastructure work. The feature owner should supply the goal, state meaning and expectations. A maintained adapter should handle supported host setup, capture, retention and reading. Tool availability must not silently redefine the user's goal or turn unsupported observations into passing checks.

Use this reasoning to make decisions:

| Value or question | Decision it supports |
| --- | --- |
| Can the user complete the intended task? | Define expected behavior before accepting a run as a baseline; include the failure branch that could invalidate the goal |
| Can a later reader explain the result? | Retain the context and layers needed for that explanation, with source and identity; explicitly mark unavailable information |
| Does the evidence support the claim? | Choose the real surface and independent effect check; narrow the conclusion when capture is partial |
| Can the next feature reuse the mechanism? | Use the domain's callable recipe and consumer seam; improve a repeated collection gap at its owner |
| Is more work useful? | Stop when the affected expectations have adequate evidence and remaining uncertainty is explicit; collecting more files is not itself progress |

## Express and maintain the usage flow

Keep one canonical scenario with the owning feature. Reference existing requirements, state models and contracts instead of copying them into a second specification. The useful minimum is:

- A stable scenario identity, user/actor, goal and why it matters.
- Starting conditions, relevant data/environment and target identity.
- Ordered meaningful steps: action or event, expected observable behavior, and important state/effect invariants. Include pending, cancellation, failure or recovery when they change the outcome.
- For each step, the check or observation that could support or refute its expectation, and where its evidence is retained.
- Links to the implementing/test runner and to actual runs or focused checks. Keep unexecuted and unsupported portions visible.

Use prose, a small table or the feature's existing scenario format. This is a semantic requirement, not a new file format.

When a scenario crosses features, keep one source with the owner of the complete user outcome. Each participating feature references the criteria it helps satisfy; shared domain rules stay with their original owners. A split into separate scenarios must name the actual handover, transferred identity, prerequisite and failure continuation. Feature membership, user step order and execution dependencies are distinct relationships. Use [requirements coverage](../domains/product-design/requirements.md#write-promises-that-can-be-challenged) to explain which paths must hold together for the claimed result.

When the missing piece is how to author that upstream flow and hand its nodes to other owners, use the shared [single scenario source method](product-design.md#maintain-one-scenario-source), then [collaborative modeling](collaborative-modeling.md) when a domain contract needs derivation. When a runner needs a structured projection, map the original nodes and criteria to its supported public format and retain the selected source bytes; a new tool does not require rewriting an established scenario.

For broader handoff or change propagation, use shared [product design](product-design.md#deepen-modules-and-return-their-results) and [source retention](collaborative-modeling.md#retain-the-needed-basis).

For example, a cancellable edit can be described as follows. These rows are a design example, not recorded execution:

| Step | Action/event and expectation | Evidence question |
| --- | --- | --- |
| Choose target | Select B, then move focus to C; the edit target remains B | Do stable target identity, focus state and visible marks agree? |
| Edit | Enter a draft and change viewport size; target and draft survive | Are the state and visible layout consistent before and after resize? |
| Cancel | Cancel the draft according to the feature contract | Does the UI return correctly, and does the independent effect record show no save/send? |

Every meaningful step needs a coverage decision. Several micro-actions may share one checkpoint if it still explains their relevant outcomes. Reuse immutable unchanged artifacts by reference. Do not collapse a transient state into the final frame when the transient behavior is what needs verification. UI checkpoints plan all three layers, with a reason for each absent or inapplicable layer; non-UI steps use relevant input/output, state and effect contracts instead of manufacturing screenshots.

When behavior changes, update the affected goal, step, precondition or expectation and its runnable binding together. Preserve the previous scenario basis with historical runs. A changed expectation needs a product reason; do not rewrite it merely to make a failed run pass. For a bug, link the failing node and the new verification to the same intended behavior. For a removed flow, record its replacement or retirement so old evidence remains interpretable.

## Design cases before writing tests

Derive cases from the feature's intended behavior, contracts and state model before choosing test code or a runner. Unit tests protect a responsibility's rules and transitions; integration tests protect the participating boundaries and complete outcomes. Neither substitutes for coherent design.

Build a compact case table in the existing feature design or test specification. State the scenario, starting state and relevant state combination, event/input, expected transition/output/effect, invariant and failure/recovery expectation. Identify which case protects which promise and which layer owns it. Reuse existing cases and fixtures when they express the same guarantee.

| Coverage concern | Design question |
| --- | --- |
| Normal and boundary behavior | Which meaningful inputs, identities, limits and absent/empty values change the outcome? |
| State transitions | Which events are allowed, rejected or unchanged in each relevant state? |
| Interacting states | Can pending work, selection, edits, permissions or lifecycle changes overlap? What must remain true for each reachable combination? |
| Failure and recovery | What happens on partial effects, cancellation, timeout, duplicate requests or a late/stale result? Which cases actually apply to this contract? |
| Integration boundaries | Do producer and consumer agree on identity, data, state, effects and errors across the real boundary? |

Choose combinations because they exercise distinct mechanisms, change behavior or threaten a shared invariant, not as an exhaustive Cartesian product. Explain important gaps and unreachable cases. Cases verify the promised capability; each untested option is not automatically a new Task or feature. A count or happy path does not establish completeness, and an evidence gap does not establish a product defect. Route new findings through [workflow change handling](workflow-management.md#respond-to-changes-at-their-owner). Write tests against the intended contract rather than incidental implementation details.

Case authorship can precede the complete implementation batch; case order is not an instruction to implement and validate each file or operation serially. A red/green experiment for an uncertain behavior does not require that cadence for the entire domain. Case authorship alone does not select execution scope or override project ownership. Execute justified scoped checks within the implementation assignment; hand shared-resource or broader integration cases, fixtures and unexecuted status to their named owner. Follow the [execution limits](../principles.md#quality-tests-and-images) and [verification ownership](workflow-management.md#bound-verification-and-integration): checks justified by the outcome and affected guarantees, no routine reruns or ungrounded broad suites. Retain actual results separately from designed coverage; passing cases do not establish structural soundness or Delivery completion.

## Select an established domain recipe

When a task covers several user goals or an entire feature area, maintain a scenario inventory before running it. Group by meaningful capability or user purpose and give every declared member a coverage disposition. Keep each complete flow's goal, preparation, evidence and attempts separate; an overall report links them instead of concatenating unrelated actions into one chain. A missing or unexecuted member remains visible.

Grouping is a reading choice, not an execution dependency. If one flow needs another's result, name the prerequisite, transferred object/artifact, failure or unknown behavior and resource owner. Prefer steps within one scenario when continuity of the same live state is essential. Preserve original run identities and failed attempts when packaging results; report coverage, execution, evidence gaps and product judgments separately. Packaging history never executes those dependencies.

Use the affected domain entry: [TUI](../domains/tui/AGENTS.md), [VSCode](../domains/vscode/AGENTS.md) or [Web](../domains/webdevelopment/observation.md). Follow its actual tool README for supported versions, setup, invocation, resource lifetime and known gaps. Do not load every domain or the research history.

A usable recipe supplies the capture mechanism and a clear place to bind consumer-specific meaning. Before creating infrastructure, identify what the existing recipe accepts: scenario/step identity, actions, relevant state, semantic region mapping and expected checks. If one adapter binding is missing, add that bounded binding under the actual owner's authority and retain the result for the next consumer. Record an unavailable capability honestly; a list of candidate tools is not an implemented recipe. Do not run an unrelated fixed demo as acceptance for the changed feature.

For UI evidence apply [observable feedback](observable-feedback.md) to capture, correlate and inspect the layers. Preserve existing runner identifiers and schemas. Business scenarios stay with their consumers; shared tools own reusable mechanisms.

## Judge the result and close the loop

Compare each affected expectation with its actual evidence. Separate observation from explanation and product judgment. Record supported, contradicted or still-unverified claims using the original runner's vocabulary. Tool success means the tool completed its operation; it does not mean the scenario is correct or valuable.

If state says “selected B” but the visible mark is on A, investigate the state/render relationship. If a save reports an error after the write succeeded, use the independent receipt before deciding what recovery is allowed. If only a pure transformation changed, its focused contract test may be sufficient for the affected step. These choices follow the same purpose with different evidence.

Finish with a maintained scenario, reachable evidence for its affected steps, explicit gaps and a feedback or correction path. When the required evidence is unavailable, retain that uncertainty and its owner; do not declare the behavior verified. New verification links to the original problem and preserves its historical basis.

