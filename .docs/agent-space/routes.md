---
schema: document
status: active
owner: space-routes-dune
related: [../../.controller/space-routes/task.md, ../../web/agent-space/AGENTS.md]
---

# Space navigation

## Design and responsibilities

The browser URL addresses the existing complete Space. Native `URLSearchParams` and History API fit `/space/` static/embedded hosting without server rewrite or a new router dependency. `src/navigation/route.ts` owns parsing/serialization and state projection; `browser.ts` owns initial load, pending asynchronous restoration, history and cleanup; `SpaceController` owns atomic navigation transitions alongside existing page-owned drafts. App composes these with the unchanged context bridge and LiveSpace read adapters. No restoration submits operations, fetches Provider transcripts automatically, handles letters, launches Agents or uploads context.

| Actual surface | URL state | Consumer |
| --- | --- | --- |
| All offices / directory / global / unknown / legacy | `office` (empty means all) | OfficeNavigation / OfficeContext |
| Scene or chat | `view=scene|chat` | App |
| Browser presentation preference | `style` (a [supported appearance](../../web/agent-space/uiux/systems/space/components/presentation.ts)) | App appearance navigation |
| Members / stopped history / posts / Skills | `tab=members|history|posts|skills` | App / OfficeResources |
| Exact conversation and channel | `scope,generation,agent,incarnation,stopped_event,record`; `channel=messages|model` | Conversation |
| Current post detail | `post` (supplied opaque ID) | OfficeResources |
| Search and status filter | `q,state` | OfficeContext |
| Scene directory / human mailbox | `panel=directory|mailbox` | SceneSpace |
| Mailbox pending/history and page | `mail=pending|all`, `before` (opaque native cursor) | LiveSpace read adapter |
| Conversation facts disclosure | `detail=agent` | Conversation |

Absent values mean scene, members, no reader/panel, empty filters, messages, pending/latest mailbox. Exact references include record kind and both nullable native identity fields; no name fallback. Existing embedding queries remain valid: omitted record infers stopped only from stopped_event; unrelated query parameters are preserved without interpretation. Native IDs remain decimal strings; explicit offline demo references are synthetic strings and retain their exact record/generation. URL target scope/generation fence the native supply. Resource identity is resolved only from supplied facts; no inferred office from cwd.

## Lifecycle and history

Initial load or popstate parses and applies one atomic navigation snapshot. Valid office/view/tab/filter state can render before member/post supply arrives. Missing exact targets remain pending with bounded inline feedback, retained URL, and explicit load-more affordance; supply refresh retries without history writes. Scope/generation mismatch never selects a same-name successor. Missing posts do not silently show the first post. User navigation supersedes pending restoration. Draft edits/uploads/supply refreshes neither cancel it nor write history.

Deliberate navigation pushes one entry; search/status-only edits replace the current entry to avoid per-keystroke history. Defaults are omitted when writing canonical URLs; unrelated parameters and hash remain untouched. Invalid/duplicate owned parameters leave a useful alert and a safe default scene until a deliberate navigation replaces them. Back/Forward restores full state without adding entries. Closing a reader removes its exact URL target while per-exact-reference draft/channel storage remains page-owned. Refresh/new tab restores navigation, not ephemeral drafts or a previously fetched Provider transcript.

Camera, zoom, pause, reader size, hover/tooltips, source/provenance/tool/reasoning disclosure, launch relations pagination, connection/receipt disclosure, action dialogs, draft message options, draft text, local note bodies, uploads and operation receipts are excluded: these are transient presentation/action/local/private state. The meaningful object reader and its facts disclosure are addressable; nested content disclosures are not separate navigation destinations. Mailbox letter contents/handling are native facts and effects, never URL payload. Existing scene camera memories and context selectionRevision fences remain owned by their original modules.

## Consumer scenarios / independent handoff

1. Open `?supply=demo`; select nested office, chat, history, stopped mora, each record channel; copy URL into a fresh tab, reload, then Back/Forward through office/presentation/channel/close. Current and stopped mora must remain distinct.
2. Open posts, choose a non-first post, copy/reload; filter then reset; Skills and empty office restore honestly. Unknown office/post/duplicate/invalid enums report useful errors without opening another resource.
3. Scene directory and mailbox open/close are copied/restored; mailbox pending/all/older/latest restores via GET only. Conversation facts disclosure round trips. Narrow iframe uses the same route.
4. Delay native supply; deep link stays in URL through empty supply, draft edits and refresh; exact target appears later and opens once. Wrong generation, missing incarnation, missing target and same-name successor never redirect. User navigation cancels a pending target.
5. Keep independent drafts/attachments while switching exact targets, office, presentation and browser history. No send/launch/stop/handle/transcript POST during restoration. Parent getTarget/addContext keeps exact identity and away/back revision fencing.
6. Preserve `supply`, unrelated host query parameters and hash; repeated supply refresh/draft typing does not grow history. Search uses replace; actual navigation uses push.

## Examples

`/space/?office=%2Fprojects%2Fatlas%2Fweb&view=chat&tab=history` opens the stopped directory in chat presentation. Exact conversation URLs additionally require the actual scope/generation/name and native incarnation or stopped_event; copy the address after selecting the actual supplied resource rather than constructing identity from a display name.

## Working record

Design recorded before skeleton/implementation. Source inventory: App, offline/controller/model/projection, OfficeNavigation/OfficeResources, SceneSpace/HumanMailbox, Conversation, live/context/controller. CBM CLI discovery failed after its 30-second daemon-client timeout; current-file inspection is authoritative and no graph completeness is claimed. Verification is finite and controller-owned for browser consumption. Source changes do not update the selected :38023 service.

## Source handoff and checks

Implementation integrates route parsing/serialization and pending exact resolution through App, SpaceController, SceneSpace, Conversation, OfficeResources and the native mailbox read adapter. Canonical UIUX scene consumers supply the same controlled directory/mailbox state. Legacy focus parsing moved out of the context bridge; bridge tests now consume an explicit selected conversation, while route tests own query focus coverage.

Scoped execution: `node --test web/agent-space/tests/routes.test.mjs web/agent-space/tests/context.test.mjs` passed 10/10 (2026-11-16). Cases cover exact current/stopped grammar, legacy stopped inference, preserved host hints, filters/history, draft/attachment preservation, delayed native-shaped supply, one exact read lookup, wrong generation and same-name successors, missing post feedback, mailbox cursor and facts state. Initial scoped typecheck passed before final read-adapter/consumer edits; independent verifier owns final product build (including scoped typecheck) and real App browser checks. No source acceptance or LAN adoption claim is made by these implementer cases.

An outside-member-page exact target uses the existing fenced `LiveSpace.openReference` GET once per pending URL intent. Failed reads retain bounded unavailable feedback and never infer a namesake. A later supply refresh retries matching supplied facts; it does not repeatedly issue lookups. When any previously resolved resource disappears or generation changes, URL intent is retained and the reader becomes unavailable. A deliberate UI navigation cancels it.

## Verified source outcome

Independent final production build and scoped compile passed on the unchanged source at `/tmp/space-routes-gila-final-r1`. Linux headless Google Chrome 147 consumed the complete App through finite R1–R9 UI/direct-open/reload/history/async exact lookup/stale-target/mailbox/iframe/context/draft/attachment/narrow-short scenarios with no product defects. The API responses were native-shaped simulated fixtures; this establishes Web consumer behavior, not native backend or live adoption. See [verification and limits](../../.controller/space-routes/verification.md#completed-independent-outcome) and [source/artifact/browser evidence](../../.artifacts/space-routes-verification/summary.json). Original harness-only attachment-disclosure failure is retained and explained there. Owned browser/server resources were released.
