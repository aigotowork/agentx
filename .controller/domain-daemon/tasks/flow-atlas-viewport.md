---
schema: task
status: completed
description: Replace finite SVG scroll surfaces with a shared container-sized pan/zoom viewport and document the existing lint contract.
links: [../flow-atlas-implementation.md, ../../../tools/flow-atlas/AGENTS.md]
---

# Container-sized canvas and lint maintenance

## User finding and cause

The user found a bounded drawing surface, mismatched DIV/SVG edges and surrounding blank space. Both viewers use content-sized SVGs with automatic margins inside scroll containers. This is a spatial ownership defect: world content bounds were being used as viewport dimensions. Model/lint scope is unchanged.

## Bounded design

A shared spatial viewport owns camera x/y/zoom and DOM navigation; the host DIV owns visible dimensions. SVG viewBox has exactly the host aspect ratio, fills the host and exposes no sheet boundary. Content can be panned outside its bounds. Camera scopes are view/mode/scenario or inspector view/process, independent of business snapshots and node layout. Resizing retains center/scale; wide initial entry fits actual content; narrow initial entry preserves readable scale and explicit fit requests overview; remounts retain camera.

Blank primary drag, middle/Space drag, wheel zoom at cursor, touch blank pan/pinch and keyboard zoom/fit are navigation. Heading drag remains a layout edit; Port/state clicks retain their meanings. Navigation cancels an active node gesture before changing its coordinate frame. Overlay controls provide zoom out/in, percentage/100% and fit. Empty canvas background is continuous and aligned to world coordinates.

Use one reusable plain-JS/CSS viewport for both offline consumers. No dependency install, framework migration, source/domain model change, new service or subagent needed. This is one cohesive spatial interaction correction with existing architecture. Existing actual lint implementation remains; add a maintenance rule index linking exact source/API/tests, configurability and uninstrumented-source limits.

## Cases and completion

Check SVG and DIV edges/aspect at desktop and390px; pan beyond all content bounds; pointer-cancel restores camera; cursor zoom anchor; fit/100%; view switch and repaint retain camera; heading drag under zoom still uses world units and Port paths terminate exactly; canvas navigation does not change playback checkpoints; touch blank pan/pinch and object access; no body scrollbars; model bytes unchanged. Reuse unaffected type/model/lint evidence. Existing artifact-only server identities remain; publish tested HTML only.

Evidence root: /tmp/flow-atlas-viewport-l28y1971.

## Accepted viewport and documentation result

Both offline viewers now use the same container-sized spatial viewport. Desktop and390px browser cases passed under Chrome147.0.7727.101: exact DIV/SVG bounds and aspect, no sheet/body overflow, unbounded pan, cancellation, pointer-anchored wheel zoom, keyboard navigation/100%/fit, scope/repaint retention, scaled object editing and exact Port endpoints, paused marker continuity, touch pan/pinch and direct contract selection. Final execution is `browser-r4/result.json`; all browser contexts/processes are closed in `browser-r4/closed.json`.

Visual assessment checked whether the drawing surface still looked like a separate sheet and whether objects remained readable. Desktop content and viewport controls fit the existing working regions; the grid fills the container to its edges. The first narrow overview reduced runtime text to31% and design text to48%, so initial narrow entry now starts at85% anchored to content top-left. Users can pan for adjacent objects or explicitly fit the whole graph. Final narrow screenshots show readable object names, Port rows and reachable viewport controls, with no document overflow. They establish the inspected initial compositions, not every possible camera position or physical touch device.

Two useful early failures remain: fractional CSS dimensions exposed clientWidth rounding, corrected by measuring the actual bounding rectangle; the wheel test then assumed fractional injected coordinates, corrected to observe actual native WheelEvent coordinates. Prior passed r3 coverage is historical; r4 includes the subsequent readability change and passed affected cases. No speculative framework or routing changes were added.

The existing dependency/design lint was already implemented and independently accepted in the prior Delivery. The new [rule index](../../../tools/flow-atlas/authoring/lint.md) links actual code, API, severity/cycle policy, CI use, source/test ownership and limitations. Existing13 design/13 trace/18 schema/five native-outcome checks are reused unchanged; this viewport follow-up does not pretend to scan arbitrary TS imports or prove uninstrumented source behavior. Not every diagnostic code has an individual executed counterexample.

## Publication and resource handoff

`published-artifacts.json` records final exact artifact/preservation/resource facts. Repository runtime HTML is282623bytes/SHAca7d6fe4ca9c4d75ec6c9581c0c41f190baca5d0be1df75305ad32a636220e3c; design HTML is41348bytes/SHAc56ee2c578812f7c5975312e0556f72681b19c2c8192054f8003bd66ca36fbeb. Both equal the browser-tested candidates and actual loopback/LAN response bytes. The unrelated request example was also regenerated from the same shared sources; it is not claimed as a separately browser-executed fixture in this follow-up.

The existing servers retain exact PID/start ticks: runtime48797/933076796 and design1175428/937240519. No new listener was created; both intentional user demos remain available. Runtime/request/design model hashes and root package/lock hashes match preservation. No product executable, user .agentx, installed/default program, Provider or protected demo operation occurred. The tool/module/runtime guides and prior implementation Delivery link this superseding viewport result. Source/artifact identity and final checks are retained in `final-handoff.json`; no work or transient execution resources remain.
