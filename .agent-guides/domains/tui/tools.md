---
schema: scenario
description: Select terminal observation fidelity and correlate application, framework and rendered output with owned resources.
tags: [tui, observation, tools]
---

# Select and correlate terminal observations

Choose the claim before capture. Use [observable feedback](../../scenarios/observable-feedback.md) for original identity, three layers and retained evidence. This guide supplies terminal-specific decisions, not a new runner protocol.

| Question | Useful mechanism | What remains unproved |
| --- | --- | --- |
| Did selection/draft/state transition correctly? | Existing model snapshot or framework test | Actual terminal parsing and pixels |
| Which components, props and Hooks were present? | Selected development renderer with actual DevTools inspection | Product meaning, physical layout and external state |
| Can an external command complete a fixed-size visible interaction? | [Black-box terminal driver](black-box-testing.md) | Hidden state, raw history and native window fidelity |
| What did the child write and how was it parsed? | Owned PTY plus pinned headless terminal parser, or owned tmux pane | OS font/compositor and business effect |
| What does retained output look like? | Labelled cell-grid SVG/PNG or browser terminal render | Original OS pixels |
| Does the actual terminal input/theme/font work? | Exact owned terminal window, OS input and capture | Unobserved internal state or transitions |

ANSI-stripped output is not a screen: cursor movement, overwrites, buffers, attributes and resize matter. Preserve raw bytes before a projection when available; a viewport-only driver cannot supply missing scrollback. Respect wide/combining characters, normal/alternate buffer, cursor and observed dimensions. Clamp extraction to the viewport rather than the backing buffer's retained width.

## Prepare an owned consumer

Read the installed adapter's README, lock/runtime/ABI requirements and consumer contract. Supply explicit executable, cwd/environment/data authority, scenario, output and private runtime roots. The release may be read-only. Keep application fixture meaning with the consumer; use the existing adapter for PTY/parser/capture plumbing. Native dependency failure is a specific load/ABI/platform gap, not a successful string-render substitute.

For the installed observable-evidence Ink adapter, its public entry is runScenario from @harness/observable-evidence/ink. Supply the consumer-owned scene and explicit absolute outputRoot, runtimeRoot and consumer; keep both writable roots disjoint from each other and from the installed tool. Follow the installed package's consumer contract for current support and failure behavior.

For Ink, require an actual supported renderer/backend pairing. The studied pairing was Ink 7.1.1, React 19.2.0, react-devtools-core 7.0.1, node-pty 1.1.0 and xterm/headless 6.0.0; this identifies source limitations, not proof of your installation. Read the selected release's exact compatibility notes before adapting private protocols. Do not discover a user's default DevTools port or alter production debugging state.

## Inspect actual framework data

Initialize the real DevTools backend before React/renderer import where required, on the owned transport. Confirm renderer registration and actual component operations; a connected socket or backend-ready message does not establish a tree. Request inspection using the returned renderer/element identity. Preserve actual `inspectElement` and `inspectedElement` records, full/partial/error/no-change status, and dehydrated fields without inventing values.

Compare relevant props/Hooks with the application's own state and output. Authored region/role maps remain design annotations linked to observed component IDs, not automatically measured layout. A fixture with unique component names and quiescent state is a limited consumer contract, not arbitrary attachment to any app. Read-only bridges should not admit state editing just because the DevTools protocol supports it.

For the studied Ink 7.1.1 adapter, load Ink with DEV unset to avoid its import-time default-port probe, connect the private backend, and enable development mode only before constructing the inspected instance. This is a pinned fixture technique, not a production configuration. The selected adapter must own any such version-specific setup.

Version-specific traps must be addressed through the adapter: Ink can probe a default backend at import and register its reconciler only for a development instance; DevTools can emit additional root operations. Backend connection alone therefore cannot prove inspection. Fail visibly on unsupported protocol data and retain it for diagnosis instead of presenting a plausible fabricated tree.

## Capture and judge a checkpoint

Wait for a named application revision/completion, then renderer flush, parser processing and image capture. An owned test seam may emit a marker after flush and correlate it with the separately returned state revision. Keep this sequence explicit; it is not automatically an atomic snapshot. Without a completion seam, use bounded visible-condition sampling and state the weaker association.

Capture integrity and product assertions are different stages. Prefer retaining available evidence before evaluating the product so a failed assertion leaves a readable scene; if the selected collector cannot retain that failure, state the gap. A screenshot timeout never authorizes repeating a business action.

Read the saved image only for UI design or user-intent review. Explain what the visible content, hierarchy, actions and state mean for the intended task; otherwise use structured state/grid evidence without opening images. Check both expected content and a forbidden remnant relevant to the risk: resize can leave stale rows while positive text checks pass. Do not prescribe remount/clear-history universally; preserve the application's state and lifecycle contract. Record whether pixels are synthesized or from an OS window.

## Adapt and close

Textual can expose Pilot/widget state and synthesized snapshots; Bubble Tea can expose Model/Msg/Update/View; Ratatui can expose state/events and Buffer/TestBackend. These are framework boundaries, not PTY or native terminal evidence. Keep the shared questions and use the supported framework seam rather than porting React private protocol everywhere.

Read saved state/regions/actual framework data/grid/image together. History reading acquires no target and forwards no old input. Rerendering needs explicit compatible code/fonts/inputs and a new derivative identity; reexecution needs fresh resources and current authority.

Stop input admission, settle accepted work, unmount the renderer where owned, restore original terminal modes, await original child exit, close observer/bridge resources and remove only private runtime files. Retain timeout, forced termination or unknown cleanup accurately. Saved PIDs and sockets are evidence, not future process-control authority.

Further APIs: [React DevTools backend](https://github.com/facebook/react/tree/main/packages/react-devtools-core#backend-api), [Ink 7.1.1](https://github.com/vadimdemedes/ink/blob/v7.1.1/readme.md). The selected installed version and adapter own executable details.
