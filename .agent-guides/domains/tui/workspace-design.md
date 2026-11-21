---
schema: scenario
description: Design terminal workspaces with low interaction burden, useful information density and contextual controls.
tags: [tui, design, usability, information-architecture]
---

# Design a terminal workspace around the user's work

Use when designing or reviewing a TUI page, command interaction or workflow. Optimize two outcomes together: reduce what users must remember, type and navigate; keep the information and actions needed for frequent work available in a few coherent workspaces. High density means more useful decisions per visible region, with a clear reading order. It does not mean filling every cell.

Start from the product's existing [feature scenario](../../scenarios/feature-scenarios.md), current contracts and actual screen. For a bounded change, revise only the affected decisions. Use [interaction](interaction.md) for focus, state, asynchronous behavior and verification, and [command-line design](../../scenarios/command-line-design.md) when changing the command grammar or automation surface.

## Find the avoidable work

Walk one frequent task from entry to result. Record the information needed to decide, the objects selected, values typed, context switches and outcome the user must recognize. Identify burdens such as copying an ID from another page, remembering a hidden shortcut, re-entering a known setting or reopening a list after a small action. Replace those steps where the current data and authority permit it. Do not add fields or confirmation merely because the implementation exposes them.

Organize primary pages by sustained user tasks, such as finding work, observing activity and editing a substantial resource. Keep filtering, sorting, selecting, short editing and immediate actions in the workspace where they are needed. Introduce a separate page when its task needs independent navigation, extensive reading or sustained editing. Do not mirror every API entity with a mandatory page, or impose a universal page count.

## Allocate information before decorating

For each main region, name the user's question and the action it supports. Put identity, current status, important relationships and actionable exceptions in the normal scan path. Give the largest region to the primary work: event content in an activity view, comparable rows in a selector, or the document in an editor. Use compact summaries and a stable action area for supporting context.

- Prefer readable names. Add a short address or distinguishing context for duplicates; retain exact identity internally and expose technical IDs on demand.
- Group by meaningful criteria such as team, state, role or recency. Use group headings to avoid repeating the same value on every row. Support search when the inventory outgrows easy scanning.
- Keep the information required for a common decision visible. Move tutorials, repeated explanations and rare diagnostics to contextual help or details. Preserve important errors, limitations and unavailable-action reasons near their target.
- Use aligned fields, spacing, separators, short timestamps and counts to make comparisons easy. Keep full content reachable when truncating. Show the timezone or complete timestamp when interpretation requires it.
- Reserve color and emphasis for differences that matter. Normal names and prose use a neutral foreground, such as white on a dark background or the terminal's readable default. Avoid saturating entire rows merely because one status is active.

Use a small semantic vocabulary of shapes and colors. Encode different dimensions independently: a square for selection, a dot for runtime status, a diamond for a special controller role, and a cursor or background for keyboard focus. A soft yellow diamond is one possible role marker, not a universal requirement. Derive a role marker from actual role data, never a suggestive display name. Do not imply model progress, readiness or success merely from a running process.

Keep symbols recognizable and consistent, with a compact legend or accessible short label where needed. Color must not be the only distinction. Test the supported color profile and glyph width; provide simple fallbacks when symbols are unavailable. Concise words remain useful for unfamiliar concepts, target names, errors and consequences. Replace repetitive text with structure, not with a code the user must memorize.

## Prefer recognition and selection to transcription

| Available information | Preferred interaction | Preserve |
| --- | --- | --- |
| Existing objects or finite values | Select, radio, checkbox group or searchable picker | Names, distinguishing context, valid choices and the exact selected identity |
| Many or remote choices | Search/filter with bounded results and explicit loading/empty states | Query, selection, hidden-selection count and stale-result handling |
| Several recipients or targets | Visible multi-select with count and names; optional parsed shorthand such as mentions | Final target preview, duplicate resolution and no silent partial action |
| A safe common configuration | Visible, editable default from project/context | Source and scope when consequential; explicit overrides |
| A new name, message or custom value | Focused text input with useful examples or completion | User text, validation and recovery of the draft |
| No usable choices | Explain why and offer the relevant create/configure/refresh action | No fake placeholder choice or silent selection of an unavailable object |

Show just enough context to choose correctly: role, state, location or membership, depending on the task. A list of opaque IDs is not a useful selector. Place filtering inside the picker rather than requiring the user to leave and look up the value. Do not overwrite deliberate edits when defaults refresh.

Apply the same judgment to interactive CLI commands: when an omitted argument has discoverable choices and a TTY is available, offer a focused picker where consistent with the command contract. Preserve explicit flags/arguments and stable machine-readable output for scripts and Agents. Without a TTY, use valid configured defaults or return the missing input and how to supply it; never hang on a prompt or silently choose the first candidate. A friendly interactive path must not make automation depend on terminal keystrokes.

## Complete small actions in place

| User intent | Suitable surface |
| --- | --- |
| Read and navigate current work | Persistent primary workspace |
| Switch scope, choose a target or set a small option | Anchored picker or compact popover |
| Write a short message while reading activity | Bottom composer that keeps useful context visible |
| Review a consequential action or a short form | Local dialog with target, effect and clear confirm/cancel controls |
| Read long technical content or perform extensive editing | Dedicated detail/editor view with reliable return state |

An overlay should preserve orientation while making its own focus clear. Capture input inside it; background rows must not respond to clicks through the overlay. Escape cancels or closes the local interaction and restores the original cursor, selection, filter and scroll position. Define draft retention explicitly. Limit stacking: prefer replacing a small step within one dialog to accumulating nested dialogs. Adapt or expand on small terminals so the field, target and submit/cancel controls remain reachable.

A smaller surface does not reduce the meaning of the action. Keep target preview, eligibility checks, confirmation where warranted and result/recovery semantics. Pending work must not retarget the action. Unknown completion keeps its original request and a way to inspect the result, rather than inviting a second submission.

## Make controls discoverable and consistent

Provide ordinary navigation before mnemonic shortcuts: Up/Down within lists, Left/Right within horizontal groups, Tab between regions, Space for multi-selection, Enter to open or activate, Escape for the nearest local cancellation. Left may return when no local horizontal control owns it. Do not steal cursor keys or Space from an active text editor. Give every frequent action a visible control or menu entry; shortcuts accelerate that same action.

Keep a compact, context-sensitive key strip and a fuller help view. Avoid requiring function keys or terminal-sensitive combinations as the only route. Mouse clicks should focus or select the visible target predictably; scrolling should affect the pointed region. Rendered positions and hit testing must share layout geometry, including overlays, scrolling, wide characters and resize.

Separate cursor, selected objects, observed scope and action targets. Make selection counts and final targets visible, including selected objects hidden by a filter. Changing a recipient must not silently change what events are being watched. Preserve original object identity across reorder or same-name replacement.

## Keep the workspace calm and adaptable

Refresh data without replacing stable content with loading text on every poll. Use a small progress indication when it adds value, and a stable error with a recovery action when it matters. Preserve drafts, focus and reading position; if new events arrive while reading history, expose a count or resume-follow action instead of forcing a jump.

Allocate panes using both width and height. Similar work panes should receive comparable usable area; choose rows, columns or a grid based on what can still be read and operated. Keep a management/sidebar region large enough for its task, or provide an explicit compact mode with an obvious expand control. At narrow sizes, reduce repeated metadata first, then adapt regions or scroll; do not clip the primary target or trap the user without an exit.

## Judge the design through a complete task

Run the affected flow from finding a target through acting and understanding the result. Include the boundary most likely to fail: ambiguous names, empty choices, hidden selection, slow refresh, a narrow terminal or cancellation after editing. Use the existing observation tools and retain the relevant state, rendered grid/image and independent effect evidence. Open the retained image when judging visual readability; unit state assertions alone do not establish it.

Compare the result with the original burden: can the user choose without copying identifiers, see the relevant status and action, complete the operation without losing context, and distinguish saved, pending, failed or unknown outcomes? Verify supported keyboard and mouse paths, an editor boundary and a resize where affected. Describe any remaining limitations. Stop when the selected scenario works; do not turn a local improvement into a mandatory full-product redesign or a claim of universal usability.
