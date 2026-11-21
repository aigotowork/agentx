---
scope: Management and Space synthetic target prototypes
status: candidate
owner: design-system
related: ["../../.controller/web-product-design/tasks/design-system.task.md", "../../.controller/web-product-design/delivery.delivery.md"]
---

# Shared interface foundation

The common interface behavior is direct: locate the subject, understand its state, take a deliberate action, and return to the same work. Management scans row-aligned resource and receipt facts. Space finds people through a list or spatial map and keeps the same conversation and draft. Keep their page structures and business Blocks local.

[Management style](management.style.design.md) · [Space style](space.style.design.md) · [Operable specimen](../../.controller/web-product-design/preview/system/index.html) · [Task and evidence](../../.controller/web-product-design/tasks/design-system.task.md)

## Foundations and consumption

Current Management’s [stylesheet](../../web/management/styles/management.css) provides the precedent for compact cool surfaces, clear selection, and short state labels. Current Space’s [UIUX foundation](../../web/agent-space/uiux/systems/space/foundation.design.md), [layout](../../web/agent-space/uiux/systems/space/layout.design.md) and [system direction](../../web/agent-space/uiux/systems/space/space.design.md) supply scoped warm surfaces, a durable reader/composer, and a separate art layer. Carry these principles forward. The redesign Delivery selects a portable synthetic static host, so the shared preview uses web platform primitives and adds neither a UIUX runtime nor a parallel catalogue to the current products.

Use these links from either sibling preview directory. The application owns its document and responsive geometry; put the system class on the primitives to adopt their appearance.

```html
<html lang="en" data-ax-style="management">
<link rel="stylesheet" href="../system/system.css">
<button class="ax-button" data-variant="primary">Create request</button>
<span class="ax-badge" data-tone="warning">Outcome unknown</span>
```

CSS is the single token source: base type/spacing and semantic roles → style/brightness mappings → component classes. Both styles support light and dark. Default is Management light; use `data-ax-style="space"` and/or `data-ax-brightness="dark"` on the document or a nested region. A nested region that declares a style establishes a complete boundary; specify both attributes when it should be dark. A native top-layer element stays in its DOM ancestry, so overlays keep their source boundary. Product previews can pin their selected style; the specimen exposes both axes independently and preserves work during switching.

| Roles | Public contract |
| --- | --- |
| Surfaces, text and accent | `--ax-page`, `--ax-surface`, `--ax-surface-subtle`, `--ax-text`, `--ax-muted`, `--ax-border`, `--ax-accent`, `--ax-accent-soft`, `--ax-on-accent`, `--ax-focus`; state tones map through the primitive variants below. |
| Type, rhythm and geometry | `--ax-font-body` for reading; `--ax-font-mono` for aligned identifiers and times. `--ax-text-{xs,sm,base,lg,xl}` provides the hierarchy. `--ax-space-{1..8}` is a 4px rhythm, `--ax-control` a minimum control size, `--ax-radius` the control corner. Use `--ax-panel-radius` for enclosed surfaces, `--ax-rule` for dividers, `--ax-focus-size` for focus outlines and `--ax-flyout` as the default compact disclosure width. Apply local aliases for other product geometry. |
| Original line icons | [icons.svg](../../.controller/web-product-design/preview/system/icons.svg): `search`, `info`, `close`, `back`, `arrow-right`, `panel`, `check`, `warning`, `list`, `grid`, `send`. Use `<svg class="ax-icon" aria-hidden="true"><use href="../system/icons.svg#info"></use></svg>`; name the enclosing control. Canvas characters and illustrations stay Space-owned. |

## Layouts

| Layout | Use and ownership |
| --- | --- |
| Resource split `ax-split` | `ax-split-index` and `ax-split-detail` are direct children. At an available width of 44rem they sit side by side. Below, set `data-view="index"` or `"detail"` to show one sibling and expose a contextual return; without it, both stack in document flow. This is presentation only: state, selection, return and focus belong to the composer. Choose explicit scroll regions in fixed-height hosts; a normal document scrolls itself. |
| Reader `ax-reader` | Direct header/body/footer: the `ax-reader-body` can scroll inside a height-constrained parent. Keep focusable composer controls in the footer. Use `ax-stack` and `ax-cluster` for local vertical rhythm and wrapping action rows. |
| Modal or edge drawer `ax-dialog` | Dialog owns viewport bounds and a single `ax-dialog-body` scroll region when composed with `ax-dialog-head` and `ax-dialog-actions`. Use for actions that need a bounded step or confirmation; use plain inline disclosure if users need to compare or keep working in the parent. |

## Controls and disclosures

| Primitive | Contract and guidance |
| --- | --- |
| `ax-button`, `ax-input` | Native link/button for navigation/action and labeled input/select/textarea for editing. Button variants: default, `primary`, `quiet`, `danger`. Set `aria-pressed` for an independent toggle; use `aria-current` on the current route. Do not style unavailable actions as actionable. Pair real `disabled` with nearby fact/reason when useful. For icon-only buttons add `ax-icon-button` and an accessible name. Errors use `aria-invalid` with associated text. |
| `ax-panel`, `ax-row` | Flat hierarchy and readable divisions, applied to native structural elements. Put actions next to their owning facts and wrap long identifiers. Avoid nested cards for routine facts. |
| `ax-badge`, `ax-notice` | `data-tone="neutral\|info\|success\|warning\|danger"` changes presentation, never meaning. Name the state in words; distinguish empty from unavailable, stale from live, and unknown result from failure. Use `role="status"` for a result or progress announced after user action; reserve `role="alert"` for urgent errors. |
| Inline detail | Native `details` and `summary` (optional `ax-details`). It changes local reading only. Focus remains on the summary when toggled. |
| Compact popover `ax-popover` | Native `popover="auto"` with a trigger using `popovertarget` (or module `togglePopover(trigger, panel)` to bind a programmatic invoker). Use for related facts or a short set of ordinary links/buttons. Browser light dismissal keeps the underlying task available, with Escape and focus return when closing from inside. Tab moves through ordinary controls; do not add ARIA menu semantics without implementing that keyboard model. |
| Dialog / modal drawer | `<dialog class="ax-dialog" aria-labelledby="title">` with optional `data-placement="drawer"`. Import `openDialog(dialog, trigger?)` and `closeDialog(dialog, returnValue?)` from `../system/disclosure.js`. Use `autofocus` on the safe first action or heading and `method="dialog"` for explicit cancellation. Browser blocks background interaction and handles top-layer Escape and focus restoration while the invoking element survives. A product that replaces the opener must supply a new focus target. Opening, closing or changing appearance has no domain effect. |

```html
<button id="show" class="ax-button" aria-haspopup="dialog">Review request</button>
<dialog id="review" class="ax-dialog" aria-labelledby="review-title">
  <header class="ax-dialog-head"><h2 id="review-title">Review request</h2></header>
  <div class="ax-dialog-body">Exact subject and effect go here.</div>
  <form class="ax-dialog-actions" method="dialog">
    <button class="ax-button" autofocus value="cancel">Cancel</button>
  </form>
</dialog>
<script type="module">
  import {openDialog} from '../system/disclosure.js';
  const show = document.querySelector('#show');
  show.onclick = () => openDialog(document.querySelector('#review'), show);
</script>
```

Platform target: current evergreen browsers with native Dialog and Popover. CSS anchor positioning is progressive; browsers without it show a bounded fixed facts panel. Native modal and popover APIs own Escape; consumers must avoid a competing global Escape handler. If a parent traps Escape for product route return, it must defer first to the active native disclosure (and to IME composition). No appearance setting is persisted by the foundation.
