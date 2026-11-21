---
schema: scenario
description: Verify terminal focus, selection, editing, async updates and resize through correlated checkpoints.
tags: [tui, interaction, state, observation]
---

# Follow a terminal interaction across change

Use when a change affects focus/selection, editing, asynchronous completion, screen size or view lifetime. Follow the application's existing interaction and effect contracts; use [tools](tools.md) for the actual capture route. The [shared checkpoint method](../../scenarios/observable-feedback.md) owns the general feedback loop.

For page composition, visual meaning, selectors and local overlays, apply [workspace design](workspace-design.md). Reuse the existing design for a bounded interaction fix.

## Define the task and invariants

Separate the navigation cursor, selected business objects, active editor and OS window focus. Give selection a stable object identity, not a row index, display label, PID or cell rectangle. State who owns async revisions, which stale results are rejected, and what pending/error/unknown means.

For the smallest affected scene, maintain the user goal, starting data, input path, step expectations and why each observation can support a judgment in the feature's existing design/test context. Record which semantic regions and state values explain the visible result; do not require a full application dump. A useful representative flow is: focus B → select B → edit a draft → begin refresh → resize while pending → complete a reordered result → exit. Run it through the installed adapter's public consumer interface when its fixture contract fits; supply explicit scenario, relevant state, semantic regions and expectations.

| Transition | Check | Evidence |
| --- | --- | --- |
| Focus/selection | Cursor and selection refer to the intended identity; visual markers agree | State and attributes/markers in the same checkpoint grid |
| Editing | Input reaches the intended editor, Unicode handling matches its contract, selection survives | Relevant Hook/app state, original input route, rendered draft |
| Pending refresh | The old value is labelled appropriately; pending does not erase draft or retarget selection | Explicit pending checkpoint rather than only final success |
| Reorder/replacement | Reordering keeps the original identity; a same-name replacement does not inherit authority | Full object key/ref, before/after states and renderer mapping |
| Resize/view change | Dimensions agree; important content is reachable; no duplicate/stale frame remains | Ordered resize/output records and a viewed grid/image |
| Exit/error | Known effects stay known; owned terminal and process resources settle | Original-owner cleanup receipt and terminal buffer/mode observations |

For an asynchronous save, preserve the pending and failure steps when the question is whether the user can understand the outcome and recover the original draft. A controlled fixture can check empty validation, known rejection, draft retention, editing and resubmission; synthetic completion proves only those UI semantics. A real persistence claim needs the service's independent effect evidence; another screenshot does not supply it. Unknown completion never authorizes replay. If only pure validation logic changed, test that contract directly and stop unless rendering or input behavior is also in question.

## Drive the right layer

Use direct model events for invariants and controlled stale-result ordering. Use framework input tests for component behavior. Use actual PTY bytes for terminal input decoding and resize behavior. Use OS interaction only when testing the real window/input path. Label each route; success in one does not establish the others.

Do not force impossible schedules into an end-to-end story. If the actual session serializes refreshes, test old/new completion ordering in its model seam and report the serial session separately. Use controlled async fixtures to retain a pending checkpoint. A timeout or missing screenshot never authorizes retrying a business action.

## Compare and finish

Check state against DevTools/framework observations and rendered output. Positive presence checks alone can miss stale rows, duplicated selection, hidden content and incorrect colors; choose a negative assertion for the concrete risk. Read an actual retained image only for UI design or user-intent review. Explain its visible content, hierarchy, actions and state against that intent, and record whether it is synthesized or captured from an OS window; otherwise make no visual-quality claim.

Use immutable run artifacts for history. Demonstrate that the history reader only reads files and that a changed selection/rendering discrepancy is detectable. Do not add broad fuzzing or every-terminal acceptance when one bounded scene answers the question.

Finish when the selected behavior has appropriate semantic, state and rendered evidence, relevant gaps are named, and owned resources have known cleanup results. Keep accepted product behavior separate from fixture success. Link the original failed checkpoint and the later verification in the task or existing feedback record.
