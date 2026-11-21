---
schema: task
status: completed
owner: design-system
description: Turn Folio and Signal into a maintainable switchable Space presentation layer and bind the refined detail assets once delivered.
links: [../AGENTS.md, ./art-polish.task.md, ../../../web/agent-space/uiux/systems/space/candidates.design.md]
writes: [../../../web/agent-space/uiux/systems/space/]
agents: [{"scope":"/home/long/works/longwork/agentx/.agentx","generation":"a6195a5b-1dd4-4a6d-87d4-1f18f1b664fb","participant":{"canonical_name":"bavola","incarnation_bits":4745315960788295064}}]
---

# Space theme integration

## Outcome

The user wants to switch between multiple UI directions while seeing a coherent, finely crafted Space experience. The existing Folio and Signal candidates already share the real App/controller/SDK path but need a proper presentation boundary and art-detail binding.

## Context

Read the candidate design, existing UIUX module guidance and the art-polish Task. Inspect the current implementation and the accepted browser captures. Preserve the default theme and exact conversation behavior. Treat theme as presentation state. Domain facts, exact Agent identity, capability gates, drafts, route state and native effects remain application-owned.

## Responsibility

Own the complete design-system contribution under `web/agent-space/uiux/systems/space/`. Establish a small stable theme registry or equivalent semantic presentation contract if the current string switch is insufficient. Keep CSS tokens and component composition coherent across Scene, Directory, Reader and CommandSurface. When the art owner hands off refined SVGs, bind them through a semantic asset role used by both the product and the UIUX candidate, preserving the existing fallback. Avoid new prose, transport access, duplicate components and per-style conditional trees.

## Acceptance

The two styles can be selected from the product switcher and URL, survive scene/list/reader transitions, and render the same exact target and draft. Candidate Canvas exposes both styles. A focused desktop and 390px run checks contrast, focus, Esc/Cancel, keyboard/touch return and no new API writes or reads from theme changes. Record changed files, remaining style limitations and evidence in this Task.

## Design-system handoff

The UIUX boundary now exports `SpaceStyle` (`classic | folio | signal`), ordered `SPACE_STYLES`, `parseSpaceStyle`, and the controlled `SpaceStyleSwitch({ value, onChange, compact? })` from `systems/space/space.web.ts`. `SpaceTheme` treats omitted/`classic` as the original CSS baseline and applies candidate selectors only to Folio or Signal. `ConversationBlock.presentationControl` is an optional narrow-only row, container-scoped below 760px with 44px controls; App owns the actual selected value, URL/local preference and Back/Forward behavior. Kula owns wiring this slot through `src/conversation/Conversation.tsx` and whole-app verification.

The delivered `folio-page-edge.svg` and `signal-panel-edge.svg` are bound to the shared semantic `conversation-trim` role at the history/composer join. Folio repeats horizontally; Signal renders one intrinsic 160×12 copy at the leading edge. The authored SVGs remain in the prototyper-owned `src/scene/art/studio/refined/` directory. The existing composer border remains underneath as fallback; optional seal/corner/indicator assets are intentionally not applied because the accepted readers leave no stable clear space for them.

Changed system entries: `components/presentation.ts`, `components/SpaceStyleSwitch.tsx`, `components/SpaceTheme.tsx`, `blocks/ConversationBlock.tsx`, `styles/candidates.css`, `space.web.ts`, `space.design.md`, `conversation.design.md`, and this Task. Prior accepted browser evidence remains [compare-02](../../../.artifacts/space-ui-styles/compare-02/report.json) and [default regression](../../../.artifacts/space-ui-styles/default-regression/report.json); captures show the intended trim join on both readers. Current source checks: `uiux check web/agent-space` passes with 17 resources and 4 candidates; `node web/agent-space/scripts/run.mjs typecheck` passes; the asset URL resolves to the prototyper-owned file. No new product build or browser run is claimed here. Product URL/history wiring, live no-extra-read/write behavior, 390px operation, focus/contrast review after the new trim, and production adoption remain Controller-owned verification. Native SDK contracts and services were not changed.

## Completed product integration

App owns the desktop and narrow switches using [the appearance helper](../../../web/agent-space/src/navigation/appearance.ts). Explicit URL choice wins on entry, and in-app changes update the current URL and save the browser preference. The final composition uses the two seams, Folio fibers and command seal, Signal command corner, and five interchangeable [Studio scene finishes](../../../web/agent-space/src/scene/art/studio/finish/README.md). It uses the existing controller and public SDK, with a read-only Studio HTTP fixture.

The [published browser result](../../../.artifacts/space-ui-styles/refined-02/report.json) passes for desktop and 390px, including exact reader/draft retention, appearance preference, product modal and local Canvas confirmation. See the [current comparison and preview links](../../../web/agent-space/uiux/systems/space/candidates.design.md#preview-and-evidence) for the exact builds and screenshots. The earlier handoff above describes the design-system contribution before app integration. No production adoption is included.
