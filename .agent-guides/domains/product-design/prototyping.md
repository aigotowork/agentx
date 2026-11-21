---
schema: scenario
description: Choose prototype fidelity and controlled data that answer a decision without overstating simulated behavior.
tags: [product-design, prototypes, fixtures]
---

# Make a prototype answer a question

Use when a product decision requires a visible or operable candidate. State the question, target task and what observation would change the choice. Reuse existing components and scenarios where appropriate; prototype completion is not validation completion.

## Choose the dimensions that must be real

| Dimension | Preserve when relevant | Misleading omission |
| --- | --- | --- |
| Structure | Object groups, entry, sequence, return and hierarchy | One isolated panel called a complete flow |
| Content | Real terms, representative lengths, counts, units and status meanings | Placeholder text mistaken for a navigation problem |
| Behavior | Actions, guards, pending, errors, interruption and recovery | Every click succeeds, hiding the question |
| Visual and temporal expression | Density, size, focus, latency and motion relevant to perception | Static end frame used to judge control or continuity |
| Service reality | Explicit simulated, manual, sandbox or actual effects | A simulated success called durable saving |

Choose the least costly material that preserves the factors driving the judgment. Sketches support grouping/concepts; clickable navigation supports finding and returning; a stateful prototype supports recovery; an actual host is needed for URL/history, native input or integration guarantees. A video can convey timing but cannot establish a user's control of it.

## Prepare coherent states and data

Distinguish domain meaning, view data, fixture, UI state, service substitute and environment. Fixtures supply examples; they should not implement a second business rule set. Select typical content, legal boundaries, permission/lifecycle differences and timing cases only where they could change the conclusion. Zero, absent, forbidden and failed reads are different states. Mark intentionally invalid data as fault input.

Keep identity, time and relevant randomness reproducible. State what a reset restores. Preserve at least one continuous flow as well as direct entries into hard-to-reach states; disconnected screenshots cannot expose lost context or timing. Placeholders retain region responsibility and representative content, but cannot replace the exact behavior under investigation.

Choose the substitute boundary deliberately: view input tests presentation; a local service substitute tests client state; a network substitute includes the SDK/request path; a real sandbox can test the connected service. Document intercepted requests, response meaning, delay, effects, reset and undeclared-request behavior. Do not accidentally fall back to production or successful responses. Clear owned timers/caches and fence old responses on exit.

For a save example, preserve draft and operation identity through pending, confirmed rejection and unknown outcome. Cancellation of waiting does not cancel the effect. A later attempt and a logical operation need distinct identities. The real domain contract decides whether query or retry is permitted; an always-successful mock cannot create that guarantee.

Use ordinary state/reducer code for bounded behavior. Configuration may select fixtures and destinations; nested arbitrary expressions, effects and error handling constitute a program needing a maintained interpreter. Extract an engine only after repeated compatible consumers justify it.

## Observe and consolidate

Freeze the candidate version and task inputs. Apply [evaluation](evaluation.md) and the [checkpoint method](../../scenarios/observable-feedback.md) for the selected claim. Stop to repair material if the task is impossible, reset is unreliable or a simulation is misleading. Keep unavailable portions visible; do not fill them with claimed results.

Choose whether exploratory code is disposable or intended to evolve with canonical components. Hand implementation the adopted decisions, invariants, fixtures, discarded alternatives and unproved service guarantees. When real contracts change, revisit affected prototype conclusions; do not keep an obsolete demo as a second behavior authority. Use [Storybook](../webdevelopment/design-system.storybook.md) for its supported component/Screen scope, and actual application hosts for host responsibilities.

Sources for further depth: [GOV.UK prototypes](https://www.gov.uk/service-manual/design/making-prototypes), [moderated task observation](https://www.gov.uk/service-manual/user-research/using-moderated-usability-testing), [Storybook mocking](https://storybook.js.org/docs/writing-stories/mocking-data-and-modules/mocking-network-requests). A carrier's features do not establish validity of a particular simulation.
