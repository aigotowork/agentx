---
schema: scenario
description: Choose task surfaces and define address, return, focus and interruption semantics that survive real navigation.
tags: [web, navigation, recovery, accessibility]
---

# Choose the surface and preserve the task

Use when a task needs a new entry, return path, deep link, overlay, cross-route draft or responsive flow. Start with task span, simultaneous information needs, interruption and recovery promises, entry sources and devices. [Experience design](experience-design.md) owns the overall task; components implement the selected navigation contract.

First read [product and interface design requirements](../../scenarios/interface-design.md). Their scenario hierarchy determines local disclosure versus a new destination; a narrow viewport alone does not justify splitting the task. Reuse this context when already read.

| Surface | Appropriate responsibility | Contract to preserve |
| --- | --- | --- |
| Independent route/Page | Stable task or object entry, sharing, refresh or external arrival | Load without prior memory; inaccessible/not-found behavior and an exit |
| Screen within a Page | Meaningful stage or view of one task | Entry guard, switching and retained state |
| Split view or inline expansion | Compare/reference while retaining context | Selection/scroll relation and narrow-screen continuation |
| Tabs | Peer perspectives of one context | Focus versus selection, activation and loading delay |
| Modal | Bounded decision requiring temporary attention | Focus entry/containment/return, cancellation and draft handling |
| Drawer/popover | Contextual reading or operation | Actual modal/nonmodal behavior, closing and return; appearance does not decide semantics |

Page/Screen names describe responsibility, an overlay describes presentation and a route is a host mechanism. Desktop dialog and direct-entry page can share an editor only through common object/draft ownership, cancellation and return semantics. Long editing with outside reference work may need a navigable surface rather than an inescapable modal.

## Define address and state ownership

For a meaningful destination, state entry source/object identity + address context + loading conditions → visible task state → exit/return + retain/clear rules. Use ordinary prose or a small table; no new routing schema is needed.

A shareable object ID or query can belong in a URL; the service still authorizes the actual read. Temporary focus/scroll/return origin can belong to navigation history. Drafts and in-flight operations belong to their task owner; only promised cross-refresh recovery requires durable storage. Do not put credentials or whole drafts in addresses, or assume all filters must be shareable. Normalize invalid/default parameters according to the public route contract.

Keep browser Back, logical parent navigation and Cancel distinct. A new destination may push history; normalization or each edit to the current query may replace it. History traversal restores declared address/context, not business effects. Breadcrumbs go to a logical parent, which need not be the previous history entry. Cancel follows the actual draft/effect contract; closing a view does not revoke a dispatched command.

Use an internal return context only while it is valid. Direct/external entry or a vanished caller needs a meaningful fallback. Never trust an arbitrary `returnTo` URL as an authorized destination. Components should call their host's navigation interface rather than independently reconstructing history policy.

## Challenge the same contract through several entries

| Path | What must remain coherent |
| --- | --- |
| List → detail → return | Relevant filters, selection and useful scroll/focus |
| New-tab deep link | Load the named object, explain location and provide a fallback exit |
| Inaccessible or stale object link | Follow disclosure policy and show a valid next action |
| Leave during submission and return | Original operation identity, known/unknown result and actual recovery; no duplicate submission |
| Refresh/restart | Only the recovery actually promised and supported, including draft limits |

A URL containing an ID cannot restore a socket, draft or unknown effect. Closing/crashing a browser is not reliably prevented by a confirmation dialog. If required recovery is absent, return that capability gap to requirements/architecture instead of hiding it behind a warning.

## Preserve focus and responsive continuity

Decide focus by task change. New main work needs orientation and a meaningful landing point; a local refresh usually preserves the active control and announces relevant change without stealing focus. A modal requires appropriate initial focus, containment and return; if its invoker disappeared, select a logical next location. Tabs distinguish keyboard focus and selected panel; delayed panels can need manual activation. Follow the adopted [WAI dialog](https://www.w3.org/WAI/ARIA/apg/patterns/dialog-modal/) and [tabs](https://www.w3.org/WAI/ARIA/apg/patterns/tabs/) patterns through actual behavior, not ARIA attributes alone.

Map the same task across wide/narrow/zoom layouts. Keep core information and controls in the working viewport through layout adaptation and local disclosure; reference content and occasional actions can use drawers, popovers or collapsible regions. A separate destination requires a substantially different scenario or working context, not merely a narrower breakpoint. Preserve the object, draft, necessary actions, errors and return context; a breakpoint must not silently remount away the draft. Preserve semantic reading/focus order after visual reordering and make scroll ownership clear. Use [browser observation](observation.md) to exercise the affected input, state and rendering boundary; a static image cannot establish navigation or recovery.

Further platform mechanism: [History API](https://developer.mozilla.org/en-US/docs/Web/API/History_API/Working_with_the_History_API). Its capabilities do not select the product's navigation or persistence promise.
