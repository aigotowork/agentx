---
schema: task
status: completed
description: Migrate the existing visual Agent Web experience without its external runtime integration.
---

# Agent space Web migration

## Outcome and authorization

The user requested migrating the existing Web capability from `/home/long/works/longwork/paseo-bots` into AgentX. Preserve the visual Agent work/conversation overview and the ability to open an Agent conversation, including both scene and chat presentations already supported by the source. Migrate source/assets rather than replacing the product with a newly invented dashboard. Remove Paseo integration, runtime packages, terminology and concepts from the executable application. Required upstream licenses and provenance remain factual historical records.

API design is independently owned by gabi. No integration, new server, native API design, Rust change, deployment, installation switch or live service mutation is required. Source repository is read-only. Do not delegate to existing Agents.

## Responsibility and workflow

One coder owns the cohesive frontend migration: inspect the source, record the selected transplant structure and source inventory, materialize boundaries, migrate reusable UI/assets and isolate transport from presentation. Destination: `web/agent-space/`, independent of existing `web/management`, `web/workbench` and `web/observability`. The coder selects exact internals based on the source and owns design/implementation/ordinary repairs. Keep sufficient actual migrated source and assets to preserve the recognizable existing experience. Do not import legacy server/shared application wholesale merely to satisfy old imports.

Use a clearly identified offline fixture/local adapter, with no invented real-agent replies or implicit network calls. Opening conversations, selecting agents, switching scene/chat views, drafts and message composition should be usable locally; local-only operations must be distinguishable from delivery to live Agents. Keep UI-facing boundaries replaceable after gabi's API is ready, without prematurely defining AgentX service contracts.

Implementer writes: `web/agent-space/**` and `implementation.md` in this directory. Prefer module-local scripts/config and existing root dependencies. Ask the controller to coordinate any required root manifest changes. Preserve all unrelated work. No additional delegation.

Controller writes: this Task, root discoverability entry and final result acceptance. Independent browser/build execution will be assigned after a usable implementation exists; implementer may run scoped static checks for development but should avoid duplicating final execution.

## Completion

- A runnable isolated Web module with source-linked migration inventory, preserved assets/notices, concise README and AGENTS entry.
- Visual scene overview and chat presentation; agent selection opens readable history and composer; selection/drafts survive view changes; fixture mode is visible and truthful.
- No runtime imports/packages/configuration or network endpoints tied to Paseo; no private source data/config copied.
- Required scoped typecheck/build plus actual browser interaction on desktop and narrow viewport, executed once on final inputs by the assigned verifier. Evidence and remaining limits go here or linked implementation notes.
- Fresh Agents can discover current scope, migration source and future API integration boundary.

## Accepted local outcome

The offline Web transplant and user-requested interaction repairs are complete. [Implementation](implementation.md) owns the source/provenance and retained/deferred capability inventory; [independent verification](verification.md#final-independent-result--completed) owns the executed build/browser result and limitations. The actual source scene/art, chat presentation, reader/composer and local state are usable under `web/agent-space/` with no legacy runtime integration or root dependency changes. Real API integration remains deliberately deferred to gabi's separately owned contracts.

Two substantive interaction defects were repaired and independently verified: opening a conversation preserves the existing camera zoom instead of forcing a magnified background; short-height scene reading keeps the composer reachable. Related close/mode/resize/panorama continuity, per-Agent drafts, local note semantics and desktop/narrow paths pass. Final scoped typecheck/build passed. No unresolved substantive defect remains within the documented Linux local Chrome scope. Browser scripts/evidence are discoverable through the module test README.

## Closed assignments

Controller zumi accepts the declared migration and repair boundary from halo (`coder`, batch `36522c05`) and finite independent execution from mofe (`coder`, batch `a30d545c`). Both assignments are closed; no further dispatch or testing is required for this outcome. Executor-owned browsers/static servers are cleaned up; the user-owned LAN preview below remains running and returned HTTP 200 at closure.

## Verification handoff preparation

Independent execution should exercise scene and chat selection continuity, per-Agent drafts, open/close conversation, local message submission and truthful offline delivery state, desktop and narrow viewport, and absence of external API/network requests. Follow module-owned build scripts once supplied. Existing project browser tests use `/usr/bin/google-chrome` and Playwright installed at `/home/long/.npm/_npx/e41f203b7505f1fb/node_modules/playwright`; verify tool availability before relying on those paths. The repository Vite builds force the portable WASI Rolldown binding because the native binding uses unsupported CPU instructions on this host; see `web/workbench/build.mjs`.

## User LAN preview

The user explicitly requested a running LAN preview. zumi started the source Vite server with runtime host override `0.0.0.0:18967`; `http://192.168.2.178:18967/` returned HTTP 200. PID 3050612, exec session 32599. Preserve this running user resource; it is separate from mofe's disposable acceptance server. Source configuration remains loopback by default; no native runtime/API services were changed.

## User-observed interaction repair

The user reports that selecting an Agent makes the scene background unexpectedly huge, and authorizes fixing related concrete bugs and appropriate interaction improvements. Halo owns diagnosis and coherent camera/layout repairs within the same frontend assignment. Mofe owns reproducing before/after scale and retained office context on desktop/narrow layouts, plus directly related open/close, view-return and resize cases. Preserve established evidence elsewhere and the running LAN preview. This is added to the existing finite verification, not an API scope expansion or broad redesign. The previously observed short-height composer reachability defect remains with the same owners.
