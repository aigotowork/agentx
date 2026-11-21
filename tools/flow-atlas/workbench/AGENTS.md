# Domain workbench UI

For linked collaboration/sequence maintenance, start at the [completed source Task](../../../.controller/domain-daemon/tasks/flow-atlas-linked-ui.md#structural-contract-2026-11-11-source-author), [accepted parent outcome](../../../.controller/domain-daemon/tasks/flow-atlas-linked-workspace.md#accepted-linked-workspace-and-publication) and [current experience design](../design/experience.md#联动工作区本轮源码设计). The linked slice is published separately on18961 with offline examples. Controller executed acceptance; source authors ran no commands. Both assignments are closed with all claims released. Preserve18957–18961 listeners and fixed artifacts. Future maintenance uses affected module contracts and cases; historical source-author execution restrictions do not impose another approval or review stage.

Read [tool guidance](../AGENTS.md), [local UI requirements](../../../.agent-guides/scenarios/interface-design.md), [Task](../../../.controller/domain-daemon/tasks/flow-atlas-domain-ui.md) and [README](README.md). This module owns portable offline presentation and JSON embedding only. The canonical semantic contract is [domain/contracts.ts](../domain/contracts.ts); checkpoints are authoritative exported facts, never reduced again here.

| Change | Owner |
| --- | --- |
| Workspace/layout/tokens | `index.template.html`, `workbench.css` |
| Safe DOM, semantic references | `dom.js` |
| Same-ID visual projections | `views.js` |
| Responsibility lanes, complete branches, scenario/applied-prefix projection | `collaboration.js`, using `views.js::projection` primitives |
| Paired/single composition, narrow tabs, occurrence references and independent pane cameras | `linked.js`, `linked.css` |
| Local details/provenance/checks | `details.js` |
| Presentation time | `player.js` |
| Presentation-only node movement | `layout.js`, consuming shared `../viewer/layout.js` read-only |
| Context and event composition | `app.js` |
| JSON-only build/receipt | `build.py` |

The browser must not evaluate source, invoke native methods, infer unknown remote effects or mutate artifact facts. Metadata is text; only explicit http(s) documentation URLs are links. Source paths remain provenance text. Keep shared camera/layout implementations canonical. Preserve stable `data-testid`, `data-ref-kind`, `data-ref-id`, `data-node-id`, `data-port-id`, `data-step-id` and `data-binding-id` hooks when changing internals.

One `app.js` context/selection and one `WorkbenchPlayer.create` serve all panes. Pane view/camera are presentation state; selection never forces projection replacement. Sequence requires a scenario and maps geometry/animation by call identity, not step identity. Exact reference metadata retains process/step/scenario/visit/call/phase; repeated step definitions remain ambiguous until an occurrence is chosen. Only applied checkpoints supply caller/effect/state/parameter evidence; future visits may supply explicitly labelled scenario input. Both builders embed the same presentation modules; the experience wrapper may override object/binding geometry, never app/player semantics. The detached `state().linked` inspection contract and stable hooks live in the source Task; no artifact facts or writable runtime handles escape through it.

Motion dots/path geometry stay in `views.js`; `linked.js` owns the bounded pane-heading illustrative payload disclosure (`linked-payload-cue`, full safe `<pre data-testid="flow-payload">`). Do not put long moving text back on operation/branch labels. Newly visible panes reveal the retained exact occurrence after mounting, without fitting/resetting their camera or changing presentation time. Responsibility headers use namespaced per-lane clipping plus glyph-width budgeting; preserve their full accessible title/name and inspector facts.

For this Task, source authors do not execute builders, Node/Python/tests/browser/server. Send source releases and authored cases to the independently assigned acceptance owner. That owner alone generates artifacts and records rendered evidence; this file does not impose those temporary execution bounds on unrelated future tasks.

This implementation Task is [completed](../../../.controller/domain-daemon/tasks/flow-atlas-domain-ui.md#accepted-ui-outcome-and-closure). Current Linux local/static-browser support, final readability/native-binding derivatives and publication are independently [accepted](../../../.controller/domain-daemon/tasks/flow-atlas-domain-acceptance.md#accepted-final-derivatives-and-publication). The README's case table is authored coverage; executable results remain with the acceptance owner. The [published offline artifact](../examples/domain-workbench.html) and [intentional demo server record](../../../.controller/domain-daemon/flow-atlas-workbench-server.json) own their precise resources. Preserve that server and earlier user/demo resources. Receipt/source snapshots describe the fixed accepted source; subsequent receiving documentation does not alter executable artifact inputs or warrant rerunning unchanged checks.
