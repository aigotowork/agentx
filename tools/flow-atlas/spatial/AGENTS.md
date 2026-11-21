# Spatial viewport

Read parent tool guidance and the [viewport Task](../../../.controller/domain-daemon/tasks/flow-atlas-viewport.md). This shared camera is consumed by the trace viewer and design inspector. The container owns visible dimensions; SVG content bounds are only initial/explicit fit input. Camera `{x,y,zoom}` is presentation state keyed by scope, separate from domain snapshots and node layout documents.

`viewport.js` owns pure `zoomAt`/`fit` and the DOM adapter `create`. `mount(svg,scope)` borrows the just-inserted SVG and retains camera on remount; scope changes cancel active navigation. `beforeNavigate` cancels node layout gestures before camera movement. `dispose` releases listeners/ResizeObserver and owned controls. Do not introduce timers, domain mutations, persistence or inferred process behavior here. No content boundary clamps panning; zoom is bounded0.1–4 for usability.

Initial entry fits content on wide hosts. Hosts below600CSS pixels start at no less than0.85 zoom, anchored at content top-left with a24CSS-pixel inset; explicit fit still shows the entire content. Keep initial readability separate from explicit overview. The helper's measured dimensions include fractional CSS pixels so the viewBox never introduces letterboxing.

Targeted outcome cases are `tests/flow_atlas_viewport_browser.mjs`. Keep SVG viewport dimensions equal to host dimensions and aspect ratio; CSS must not restore content-sized sheets, automatic centering margins or competing scrollbars. Pointer node editing remains owned by viewer/layout-ui.js. Callback cancellation and zoom-aware coordinates protect that integration.
