---
schema: scenario
description: Preserve semantic layout, relevant state and rendered results at meaningful UI checkpoints, then use them for feedback and verification.
tags: [feedback, observation, ui, evidence]
---

# Observe, retain and use a UI checkpoint

Use when designing or changing a UI workflow, diagnosing a rendered or temporal failure, recording a user journey, or collecting evidence for human/Agent feedback. Select the affected scene and risk. A pure function change with adequate contract tests does not require a UI campaign. Keep the method and evidence in the owning Task or consumer scene; do not create a second product database.

Read the affected application's entry and current public contracts. Apply the relevant [Web](../domains/webdevelopment/observation.md), [VSCode](../domains/vscode/AGENTS.md) or [TUI](../domains/tui/AGENTS.md) route for tools. Reuse context already read.

Apply the [test and image limits](../principles.md#quality-tests-and-images) before choosing tools. This method does not make routine implementation a test run or a screenshot review. Image reading is limited to UI design or reviewing its fit to user intent; name that question and explain the image after reading it. State or contract diagnosis uses the relevant structured evidence without opening images by default.

For an early, evolving surface used to discuss a complex design model, apply [visual model communication](interactive-design-communication.md). The result is a portable componentized HTML file; serve it through LAN HTTP when shared access is needed. For ongoing frontend implementation, apply [continuous preview](../domains/webdevelopment/continuous-preview.md). Those methods own visibility and update timing; this method owns retained checkpoints and evidence.

## Purpose and judgment

Start from the maintained [feature scenario and its purpose](feature-scenarios.md). The checkpoint preserves the situation needed to judge one of its expectations after the live process is gone. Semantic layout tells the reader what the objects mean; state helps explain their current expression; rendering shows what was actually presented. Comparing them reveals failures that any one view can hide. For example, correct selection state does not establish that the visible mark is on the correct row.

Choose capture from the question being answered. The three layers are complementary context, not three automatic approvals. A complete capture can still fail the scenario, and a declared gap can leave one claim unverified while another is supported. Preserve enough original evidence and association to let a later reader challenge the conclusion. This is why the method records provenance, timing and effects, and why it does not demand unrelated full-memory or every-frame dumps.

## Plan three complementary layers

Before execution, state the user's task, expected result and one failure that would invalidate it. Choose meaningful checkpoints: a prepared state, an important transition, a pending/error state or an outcome. Several small actions may lead to one checkpoint. Do not equate every event with a requirement to dump all memory or capture every frame.

For every declared checkpoint, specify how each layer will be captured and represented. Record a reason when a layer is unavailable or inapplicable; share an immutable artifact reference when it is unchanged. Missing evidence is not a passing check.

| Layer | Retain or reference | Check its limits |
| --- | --- | --- |
| Semantic layout | Regions, roles, components/controls, relevant hierarchy, object identity and source/design links | Label intended design separately from observed runtime structure. A React tree, DOM/AX tree or control map is not necessarily the visible spatial layout |
| Relevant state | Inputs and data required to interpret the scene; selection, focus, draft, phase, revision and pending work where relevant | Identify the owner and sampling boundary. Props alone do not capture arbitrary Hook state, external services or process resources |
| Rendered result | Actual outputs such as terminal bytes/cells, DOM/AX, bitmap, frame sequence or recording; retain original artifacts and environment | Name the capture route. An emulator image, browser viewport and OS window prove different surfaces. A file reference without bytes does not prove capture succeeded |

An artifact can contribute to more than one layer. Reference it once and explain both contributions; do not duplicate data just to fill three folders. A UI screenshot cannot stand in for private state, and an application snapshot cannot establish actual pixels.

Keep expected criteria, actions, external effects and environment alongside the layers. They explain what happened and whether it met the goal. E2E can contribute actions, traces and effects as well as pictures. Storybook contributes state inputs and executable Stories; preserve actual runtime changes separately when the question depends on them. Inspection contributes observed components, named state and visual facts within its supported scope.

## Establish identity and capture boundaries

Use the existing runner/host's records. The following are semantic requirements, not a new shared DTO or a replacement for any product schema:

| Question | Required information when relevant |
| --- | --- |
| What run and basis? | Run/attempt, scene version, build/source, fixture and environment |
| What happened? | Stable step and action identity, actual input path, outcome or unknown result |
| What object? | Original resource and instance identity; component/source reference is additional context |
| Which observation? | Checkpoint identity, producer, capture interval/sequence, model/section/frame revisions and coordinate system |
| What is retained? | Original content/reference, format, source, partial/truncated/redacted/missing reason |
| What can be reconstructed? | Inspection-only artifacts, explicit rerender inputs, or a reproducible execution plan; list missing prerequisites |

Prefer a controlled completion event or revision over a fixed sleep. Record boundaries separately: action admission, application completion, output flush, terminal/browser processing and image capture. Do not call independently sampled state and images atomic. If they cannot be reliably related, expose the mismatch and narrow the claim.

Capture pending/transient states through controlled fixtures or bounded sampling when they matter. A final successful picture can miss a wrong intermediate target. Record resize, navigation, scope changes and asynchronous completions in their actual order. Time proximity alone does not prove cause or common identity.

## Execute the smallest useful closed loop

1. **Prepare.** Choose the real host and tool for the claim, a known starting state, explicit version/environment and owned resources. Keep fixtures separate from user data. Write criteria before accepting the produced output as a baseline.
2. **Drive.** Record whether the input was a model event, API injection, browser/renderer action, PTY input or OS interaction. Wait for the intended observable condition. Input accepted is not an external effect receipt.
3. **Observe and retain.** Capture the planned layers at each meaningful checkpoint, plus the independent effect needed by the criterion. Preserve raw artifacts before producing summaries or differences. Record failed capture and known gaps.
4. **Inspect for the declared question.** Read the saved semantic/state artifacts. For UI design or user-intent review, open the actual image or recording and explain the visible hierarchy, content, actions and state against the intended task, including discrepancies and limits. Otherwise leave images unopened and make no visual-quality claim. Hidden DOM text is not visual readability; a status field is not proof of persistence.
5. **Explain and discuss.** Record observation, expectation, user impact and evidence references separately from possible causes and proposed changes. Use the current feedback mechanism selected by the consumer. Keep automated verdicts and human product decisions distinct.
6. **Correct the cause and retain the result.** Repair the owning design/implementation or explicitly revise the expectation. Author the affected cases; produce a new run only within the declared integration or user-requested execution scope. Preserve the old result and link versions. Do not overwrite a failed baseline to imply a fix.
7. **Close.** Retain portable evidence and known limitations, close owned processes/subscriptions and report cleanup. Stop when the selected claims have sufficient evidence; do not expand to every host or historical campaign by default.

Use `pass`, `fail`, evidence insufficient, not applicable or disagreement as appropriate to the original runner. Do not coerce these into one overall success. Attribute a failure to product, fixture, driver, capture or environment before deciding the next action. A lost write reply, capture timeout or cancelled wait does not authorize repeating the business action.

## Read history, rerender or execute again

| Need | Action and preconditions | Claim |
| --- | --- | --- |
| Understand a past node | Open retained artifacts and original criteria; follow bounded references | Historical inspection; no action replay or renewed live authority |
| View saved state through a renderer | Use explicit compatible code/dependencies, inputs, environment and a side-effect-free rendering entry; create a new artifact identity linked to the old node | Reconstruction under declared conditions; not the original pixels or original process |
| Reproduce the full behavior | Start a new owned run with the scenario, fixture, services and current action authority; record nondeterministic inputs and differences | New execution; not restoration of the old run |
| Resume a still-live controlled journey | Revalidate its current target/state and original controller's continuation policy | Continuation only where the runtime contract permits it |

Never infer resumability from JSON serializability. Closures, sockets, in-flight requests, process handles and external side effects require their actual owners. Default a saved-evidence viewer to inspection-only. A reconstruction must show its derivation and differences, not silently replace the historical result.

## Keep evidence output separate from its inputs

An evidence writer must not modify the sources it describes. Resolve its explicit output destination once, keep it distinct from input/source paths and reject overlap before writing. Use an owned temporary file, parse the resulting artifact, then replace only the declared destination. When preservation is part of the claim, compare the affected sources with retained original bytes after the write; an inventory collected beforehand does not prove the writer left them intact. If an overwrite occurs, retain the damaged artifact, restore from an exact retained source where available and record the actual correction and limits.

## Preserve derived and offline results

A saved research summary, index or export projects original facts; it must retain their selected scope, snapshot, sources, omissions and evidence limits. Do not refresh it by silently navigating the live source, substitute latest data for a historical observation or turn an unavailable read into a failed operation. An associated request and a nearby image do not by themselves establish causality or business completion.

For an offline handoff, include the material required by the reader's promised task: records and relationships, referenced artifacts, readable source/interpretation distinctions and explicit missing or online-only dependencies. Declare the supported copy/import/read boundary. A file existing on disk, a complete-looking inventory and a discoverable consumer entry are separate claims.

Resource reading, execution, observer waiting and byte retention have independent lifetimes. Closing a reader releases its own observation; it does not cancel an executor or delete evidence. Query/reconcile an original uncertain operation only through its actual recovery contract. Archive, unregister and physical purge need their owning dependency/scope/recovery rules; do not apply identical CRUD to every noun.

## Keep the evidence useful

Expose a short checkpoint summary, then the selected layers and original artifacts. Preserve the full available data separately from the initial display. Select fields needed for the question and document omissions; avoid dumping credentials, unrelated user data or an entire private store. If a collector changes timing, retain that limitation.

A feedback record should let another reader identify the scene, original target, action, discrepancy and next verification step without reading the chat transcript. Link existing source and evidence owners. Current comments can change while their historical basis remains fixed. Current-file feedback, inline discussion and historical acceptance have distinct guarantees; choose the existing mode deliberately.

When a repeated failure exposes an observation gap, improve the narrow collector/scene or owned rule. Extract shared mechanisms only after actual consumers demonstrate compatible needs. Do not turn one successful fixture into a promise of arbitrary UI restoration.

