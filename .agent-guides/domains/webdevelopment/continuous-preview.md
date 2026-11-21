---
schema: scenario
description: Keep frontend design and implementation visible through a stable shared preview and deliberate updates
tags: [web, preview, development, feedback]
---

# Continuous frontend preview

Apply to all frontend page design and implementation, including prototypes, redesigns and bounded fixes. Establish a reachable preview before substantial private implementation, then keep it available as the work progresses. Reuse the actual affected page or existing story; a small change does not need a new preview application. Compose [visual model communication](../../scenarios/interactive-design-communication.md) when discussing new models or complex alternatives; that shared method owns the visual model and discussion loop.

## Establish the observation surface early

Inspect the project's supported run command, runtime/data profile and current page entry. Start or reuse a server reachable on the user's LAN and share the real page URL with the first useful layout or behavior slice. Keep the primary task, representative state and current integration limits visible. A successfully started process is not a usable preview until its page loads and the relevant control works.

Expose the first coherent composition with representative content, then keep that same preview available while authoring the related pages, Blocks, contracts and behavior in substantial batches under [whole-domain authoring](../../principles.md#whole-domain-authoring). Preview availability does not require a screenshot, browser run, review or report after each component or file. Share meaningful operable checkpoints; do not hide the work until final polish or let the preview stay stale. Use an early interaction probe when it answers a concrete design uncertainty, then concentrate rendered/behavior checks after the declared batch.

## Stable manual refresh

Prefer an open page that stays stable while code changes, with ordinary manual reload loading the newest usable frontend code. Enable HMR when the user's task benefits from it or they request it; do not let hot updates, dependency optimization or automatic reload repeatedly interrupt selection, typing, inspection or scenario exploration.

Use the framework's supported configuration before adding a custom mechanism. Establish the full update path:

| Boundary | Required behavior |
| --- | --- |
| Open browser document | Edits do not replace its UI or discard its exploration/input state unexpectedly |
| Development server | New navigation/reload reads current source or the latest successful build, rather than a stale in-memory transform |
| Browser cache and assets | Entry HTML is revalidated or not stored; changed asset URLs/cache rules prevent mixing old and new versions |
| Build-backed dependencies | Rebuild required outputs or explicitly resolve development entries to source; cache invalidation cannot compile unbuilt code |
| Backend/configuration | Identify changes requiring a service restart; refreshing a page does not reload Node module state or server configuration |

Disabling HMR alone does not establish freshness. Inspect the installed framework's behavior and verify both promises: the open page stays unchanged, and normal refresh gets the changed version. Include a relevant missed-watch or delete/rebuild case when the project consumes generated packages. Keep existing draft persistence/recovery guarantees; manual refresh itself does not guarantee arbitrary unsaved application state survives.

For Vite projects, `server.hmr: false` can suppress browser updates while the watcher invalidates module transforms. If rebuilds can escape watcher invalidation, invalidate the affected server module graph on document navigation before cached transforms are served. Do not broadcast reload messages. Use source aliases only for the actual development consumers, and keep the production package/build boundary intact. Treat this as a version-dependent recipe, not a universal framework contract.

A no-build HTML discussion page can use the [portable static example](../../scenarios/examples/design-discussion/README.md). A source-backed app should use its existing dev server. Keep the project's exact implementation, restart recipe and regression command with that project rather than copying its port, paths or storage into shared instructions.

## Verify and maintain the loop

Use a focused browser check at the affected entry: open the page, enter representative text or select an object, change the source, confirm the current document/state stays stable, then manually reload and confirm the new version is loaded. Check the changed interaction and relevant narrow viewport. Distinguish development freshness from production deployment/update guarantees.

Share useful checkpoints as they become operable and continue authorized work without waiting for acknowledgement. If the preview breaks, restore a usable slice or identify the exact gap and next recovery action; do not silently continue a large invisible batch. During handoff, identify the current URL, source/run command, actual verified behavior, remaining simulations/integration and whether the server remains available.
