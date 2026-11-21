---
schema: document
description: Web methods for semantic layout, state and rendered checkpoint evidence.
---

# Web observation and feedback

Use for a browser UI's development feedback, component-state review or dynamic journey evidence in the consuming project. Apply the [Web development domain](AGENTS.md) for design/implementation and the shared [checkpoint method](../../scenarios/observable-feedback.md) for observation and retention. These methods compose; this entry does not replace frontend guidance or an application's current contract.

Start at the owning consumer's `AGENTS.md`, scenario and existing runner. Keep scenario fixtures and run evidence with that consumer. Use the selected component inspection producer only when component/source/state/visual observation is needed.

Maintain the [feature scenario and its value](../../scenarios/feature-scenarios.md) before choosing a driver. Browser automation is useful because it can connect user actions to observable results; component/state inspection helps explain those results. Neither a component operation nor a screenshot alone proves the user could complete the flow or that a save persisted. The scenario determines whether to use an isolated Story, a real navigation path or an independent effect check. Keep that reason beside the chosen capture plan so the next Agent can adapt it instead of copying a sequence without its purpose.

## Choose a scene and tool

| Situation | Use | Retain and inspect |
| --- | --- | --- |
| Review an isolated component, page state or bounded interaction | Existing Storybook Stories or the consumer's state-preview entry; controlled args/fixtures and actual interaction | Story/fixture/version, state inputs and relevant runtime changes, actual screenshot and criterion. Args are not a dump of all internal or external state |
| Exercise navigation, submission or recovery in the real browser | Existing Playwright/E2E runner | Actual actions/trace, page/context identity, DOM/ARIA, screenshots, related state and independent service/file effects |
| Explain which mounted component or state produced a visual result | Current Inspection producer/client/public workspace, scoped to the admitted Document/session | Component/source identity, bounded named state, observation gaps and time/revision association with the visible result |
| Diagnose layout, focus, clipping or accessibility | Browser geometry/CSS and DOM/ARIA, actual bitmap, keyboard path; selected accessibility checks | State and layout before/after the action. DOM text and automated accessibility results do not establish all visual or assistive-technology experience |
| Discuss a retained finding | Existing browser/Workbench/Feedback route, or current consumer-owned file feedback | Original run/checkpoint and evidence links, observation/expectation/impact and explicit decision |

Use the consuming application's existing runner and the installed observer/recorder README for setup, supported versions and output roots. Bind a real scenario; do not run an unrelated fixed example as acceptance. Project/Document/session selection precedes component reads, and hosts retain connection and action authority.

## Bind the three layers to one checkpoint

For an async save scene, prepare a controlled pending response and an independently readable saved result. Before the first write, declare the expected target, allowed effect count, draft retention and visible pending/success behavior. Then record:

- Semantic layout: selected page/region, component/control roles, runtime component identity and design links. DOM, AX and React trees may describe different relationships.
- State: relevant fixture/props, draft, selected object, pending phase, request/revision and current source. Only collect fields supported by the actual producer.
- Render: actual viewport/region image, viewport/DPR/fonts/theme and useful DOM/ARIA/geometry; include an event sequence where a single frame could miss the error.
- Context: step/action, capture intervals, mutation receipt or independent effect, expected/observed checks and missing sources.

Use controlled completion or observable predicates. Playwright actionability and retrying assertions help drive the browser; they do not prove business completion or product intent. Avoid `force` when obstruction is the property under test. A pair of identical screenshots can be a stable picture of an incomplete operation.

If Inspection and Playwright capture separately, declare the association limits. Verify the component and screenshot belong to the same admitted Document and scene; do not paste a new live state into an old node. A missing bitmap must remain missing. Inspection reads and explicit operations are different capabilities; selecting a component does not itself authorize a mutation.

## Keep execution and historical presentation separate

Use the runner's original scene/step/attempt identifiers and current artifact format. A consumer may attach a three-layer sidecar when a public runner has no native slot, but must label it as consumer-owned. Show criteria, actual actions, each layer, checks and gaps when browsing a node. Merely importing an E2E result does not provide guided Next/pause/takeover behavior.

For guided execution, require a callable runner/control contract with current admission and resource ownership. Importing a report is not implementation of Next, pause, takeover or business Feedback submission.

Read saved images only for UI design or user-intent review. Explain their visible hierarchy, content, actions, state and fit to the intended task before reporting visual conclusions; ordinary implementation leaves images unopened. Retain failed states, including late responses and navigation/context changes. For result-unknown writes, read the original result under its contract instead of repeating the action. BrowserContext isolation resets browser storage, not service state; the fixture owner must prepare and clean up its own resources.

Stop after the changed scenes and substantive failure paths have adequate evidence. Pure domain changes need scenario/state case design and authored tests, with execution governed by the [shared limits](../../principles.md#quality-tests-and-images); desktop extension claims must use the [VSCode domain](../vscode/AGENTS.md), not a Webview-only run.

Further official usage: [Playwright traces](https://playwright.dev/docs/trace-viewer), [ARIA snapshots](https://playwright.dev/docs/aria-snapshots), [visual comparisons](https://playwright.dev/docs/test-snapshots). Select versions supported by the real consumer.
