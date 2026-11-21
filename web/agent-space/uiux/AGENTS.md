# Space design resources

Read the product [AGENTS.md](../AGENTS.md), [Space UIUX task](../../../.controller/office-details/space-uiux.task.md), and the local design-system guidance before changing these files. `systems/space/` owns semantic tokens, resource declarations, rationale, and executable scenes. Its scenes import the same public React components consumed by Space. They must remain deterministic, silent, and free of native queries or sends.

The canonical runtime entry is `systems/space/space.web.ts`; scene modules may import product presentation adapters and deterministic fixtures, but the public runtime entry must not import scenes, product-private runtime modules or service adapters. CSS is authoritative for tokens; no parallel token JSON exists. Keep the `SpaceTheme` portal boundary outside its `space-shell` sizing container so fixed fact placement remains in viewport coordinates. Preserve pure `.ts` compatibility imports used by existing Node test paths.
