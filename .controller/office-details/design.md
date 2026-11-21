---
schema: document
status: active
owner: zila
last_updated: 2026-11-15
description: Implementable compact office treatment and affected acceptance criteria.
links: [./delivery.md, ./evidence/baseline.json]
---

# Selected candidate

A person remains an exact Agent instance, with a small name above the head and a single contextual information surface. The office keeps its pixel art, status motion, camera, conversation and drafts. The five requests are one compact scene refinement; no new navigation or permanent help panel is needed. This is an implementation handoff, not completed browser/API acceptance.

## Question, method and observed basis

The user needs to identify people, recognize the adopted controller, and inspect each person's function and profession without a label box and status sentence below every sprite. The deciding question is whether denser seats plus quiet head labels make more useful people visible while preserving selection and readable contextual facts. Use a bounded refinement of the existing actual host, source geometry and real data, then independently operate the affected candidate. No separate runnable prototype is necessary: a second scene implementation would add cost and supply a weaker answer than the existing host and native scene model.

Observed the selected live `/space/` at 1440 × 900 using Linux Google Chrome, native same-origin GET supply and reduced motion. [Raw baseline](evidence/baseline.json), [desktop](evidence/before-desktop.png), [resize capture](evidence/before-mobile.png) and [collector](evidence/design-baseline.cjs) retain source/API association. Scope `/home/long/works/longwork/agentx/.agentx`, generation `a6195a5b-1dd4-4a6d-87d4-1f18f1b664fb`. The observed all-office scene has 21 current members, including 14 in the project office, and five available Roles. No mutation request or page error occurred. The first desktop viewport shows only three resting project people completely; additional rest rows extend below it. Resting centers are about 147 screen px apart; work rows are about 110 screen px apart. Actor buttons measure 132 × 93.25 screen px with 13px text; their lower name/status rows dominate the character's footprint.

The desktop screenshot shows the working and resting zones distinctly, but repeated beige name boxes and status prose compete with small pixel people. Preserve the zones and quiet the labels. The resize checkpoint shows an empty visible canvas after desktop→390px resize; the existing code retains camera center while the world reflows. This is baseline friction, not evidence of a new regression and not authorization to replace the camera model. Acceptance must check fresh narrow entry, explicit panorama recovery, and retained selected context separately; do not claim the resize checkpoint is a usable mobile entry. The live controller Role currently has no description: missing-description handling is required, not hypothetical. A separate fresh 390 × 844 browser run reported one non-hidden actor at direct entry and fifteen after explicit panorama ([structured observation](evidence/mobile-baseline.json), [direct capture](evidence/before-mobile-direct.png), [panorama capture](evidence/before-mobile-panorama.png)); these counts establish DOM availability, not label legibility. The first narrow collector incorrectly waited for the first roster actor to become visible and timed out because that actor was outside the view; the corrected collector waits for attached actors and inspects availability separately.

## Source map and semantic contract

| Source | Existing responsibility | Selected change |
| --- | --- | --- |
| [`scene/model.ts`](../../web/agent-space/src/scene/model.ts), `HALL_OFFICE_LAYOUT` / `officeRooms` | Shared world coordinates for furniture, people and zones | Compact pitches and capped centered work columns below. |
| [`scene/HallScene.tsx`](../../web/agent-space/src/scene/HallScene.tsx), `HallScene` / `HallArt.draw` | Screen-projected accessible buttons and canvas sprite/emote drawing | Actor label/disclosure, crown and no standing status row; use the same moving frame for every anchor. |
| [`scene/art/pixel/characters.ts`](../../web/agent-space/src/scene/art/pixel/characters.ts), `CHARACTER_HEIGHT`, `character` | 16 × 24 pixel bitmap, foot anchor; HallArt uses art scale 2 | Head anchor is feet minus 48 world units; use this rather than fixed screen offset. |
| [`scene/office.css`](../../web/agent-space/src/scene/office.css), `.office-actor*`; [`space.css`](../../web/agent-space/src/space.css), scene width override | Actor label and hit region | Local semantic tokens, unboxed 11px labels, explicit focus treatment and accessible target. Remove stale 132/112px overrides for this actor treatment. |
| [`live/projection.ts`](../../web/agent-space/src/live/projection.ts), `projectAgent` / `projectPosts`; [`offline/model.ts`](../../web/agent-space/src/offline/model.ts) | Native→presentation facts | Keep display identity `@name`; carry optional responsibility separately and adopted Role description from the member's supplied Role. |
| [`OfficeResources.tsx`](../../web/agent-space/src/OfficeResources.tsx), post list / `PostDetail` | Current available Role definitions | Add Role disclosure at list affordance; existing detail remains complete click/touch route. |
| [`scene/SceneSpace.tsx`](../../web/agent-space/src/scene/SceneSpace.tsx) / [`scene/geometry.ts`](../../web/agent-space/src/scene/geometry.ts) | World camera, exact selection, return camera and reader | Preserve ownership; disclosure alone must not pan/select/reset. |
| [`src/management/offices.rs`](../../src/management/offices.rs), `project_collaboration` / `role_view`; [`contract.rs`](../../src/management/contract.rs) | Adopted Role facts and native API | Existing `RoleView.description` already available; preserve adopted vs current-catalog meaning. New title declaration belongs to native contract and generated TS. |

`AgentResource.title` currently means the visible `@name`; do not overwrite it with launch responsibility. A separate nullable presentation field (e.g. `launchTitle`) carries native `AgentView.title`. Role description on a person comes from `native.collaboration.role.description`, not a lookup by name in the current Role catalog. Available post disclosure uses that post's own description and source. Never substitute status/detail for a missing responsibility. Crown condition is exactly adopted Role with `name === 'controller'`; role `default`, agent name, launch title, current catalog default and substring matches do not decide it.

Lumo owns the title native contract and has selected the existing durable `instances.launch_context.title`: trimmed single-line nonblank text, max 160 Unicode characters; omission null, old records remain null, immutable recapture preservation, stopped snapshot and resume/fork inheritance. CLI `--title` and management launch/API use the same validation. Unsupported remote/shared app-server reservations must reject explicitly rather than silently drop the field. This design consumes those actual facts; independent acceptance must retain the exercised host's limits.

## Geometry and visual treatment

All distances in this table are world units unless explicitly marked screen CSS px. Put literal values in the existing owning local geometry/tokens, not inline scattered CSS.

| Dimension | Existing | Candidate |
| --- | --- | --- |
| Work column pitch | 144 minimum, then stretched `work.width / columns` | 120; choose columns with existing available-width rule, cap actual pitch at 120 and center occupied columns in work zone. |
| Work row pitch | 144 | 120. |
| Rest row/column pitch | 192 | 112. |
| Side lounge width | 440 | 320, retaining existing breakpoint and side/below rules. |
| Lounge heading/fixture clearance | 112 | 88; first rest row must remain below fixed lounge fixtures. |
| Room inset / heading / room gap | 32 / 64 / 32 | Preserve; these protect walls, headings and props. |
| Name | Boxed 13 screen px below sprite | 11 screen px, 14px line height, medium weight, max 80px width, ellipsis, dark office ink with subtle light text shadow; no fill, border or pill. |
| Name placement | Foot label with fixed screen padding | Name row bottom = moving foot screen y − `48 × camera.zoom` − 4 screen px. Center at sprite x. Name text never scales with zoom. |
| Crown | Absent | Code-native 10 × 8 screen px gold crown, dark outline, 3px gap immediately before name; inline row total max 96px. Keep both aligned above head. No emoji/font-dependent crown. |
| Person hit region | 132 × ~93 screen px | At least 44 × 44 screen px centered over the sprite; name also clickable, total target must not cover adjacent person's center. Transparent at rest. |
| Active/hover/focus | Boxed label highlight | Slight name weight/accent and visible focus outline on transparent actor hit region; selected state also `aria-pressed`, no permanent name box. |

At default .75 zoom, rest centers become 84 screen px instead of 144; work centers become 90 screen px instead of at least 108. Names max 80px fit with a small gap, and 44px targets remain distinct. At zooms where all names cannot fit, keep the hovered/focused/selected name fully visible above peers and allow the panorama to be an overview; do not shrink labels below 11px to claim readability. Full identity is always available in the local disclosure and roster. A small crown plus a long name uses the capped row and truncates the name, never the crown.

The existing emote is at feet−52 world units, close to the new label. Move attention/error/unknown emotes to the sprite's upper-right: sprite center +22 world units, feet−42 world units. Preserve emote semantics and motion; test the crown/label/emote combination. Keep controller crown inline with the name so it does not collide with the sprite or upper-right emote. Status state remains in animation, accessible name, local disclosure and member reader; remove only the actor's standing `<small>` status/unread prose.

## Contextual disclosure and input

Use one noninteractive person information tooltip anchored to its current actor target, shown on pointer hover (~180ms) and keyboard focus, with `aria-describedby`; Escape closes it and it must remain visible while pointer is over tooltip content. Max width 280 screen px, viewport side margin 8px, padding 10px, gap 4px, 12px text. Place above the name with an 8px gap, flip below when needed, clamp horizontally and vertically. Render in a non-clipped overlay owned by the scene, above labels but below the open reader/menu. Track the actor's actual moving frame or close on pan/zoom/actor movement if reliable tracking is unavailable. No stale bubble may remain on a different person after filtering, office switch or refreshed identity.

Person content order: full `@name`; `职责 · <launch title>` when supplied; `岗位 · <adopted role name>`; Role description when supplied; concise current status. Missing title omits the responsibilities row. Adopted Role without description says `岗位说明未提供`; no Role says `未采用 Role`; unknown adoption says `岗位信息未知`. These are local data cues, never standing page prose. Controller's accessible description includes `controller · 控制岗位`; crown itself is decorative to avoid redundant announcement.

A normal person click/Enter/tap keeps the existing selection→conversation path. On touch, supply the same responsibility/Role facts in the selected reader's existing local detail disclosure, with an obvious compact information control; one tap opens reader, its information control reveals facts, close returns without losing selection or draft. Do not make the first tap merely show a tooltip and require a second tap to select. Focus disclosure does not open the reader; Enter does. Keep tooltip text noninteractive; long facts wrap, bounded height can scroll if needed, and complete facts remain in the reader.

For current available Roles, hovering/focusing the existing post-list button reveals Role name, description or `岗位说明未提供`, and origin (`本地`/`继承`/`全局`). Click/tap still selects the existing `PostDetail`; no new route/modal or permanent repeated description is needed. A person's adopted Role description and a post's current definition can differ legitimately. Never show current catalog text as the person's adopted historical description.

## Independent affected acceptance after implementation

| Case | Required observation |
| --- | --- |
| Supplied and omitted title | Disposable real CLI launches, actual persisted native record and API read agree; omitted path and old data read remain compatible. Retain exact binary/scope/host and title validation limits. |
| Durability and lifecycle | Re-read after owned daemon restart or equivalent actual persistence boundary; verify selected stopped/resume behavior with original and new exact references. No mutations on live user Agents. |
| Person facts | Browser hover/focus shows exact title and adopted Role description; absent title/description and none/unknown Role are graceful. Escape dismisses; click/tap still selects reader, reader detail supplies facts without hover. |
| Crown | Adopted controller has one small crown; coder titled “controller”, similarly named person and default non-controller Role have none. |
| Compact scene | Desktop live-like 14+ mixed running/resting members plus representative long label/title and controller+attention fixture: no boxes/status prose beneath people, label above moving head, distinct hit regions and tighter furniture/rest centers. Compare at the same viewport, camera and supply. |
| Continuity | Hover/focus leave camera and exact selection unchanged; select at existing zoom, draft, close, switch scene/chat/office and return retain existing guarantees. Disclosure must not send messages or request Provider transcript. |
| Narrow viewport | Fresh 390 × 844 entry; focus/touch facts; selected reader and input reachable; all overlay edges within viewport. Resize a selected reader and confirm context; note existing unselected-center reflow limitation separately. |
| Selected LAN adoption | After owner rollout, read-only GET/render smoke at selected 38023 address with candidate asset identity and actual API shape. No send/stop/recovery/collection operations. |

Representative browser fixture content can be supplied at the native response boundary in an isolated browser context against the exact candidate assets; mark such layout/error fixtures simulated and use separate disposable native launches for durability. This is bounded expert walkthrough evidence, not user research. Independent executor zila will write `acceptance.md` and immutable attempt artifacts only once lumo supplies a reviewable candidate and run instructions. A build or static source review does not close these cases.

## Recorded uncertainty and boundary

The actual controller Role has a null description, so the implementation must not invent a profession sentence. Long launch titles are bounded natively but still wrap in the local surface. Existing panorama can fit the whole world below normal near-view zoom; dense overview labels may overlap unless prioritized. Candidate observation decides whether the proposed cap needs a small local adjustment. Any adjustment should stay in this geometry/disclosure owner and be recorded here, without changing camera, membership or identity semantics.

# Added scope: real message motion, desk mail and state symbols

## Existing support and factual limits

Source inspection for the added request found no executable inter-Agent message animation in `HallScene`: it draws office art, workstations, character frames and a subset of emotes. [`props.ts`](../../web/agent-space/src/scene/art/pixel/props.ts) already supplies `prop('envelope')`; existing `.office-envelope` CSS is not an executed message path. Reuse that code-native art and the scene's one animation clock rather than importing a former runtime or building an animation engine. [`live/controller.ts`](../../web/agent-space/src/live/controller.ts), `readMessages`, currently reads only the selected Agent's messages, which cannot supply office-wide activity. Its selection-local cursor is not an Agent read marker.

Native [`MessageView` / `MessagePage`](../../src/management/contract.rs) and [`store.rs`, `messages`](../../src/management/store.rs) expose event identity, sender, recipients and time. They do not establish model/human read acknowledgement or exact delivery completion. [`db/events.rs`, `try_get_unread_messages`](../../src/db/events.rs) uses matching events beyond `instances.last_event_id`. [`messaging/inbox.rs`, `DeliveryEvidence`](../../src/messaging/inbox.rs) and [`inbox_store.rs`, `confirm`](../../src/messaging/inbox_store.rs) distinguish provider/hook/tool delivery boundaries from admission and business completion. Therefore the user-desired accumulated mail can truthfully represent **待接收消息** (pending native delivery), while “human/model unread” remains unsupported. No browser click may advance the native delivery cursor or pretend to clear pending mail.

Lumo's proposed native read-only bounded office activity supply with watermarks/pagination, current exact Agent references and authoritative pending counts is the selected integration direction. Native Rust declarations and generated TS remain authoritative; no prototype-only API DTO. Lumo must complete routing/identity inspection before implementing the contract. New supply should disclose partial coverage/error separately from confirmed zero and must not require message-body collection just to draw envelopes. A count must declare whether complete or a lower bound if bounded scanning is used. Current entity identity alone cannot retrospectively prove a historical sender/recipient incarnation; ambiguous events must not animate toward a replacement.

## Visual and interaction decisions

| Element | Meaning | Treatment |
| --- | --- | --- |
| Flying envelope | A newly observed accepted/addressed native message, not completion or read | Existing pixel envelope, one shallow arc from sender workstation to recipient workstation, 700ms ease-out; no text travelling across the scene. |
| Desk letters | Native messages still pending delivery to this exact participant | One to three fanned envelopes, with small count when count >1; exact or lower-bound value in local disclosure. |
| State symbol | Current supplied native Agent status | One small upper-right glyph, separate from controller crown and desk mail. |
| Person disclosure | Identity, function, profession, state and inbox facts together | Append `待接收消息 · N` or `至少 N` / `待接收信息未提供`; explain cursor boundary only inside this local information surface. |

Desk anchor is `actor.home.x + 12`, `actor.home.y - 20` world units, on the workstation surface even when its owner is resting. Mail is owned by the workstation/exact Agent, not carried by the resting character. Fan offsets are 3 world units right and 2 upward, capped at three letters. The envelope art remains small (reuse its bitmap at art scale 1; max 18 × 12 world units footprint) and count is 10 screen px in a compact local marker; no card, prose, recipient name or bouncing stack. Place count just right of the stack and clamp within that desk's region. Keep normal desk monitor and seated person recognizable. If the actual art footprint differs, fit within this reserved region rather than expanding cell geometry.

Envelope start/end anchors use the same workstation coordinate model and current camera as desk stacks. Arc apex is 32 world units above the straight path; cap its screen rise at 40px. The canvas flight is decorative and pointer-inert, so it cannot intercept pan, selection or reader controls. Start represents acceptance; arrival must not change the count, status, native cursor or receipt. Native delivery may already be confirmed by the next snapshot: the accepted-event flight can still finish while zero desk mail remains. Conversely a failed delivery can leave a stack after the flight ends.

Use the existing actor button for full desk/person access rather than making tiny pixel letters an isolated touch target. If desk mail itself is clickable, its transparent target must be at least 44 × 44 screen px and must not overlap a neighboring actor or steal the person's click; route it to the same exact person's existing messages reader, without marking native mail read. Selected reader detail and roster must supply mail facts when a workstation is offscreen. No message is sent or Provider transcript requested by opening mail.

## Event lifecycle, clutter and accessibility

1. On first activity read, establish a native watermark and current pending counts. Existing pending mail appears immediately as a static desk stack. Initial history, remount, reconnect and restored saved state never replay flights.
2. Fetch subsequent bounded pages after the watermark. Deduplicate by scope + generation + native event ID + exact recipient reference. Advance only through actually processed page boundaries. Do not compare native string event IDs lexicographically or invent a browser clock cursor.
3. Start a flight only when both exact current endpoints are unambiguous, belong to the drawn office scene, are within the viewport, and remain the same instances. Missing/system sender, stopped/replaced identity, cross-office/offscreen endpoint or self-message produces no fabricated route. Pending counts remain factual regardless of flight eligibility. This limitation belongs in the contextual activity coverage cue, not a permanent banner.
4. Allow at most six simultaneous flights and a transient queue of 24; coalesce same sender→recipient bursts into one envelope with a disclosed event count. Overflow and events older than ten seconds at observation update facts without replaying an animation backlog. Native event time parsing is the source for age; unknown age skips flight rather than inventing freshness. No animation promises that polling observed the original send instant.
5. Keep scene motion pause/hidden-tab/reduced-motion behavior coherent: pending snapshots still update, but flights do not run or queue for replay. Reduced motion uses immediate static mail changes only, no fade/slide substitute. On return resume fresh observation from a new baseline if a gap would produce stale flights.
6. On pan/zoom/layout change, render flights through the same updated camera; cancel when either endpoint leaves view. On office/scope/generation change, exact-instance replacement or unmount, clear owned flights/queue/dedupe window and stale mail disclosure. Preserve conversation selection/draft/camera ownership.

One aggregate polite accessible status can announce `新增 N 条办公室消息` once per poll/burst, never announce every animation frame, never announce message bodies unsolicited, and suppress historical baseline. All icon meaning, exact pending count and raw status remain reachable on keyboard focus and reader information touch route. An API error or unavailable count is unknown, never zero; a previously seen count may be retained only with an explicit stale cue inside disclosure and must not be presented as current confirmed pending mail.

## State symbol mapping

Retain raw native `status` in the presentation alongside the existing scene pose bucket. Current projection collapses `blocked` into `failed` and `inactive` into `idle`; symbol/disclosure must not lose that native distinction or infer a model phase from a pose. Use existing [`emotes.ts`](../../web/agent-space/src/scene/art/pixel/emotes.ts) bitmap family at upper-right (`x+22`, `feet−42` world units), with one chosen glyph only. Its color supports the distinct glyph, not vice versa. Crown remains inline before the name and independently present for adopted controller.

| Native supplied state | Glyph / existing art | Local meaning |
| --- | --- | --- |
| active / working | `…` / `thinking` | 工作中; dots describe activity, not proof of model reasoning. |
| listening / idle | small cup / `readyCup` | 等待消息; not asleep, stopped or unread. |
| ready | `✓` / `check` | 就绪. |
| blocked | `!` / `alert` | 受阻; use supplied detail when available, never invent a question or required answer. |
| failed | `×` / `fail` | 执行失败. |
| inactive | quiet hollow circle in existing palette | 未活跃; do not relabel it as confirmed waiting. |
| unknown / unsupported value | `?` / `question` | 状态待核对; keep unsupported raw value in disclosure. |
| stopped historical record | no live person glyph | Existing cabinet/history, not an active seat. |

A generic `attention` boolean must not override native failed/unknown glyph into an ambiguous alert. Mail count has its own desk marker; do not add a second head bubble to mean pending mail. The currently supplied failed/blocked attention flag is not unread evidence. If an explicit independent attention fact is introduced later it can be included in disclosure without displacing the primary state.

## Added independent acceptance cases

| Case | Evidence needed |
| --- | --- |
| Existing support question | Report current baseline as no executable message flight; art/CSS availability is not behavior. |
| Real new direct message | Disposable native sender/recipient, actual accepted event plus observed envelope starting/ending at their desk anchors; no API-request initiation from animation. |
| Initial/repeated/reconnected events | Pre-existing events cause no flight; duplicate pages/polls cause one flight maximum; generation or same-name replacement cannot inherit old flight. |
| Pending pile and clearance | Seed multiple real pending messages in disposable scope, observe actual count and max-three visual pile; authoritative delivery-cursor advance changes pile. Opening browser messages alone leaves native count intact. |
| Non-delivery/read semantics | Flight completion does not clear pile or become read confirmation. No unread claim appears absent a real read contract. |
| Partial/error and endpoints | Supply missing/stale/lower-bound data, ambiguous/system/self/offscreen endpoint and cross-office message: truthful count/coverage, no invented route or zero. |
| Busy scene and interruption | Burst beyond flight cap, pan/zoom, office switch, selected reader, long title/controller/failure combination: bounded clutter, correct glyph/crown, no interception or lost draft. |
| Reduced motion/pause | Actual browser reduced-motion preference and motion pause suppress flight/replay while factual pile/status updates continue. |

Native storage/API and actual delivery evidence are required for the pending/durability claims. Browser interception fixtures may exercise bounded crowding, missing supply and symbol combinations only; label them simulated. Final LAN smoke remains read-only. This added scope is designed but not independently executed until the combined candidate is supplied.

# Coherent office behavior model

The expanded user intent is a friendly office whose scene-relevant behavior corresponds to real lifecycle, work/rest, messaging and attention facts. It does not require animation for each CLI command, read, refresh or local filter. All treatments below consume the same exact-instance scene model and `HallScene` clock; animation completion never admits a native transition, changes an action receipt or delays the visible factual state. The scene is a metaphor for observed facts, not independent execution evidence.

## Existing engine and selected transitions

[`motion.ts`](../../web/agent-space/src/scene/motion.ts) already owns `beginHallMotion` / `sampleHallMotion`; `HallScene` retains frame/motion per exact actor key, retargets from its currently displayed frame when `hallActorGoal` changes, and initializes newly projected or relocated seats at their goal. Therefore rest→work and work→rest already have routed walking when a supplied state actually changes. First appearance currently has no separate entrance path, and failure/unknown/attention only change pose/emote. Reuse these structures; do not build a parallel timeline, general behavior DSL or second animation scheduler.

| Behavior / authoritative trigger | Selected visual effect | What must not be inferred |
| --- | --- | --- |
| First successful snapshot, history page, loading another office, later membership page | Place existing people at their supplied current work/rest goals, display their facts immediately | First browser appearance is not Agent creation. |
| Real newly created exact instance after observation baseline | Walk room entry door→native work/rest goal using existing aisle routing; name/crown/glyph follow moving frame | A new exact key alone is not proof of creation time; resume/reincarnation needs actual new native lifecycle evidence. |
| idle/listening/ready/inactive→active/working | Existing rest→desk route, work pose at destination; immediately expose new raw status in glyph/disclosure | Walking time does not mean Agent has not begun work; no fake native busy interval. |
| active/working→idle/listening/ready/inactive | Existing desk→rest route, destination pose uses supported state family | Rest animation does not imply sleep, shutdown or quota waiting. |
| Work/rest bucket unchanged but raw state changes | Update one primary glyph and local facts; one subtle 240ms scale settle allowed for the changed symbol | Do not manufacture travel when location responsibility is unchanged. |
| Native failed/blocked/unknown | Correct × / ! / ? appears immediately; maintain existing world goal policy and route if it changes | Do not show a question needing a reply or recovery success absent those facts. |
| New accepted message | Selected envelope flight and factual pending pile behavior above | Flight arrival is neither native delivery nor read. |
| Confirmed delivery-cursor advancement | Update/shrink desk pile immediately; optional one 180ms opacity settle of changed pile | Browser opening reader must not clear mail or create delivery evidence. |
| Native stop/removal | Remove active interaction/capabilities immediately; actual stopped record remains in cabinet/history. If a true stop event and cabinet are supplied/visible, one 300ms quiet cabinet highlight | Absence due to paging, refresh error/filter or unknown membership is not a stop; no theatrical death, forced walk-out or invented lifecycle event. |
| Replacement/resume/fork | Old exact actor motion is canceled; a new exact reference may enter only with corresponding observed native creation/new-instance fact | Same display name does not inherit flight, draft, entrance animation or old pending mail identity. |
| User selection, reader close, pan/zoom and view switch | Preserve current camera/reader behavior and direct visual selection; actor freeze/release remains existing controller policy | Local navigation is not a native Agent event; no automatic send or lifecycle operation. |

The creation door is `room.office.door`, shared with existing office geometry. Seed a creation's existing `HallMotion` at that door and route to the current goal. If door or target is absent/offscreen, show the current actor at its target without a contrived route. Avoid forcing a viewport pan to show an arrival. Work/rest routes use the existing aisle path; do not shorten them by cutting through other desks merely to look faster. Cap decorative creation travel at 1.6 seconds by scaling its existing path timing; ordinary existing walk timing may remain unless the actual compact geometry makes it excessive. Distances/appearance remain pixel-scene conventions, never a claimed native physical location.

## Observation and cancellation rules

The live controller owns a successful complete baseline and native observation watermark. Pagination and discovery completeness must be established before treating set-difference as a possible new actor. Creation eligibility requires an exact newly created native instance fact after that baseline (or trusted supplied creation timestamp after observation start with same-snapshot identity fencing), not `createdAt: ''` from current presentation, browser wall-time guesses or absence from a prior partial page. Native owner lumo determines the precise available source and exposes only what it proves. A delayed, failed or stale page is never creation evidence. If creation facts are unavailable, explicitly report entrance unsupported rather than animate old members on refresh.

Transitions are keyed by scope, generation, exact current instance and current projected seats. Rapid native changes retarget from the currently sampled frame to the newest true goal; they do not enqueue old behavior for playback. If a person starts work then rests before arrival, turn toward rest from that current point. One actor has one movement at a time. A new layout coordinate frame (resize/reflow, different office projection) snaps to the current goal as the existing seat-relocation logic does; do not mistake it for work/rest motion. A native failure while walking updates its symbol immediately, then follows its newest projected goal. Stopping, disappearance, incarnation change or scope reset cancels retained motion and flights involving that old exact actor.

First load, reload, reconnect after an observation gap and returning from a hidden/paused scene establish static baseline without replay. Under reduced motion, native snapshot changes appear at the final goal with static glyph/mail updates. Under the user's pause control, ongoing decorative travel settles to current true goal and fresh changes update there; on resume no queued creation, walk, flight or symbol pulse plays. Keep factual readings current while decoration is paused. The existing one-clock cleanup/visibility listener remains the owner; clear pending animation collections on unmount and scope changes.

For a selected Agent with an open reader, retain the existing selected-frame/camera policy. New native status and mail facts still update immediately, but decorative entry/travel must not move the reader anchor away from its current selection or reset a draft. On closing, any movement resolves toward the latest true goal rather than replaying all transitions suppressed while reading. Labels/crown and hit targets follow the displayed sampled frame throughout travel; workstation mail remains anchored to the workstation.

## Coherent affected acceptance additions

Exercise one isolated continuous scenario with exact refs and native facts: establish existing resting members → create a fresh exact member after baseline → observe door entry → switch that member from resting to working and back → interrupt a walk with another true state → accept a real sender→recipient message and native delivery → stop the exact member. Associate API/native observations with rendered paths, glyphs and stacks. The status should be immediately factual while motion is still in progress; verify no browser native mutation occurs merely from animation or selection.

Also verify no entrances on first snapshot, pagination, filter, office switch, resize or refresh; no transition inheritance on same-name replacement; cancellation of stopped/offscreen actors; reduced motion and pause behavior; selection/reader/draft/camera continuity. Static status-only browser response fixtures may supplement hard-to-reach timing/symbol cases, clearly labeled simulated, but actual creation/message/pending facts require disposable native evidence. Current user services and Agents remain read-only acceptance targets. Entrance eligibility/native contract details and this combined candidate's runtime evidence remain pending implementation.

# Fixed human endpoint: @bigboss cat desk

## Verified meaning and product decision

The controller accepted the bounded human endpoint in message #25992: fixed cat desk and real communication representation, with no new human read-ack subsystem, no unread/clear claim and no artificial delivery cursor. This section supersedes any earlier suggestion to clear a page-local cat counter. Cat inbox is a read-only list of real addressed messages; viewing it changes no native delivery/read state.

Source findings: [`management/store.rs`, `Request::Send`](../../src/management/store.rs) explicitly sets `external_sender: Some("bigboss")`; [`worker.rs`, `directory_send`](../../src/management/worker.rs) reaches the same fenced send after resolving an exact Agent. [`messaging/service_store.rs`, `sender`](../../src/messaging/service_store.rs) separately checks claimed caller authority and rejects external impersonation by subagents; the presentation name is not a credential. [`shared/constants.rs`](../../src/shared/constants.rs) defines `SENDER = "bigboss"`, while [`routing.rs`, `target_instances_with_sender` / `match_target`](../../src/messaging/routing.rs) recognizes the virtual device-agnostic target. That recognition is not a normal Agent instance, workspace membership or evidence of a pending/read cursor.

A read-only GET of current live descriptor/first 100 Agent entries returned the established generation, no `bigboss`, and a stopped-history continuation. [`store.rs`, `list`](../../src/management/store.rs) emits current local instance rows before stopped history, so this scope supplied no current local bigboss row in that observation. This does not prove a virtual human inbox is empty. Lumo's current working activity projection only represents exact Agent endpoints; its null external sender and missing virtual delivery target cannot be reinterpreted by the browser as the cat. These are working-source findings, not evidence the running binary has the new activity support.

Native owner must supply explicit human endpoint facts: outgoing event is accepted under external `bigboss` authority; incoming event has actual native resolved human addressing. A string `@bigboss` in message text, a reply-looking message, a matching name or a synthetic AgentRef is insufficient. `delivered_to` currently derives actual deliverable instances, while virtual addressing can be represented separately in effective mentions/routing; lumo must preserve that distinction in the canonical event contract. Legacy events missing required authority/addressing facts remain unanimated. Never fabricate an Agent incarnation for the human endpoint.

## Single spatial anchor and filtering

Choose one **scope-level foyer desk outside all office rooms** rather than putting a cat in every office or calling it a global Agent member. The same scope-human endpoint is rendered exactly once in the current scene, both all-offices view and individual office view. Office selection filters Agent rooms, not this human endpoint. Thus changing office reprojects the same endpoint; it neither moves the human's membership nor creates another inbox. Local header/disclosure identifies it as `人类入口 · 当前 scope`, with full scope path disclosed, so the cat is not mistaken for the selected directory's controller.

Reserve a compact 112-world-unit foyer strip before room origins; add that offset once to the room projection, cabinet/props included. At normal .75 zoom this costs 84 screen px, traded for an explicit unique communication endpoint without crowding an existing desk. Retain all room/internal compact geometry. Cat desk center is x=80 world units from scene left; cat foot anchor y=84 within the strip. Reuse existing desk/chair art and a fixed seated `character({ role: 'meow', ... })` with stable scope-derived appearance; no bitmap generation or new sprite library. Its name uses the same 11px head-label rule, without crown or state glyph. Maintain minimum 44 × 44 screen-px accessible target. Keep the right side of the strip unfilled; it is a transition gutter, not a new header/card/banner. At minimum scene width ensure desk/name stay inside the same world bounds. Room y offset and backdrop must share the one projection; do not apply CSS-only offsets that detach camera/hit targets.

The cat remains seated at this workstation, including when messages arrive, pause changes, offices switch or Agents stop. There is no door entrance, work/rest walking, failure/unknown/ready animation or native lifecycle control for the cat. “Always at desk” means fixed pose and world location, not screen-pinned: user pan/zoom can move it offscreen. Do not force camera recentering, add a floating duplicate cat, or shrink all Agents to keep it continuously visible. The read-only human inbox control remains reachable from a compact local `@bigboss` entry in the existing scene/reader navigation if its desk is offscreen.

## Messages, disclosure and action boundary

Outgoing accepted human events originate at the cat desk and end at the source-proven exact Agent workstation. Incoming source-proven human-directed events start at their exact Agent workstation and end at the cat desk. Reuse the same bounded flight rules, watermark, per-event-recipient dedupe, pause/reduced-motion handling and missing/offscreen-endpoint suppression. Scope endpoint identity is explicit (scope + generation + human kind/name), separate from exact AgentRef. A flight still means accepted and addressed, never delivery or human read. Only endpoints in this selected scene can support a visible route; an incoming message from another office remains in the scope-human list without a fabricated flight through the selected office.

Desk mail can show one to three static fanned letters **only as representation of real messages in the loaded addressed-human list**, with no unread badge or decrement animation. No pile is shown for unavailable supply, and confirmed empty list is distinct from unavailability. More than three messages remains a bounded pile, with list coverage/count available only as `已加载 N 条` or the native bounded-history label; do not label it pending, unread, new or needing acknowledgement. Opening the list does not clear the pile. This differs intentionally from real Agent pending-delivery stacks, whose native cursor semantics do exist.

Tooltip/focus content: `@bigboss`; `人类通信入口`; `当前 scope · <path>`; `收件确认未提供`. Click/Enter/tap opens one compact read-only local inbox, retaining scene camera, office selection and any exact-Agent draft. It lists actual sender, timestamp and body for native-addressed human messages, with thread/reply fields where supplied and bounded paging/error/empty states. Header `发给 @bigboss`; small local `收件确认未提供` cue belongs near list facts. No mark-read, clear-unread, send-to-self, stop/resume, Role, model or Provider transcript controls. Cat selection must not reuse fake AgentResource state to unlock those actions. Close restores prior selected reader/draft and focus to the cat/invoking navigation control. If existing reader architecture cannot preserve that context, use a small scene-owned read-only drawer over the stage, not a new full page.

The existing deliberate human→Agent compose paths retain their exact recipient and native authority/receipt handling. Clicking cat never sends, and the scene never sends a greeting or acknowledgment. Full messaging permission decisions stay with native routing; a visual endpoint grants no additional authority.

## Added acceptance boundaries

- Verify real disposable browser sends carry external bigboss authority and source-proven addressed Agent identity; observe one cat→desk flight, no flight for unconfirmed/failed action.
- Verify real disposable Agent→virtual bigboss resolved addressing is included in native human list and supports desk→cat flight. Include text-only mentions, system/external sender and legacy events without supported facts as negative/limited cases.
- First snapshot/history load/reconnect creates no fresh flights; incoming list never reports unread/pending or clears on open. Missing supply and bounded/empty lists remain distinct.
- All-offices→office A→office B contains one scope-human endpoint per scene and the same inbox, no Agent count/Role/crown duplication and no inferred directory membership.
- At desktop and 390px direct entry, check the new foyer cost against useful visible people, compact geometry and reachability. Selected Agent draft/camera survives cat disclosure and return; cat target remains usable, arrival/status animation does not affect it.
- Static seated cat and factual messages remain under reduced motion; no automatically generated message or native cursor mutation. Final selected LAN check stays read-only.

Cat geometry is an implementable candidate. Native addressed-human facts and actual resulting compactness remain implementation/independent-acceptance questions; do not infer them from this document.

## Native supply follow-up confirmed by owner

Lumo confirmed routing and selected supply in message #26036. `routing::compute_scope` resolves targets against `target_instances_with_sender`, including canonical virtual bigboss; submission retains effective scope and resolved mentions. A virtual human recipient need not appear in `delivered_to`, which remains actual Agent delivery targeting. Freeze explicit human-sender/human-recipient facts into accepted scene addressing from external sender authority and canonical effective mentions, alongside exact Agent references frozen within the send transaction. Do not replace this union/metadata with null-as-human inference.

The bounded human feed selects accepted `scope=mentions` records with canonical resolved bigboss in mentions; it has no acknowledgement, pending or unread count. This establishes the eligibility rule for real incoming cat messages; raw-text mentions alone remain ineligible. Native creation events now freeze `ParticipantRef` in their created-event facts, enabling post-watermark entrance eligibility without browser clocks or name inference. These are implementation-owner contract decisions; actual native/API/browser execution remains pending and must be recorded independently in acceptance.

# Population-driven room capacity

This follow-up supersedes earlier viewport-stretched room sizing and status-dependent lounge height. Keep the selected compact 120 work pitch, 112 rest pitch, label/crown treatment and scope-human foyer; derive each room's own capacity and dimensions from actual current membership. The user-observed oversized GABI room is a direct consequence of `officeRooms` assigning equal viewport-derived widths regardless of population.

## Inputs and invariants

Capacity counts exact current members in each canonical office before search/state filters. Stopped history, available Roles, cat/human endpoint and message counts contribute zero capacity. Unknown/legacy/global remain their own supplied membership buckets; do not assign them to directory offices by cwd. Work/rest status changes do not change membership, capacity, room dimensions, seat order or office packing. Both zones reserve the entire room capacity, so everyone may switch between working and resting without room resize or moving another person's rest slot.

Native office count/coverage and loaded member count are different facts. The current client has `more` paging and discovery errors; propagate member completeness or an equivalent truthful presentation fact rather than parse a coverage string. A complete loaded current list can establish its membership count. If an authoritative complete native total is supplied independently, use that for minimum capacity while reserving slots for all loaded exact members; otherwise loaded membership is only a lower bound. During partial paging, errors or stale supply: grow if newly loaded known members require it, never shrink, never call loaded N a complete population and never display an empty room as confirmed empty. The whole-scope history tail does not by itself imply more current members: native owner can expose current-members completeness separately if it is established; do not infer that from arbitrary browser limits.

This is page-owned geometry, not persistent office allocation or a new native capacity domain. Keep a small per-office capacity/seat map in existing scene memory keyed by scope+generation+office. A fresh complete baseline starts at its smallest fitting tier. Initial known people take deterministic exact-reference order. Later exact members keep slot indices; removed members free slots, and new members use lowest free slots. Poll reorder/search/status cannot reorder slots. No storage migration, editable floor plan, localStorage layout registry or new service is justified.

## Capacity and growth/shrink

| Complete or loaded lower-bound members | Smallest capacity | Preferred columns per zone |
| --- | --- | --- |
| 0 confirmed current members | 0 | None; compact cabinet/empty-room treatment. |
| 1–2 | 2 | 1. |
| 3–4 | 4 | 2. |
| 5–6 | 6 | 3. |
| 7–8 | 8 | 4. |
| 9–12 | 12 | 4. |
| 13–16 | 16 | 4. |
| 17+ | Next multiple of 4 | 4. |

The spare capacity is zero to three seats, at most one slot for the smallest room. Do not expand just because a room reaches its capacity; expand only when actual supplied membership exceeds it. Thus GABI alone gets capacity two; the second member fits; a third grows to four. At 16 people, keep capacity16; the seventeenth grows to20. Empty seats need no placeholder people or `空闲` label.

Shrink uses simple page-local hysteresis: only after two consecutive successful **complete** current-members snapshots with population at or below half the retained capacity, reduce to the smallest fitting tier. A partial/failed/stale read resets the shrink streak. A changed population restarts it; ordinary status changes neither trigger nor interrupt it. Two complete observations are enough; no background timer or native lease. Shrink then compacts slot indices in their existing relative order and updates both zones atomically. An ordinary removal that still leaves more than half capacity keeps the room stable. New scope/generation resets the memory; an explicit fresh load can choose the minimal initial tier. If no complete membership can be established, report shrink unverified/unavailable for that supply instead of guessing.

## World geometry and packing

Replace `roomWidth = viewport/equal tracks` with this deterministic per-room construction. Distances are world units. Keep inset32, title/header64, work pitch120, rest pitch112 and room gap32. Capacity is C, columns k, rows r=`ceil(C/k)`; derive all home/rest slots from the same stable member index. Center each full reserved grid in its zone, not the currently occupied or resting subset, so adding within spare capacity and work/rest switches keep existing anchors stable.

Side-by-side preferred width = `64 + 120*k + 16 + 112*k`. Work zone width=`120*k`; lounge width=`112*k`. Both share the same top at origin.y+64. Work zone height=`120*r`; lounge height=`56 + 112*r`; room height=`64 + max(workHeight,loungeHeight) + 48`. The lounge header56 replaces the earlier fixed88 for these capacity-driven rooms, because furniture is selected to fit rather than imposing three large props on every population. A small room has coffee only, with no crowded sofa/low-table trio. Add existing sofa/table only when the actual lounge columns/clearances fit; omit decorative furniture before enlarging a population tier. Cabinet foot48 replaces80 for this treatment; locate cabinet at bottom-right with clearance from last person/desk and keep entry at bottom-left, using shared geometry for art and click targets.

Choose side-by-side if that preferred width fits the available scene world width. Otherwise stack work above lounge with gap16; keep columns as large as fit `64 + 120*k`, never beyond preferred tier columns or below one. Stacked width=`64 + 120*k`; lounge gets the same inner width, but uses the same k reserved columns at rest pitch112. Stacked height=`64 + workHeight + 16 + loungeHeight + 48`. Do not stretch a small room to fill spare viewport width. At .75 near zoom, 1–2 members fit a side room312×392world =234×294screen; capacity4 fits544×392world; capacity16 with four columns fits1008×616world. The one-member room loses roughly three-quarters of the old GABI room area seen in the baseline while retaining both usable zones and one spare slot. These are design-derived dimensions, not yet executed measurements.

Pack variable-size rooms in deterministic canonical-office order using a simple shelf: place next room right of previous with gap32 if it fits; otherwise start next shelf below the tallest previous-shelf room with gap32. Do not shortest-column/rebalance on each poll or distribute a single room across the full width. At the baseline approximately1216px scene width, a 16-seat project room and two-seat global room can share a shelf with these world widths, rather than giving both equal oversized halves. A fresh view or viewport resize may reflow shelves. The scope-human foyer offset112 is added once before all rooms; it neither consumes an office seat nor changes with membership. Unknown totals and history counts must not expand the foyer or room.

Confirmed empty office uses width192 and height160world, with one compact existing bookshelf/cabinet treatment and no work/rest grid. A large stopped history does not enlarge it; `停止历史 N` remains a disclosed cabinet reader, not N physical desks. For source errors keep the known office visually unavailable with existing local error/coverage disclosure, not a fictional empty population.

## Camera, movement and continuity

A capacity/tier/reflow change is a coordinate-frame change, not an Agent walking or entering. Rebuild furniture/zone/door/hit-target anchors atomically; initialize affected actor frames at the current true goal as current seat-relocation logic does, without spurious aisle crossing. Preserve camera zoom. If an exact Agent reader is selected, translate camera by the selected displayed anchor's old→new delta so that Agent/reader remains at its existing screen position; also update saved return-camera context so closing does not jump to an obsolete layout. Keep exact selection, draft, channel and reader size untouched. For an unselected view preserve the existing world-camera intent, retaining its known resize limitation; do not auto-pan to the growing room or auto-fit the entire office on each refresh.

Stable room origin and seat indices remain unchanged when count stays in the same retained tier. Actual tier growth may shift shelves and reserve columns; it cannot preserve every world anchor simultaneously. Keep the selected subject fixed onscreen as above, and make no claim that a tier change is invisible. Do not animate room resizing or trigger entrance for relocated existing people. Message-flight endpoints follow shared new anchors or cancel under the earlier endpoint/reflow rules; accepted event facts and pending mail remain independent of geometry.

## Affected representative cases

| Case | Required result |
| --- | --- |
| One global GABI; project14 | Global capacity2 compact room, project capacity16, proportional independent widths; cat once outside rooms. Record actual dimensions against baseline. |
| 1→2→3→4→5 members | No growth at second/fourth within capacity; growth at third/fifth; existing slot order stable until tier transition, correct spare seats. |
| 8/12/16/17 and many | Tier threshold correctness, max-three normal spare, both zones fit all current people, bounded labels/props/targets at dense rows. |
| All idle→all active→mixed | Identical room size/packing/slot indices; existing walking connects each person's own rest/home anchors, no rest-index cascade. |
| Filter hides everyone or switches status | Capacity, room shape and actual population label unchanged; filter-result count remains distinct. |
| Partial list→more page→complete, error/recovery | Conservative growth, explicit loaded lower bound, no shrink on partial/failed reads or fabricated total. |
| Complete removals and history | Half-capacity threshold plus two complete observations shrinks; removed current members do not size via stopped records; empty history-only office stays compact. |
| Refresh reorder/same-name replacement | No random room/seat reshuffle; exact identity handling and true lifecycle event eligibility remain intact. |
| Selected reader during tier change | Same screen anchor/zoom, exact draft/channel/selection retained; close returns to an updated valid camera context. |
| 390px and resize | Narrow stack/side fallback fits declared world geometry, no fixture collisions, explicit panorama still available; do not treat responsive relocation as creation. |

This addition changes geometry, finite layout fixtures and affected continuity acceptance; it does not invalidate already retained native title/routing facts or justify rerunning unaffected suites. Lumo owns coherent implementation within current scene memory/projection; zila independently executes the combined candidate after it is reviewable.

# Native-state-specific activities and error precedence

This refinement supersedes the earlier generic “all non-running people rest” goal and inactive hollow-circle treatment. Population capacity/slots and the one motion clock remain unchanged. Friendly activities are visual metaphors for supplied states; a seated sleeper is not proof that a process can wake, and an ambient chat posture is not a real communication event.

## Authoritative status and supply gap

[`management/store.rs`, `live_view`](../../src/management/store.rs) calls [`agents/lifecycle.rs`, `get_instance_status_stored` / `instance_status`](../../src/agents/lifecycle.rs): the exported `AgentView.status` is **effective native state**, not merely the stored status column. A new placeholder can project `launching`, and heartbeat/staleness can project `inactive`. Process liveness is a separate fact. The Web `running` value is currently a presentation bucket; it is not a second native work state and must not override effective native `active`. Do not infer active from a nonzero PID, lack of error, a flight or an unfinished walk.

The current `projectAgent` collapses inactive→idle and blocked→failed while native `error` becomes unknown. This loses the distinctions required by the user. Retain effective native status as the scene's primary discriminator; keep compatibility pose buckets only where legacy fixture/filter consumers require them. Canonical current native values include `launching`, `active`, `listening`, `blocked`, `error`, `inactive`; stopped history is separately established by the exact stopped record. Unknown/future values remain explicit unknown, not assumed listening. Historical fixture aliases `working`, `idle` and `ready` must not be presented as proof of additional native runtime phases.

Read [the model-request error source](../../.docs/model/model-request-errors.md) and [`model_request_store.rs`](../../src/agents/model_request_store.rs): failed request enters `error` with context `model:request`; ordinary prompt/tool/heartbeat and another request start do not clear it. Actual successful request observation clears it, while cancelled requests do not establish recovery. Native inactive/exit and blocked approval can take precedence; another error source is not replaced by model-request state. Retained request-error metadata can coexist with an effective higher-priority state. Native owner must expose effective context and the existing current-session request-error facts needed for disclosure, using native declarations/generated types rather than parsing `detail` or duplicating frontend precedence rules. Existing management fields do not yet expose that structure, so distinguishing request errors by rendered text is insufficient. No new request retry/recovery operation belongs to this scene refinement.

## Selected primary visual mapping

| Effective native fact | Location / character | Workstation | Primary state cue / local description |
| --- | --- | --- | --- |
| active | Own home slot, `sit` facing up with existing typing frames | Lit, quiet existing screen animation | Small work `…`; 工作中. Does not claim model reasoning specifically. |
| listening | Own reserved lounge slot; quiet varied activities below | Idle/off screen | Small cup/quiet waiting cue; 等待消息. Never zzz. |
| inactive, ordinary/stale/dormant/exit context | Own home slot; `doze` seated facing down, or existing seated sleep art | Off, not flashing | `zzz`; 未活跃, with actual context/detail locally. This is a visual sleep metaphor, not confirmed resumability. |
| inactive with native launch_failed context | Own home slot, neutral still `idle` | Error X | `×`; 启动失败 with actual supplied detail, not an ordinary sleeping member. |
| blocked | Own home slot; brief/static arms-up `stretch` frame1 | Static neutral screen | Raised-hand posture plus `?`; 等待处理, actual reason locally (approval/permission only if supplied). Distinct from unknown-state question by raised hand and explicit label. |
| error + model:request context | Own home slot; neutral still seat, no typing loop | Error screen with visible X | Amber `!`; 模型请求失败, source reason and supported failure facts. Failure is not process death or autonomous recovery. |
| error from another/unknown source, or legacy explicit failed | Own home slot; neutral still seat | Error screen with visible X | Red `×`; 执行错误, actual source/context/detail when supplied. Do not call unknown source model-request failure. |
| launching | Home/eligible real entrance route, neutral idle until native active | Neutral/off | Small quiet `…`; 启动中. No premature typing or fake successful launch. |
| unknown/future state | Own home slot, neutral still idle | Neutral/off | `?`; 状态待核对 plus raw value. No assumed lounge work/sleep. |
| exact stopped record | No live person/desk ownership | Existing cabinet/history | No active state animation. |

Error uses a screen X as well as color. Model-request and generic error can share that monitor motif because source/context remain distinctly supplied and the head cues differ; do not collapse their native status/source into one failed bucket. When effective inactive/blocked overrides a retained model error, its primary row wins: off/zzz or raised-hand/? respectively, while the unresolved request error is disclosed as retained diagnostic context. Do not show active typing and model-error X concurrently because a low-priority callback suggested activity. The native projection decides precedence; Web consumes it.

`hallActorGoal` must select lounge only for effective listening (or explicit fixture waiting/ready aliases). Active/inactive/blocked/error/launching/unknown all use home. `sampleHallMotion` chooses terminal pose from the same semantic classification, and `HallArt` chooses screen/emote from that classification; avoid separate inconsistent state switches. A small pure presentation helper at the current scene owner is sufficient; it consumes facts and emits goal/pose/screen/glyph, not a general state-machine engine or invented lifecycle transition system.

## Quiet varied listening activities

Retain each person's own stable lounge anchor; no exercise route through neighboring seats, synchronized crowd animation or extra micro-walking is required. Hash the exact instance key once to choose phase and activity order. Use scene elapsed time, not repeated `Math.random` or each poll's array order. Select a deterministic 36–52 second cycle (per-key phase/period), with one 4–6 second activity segment and quiet baseline posture for the rest. Activity order can progress across cycles via hash(seed, cycle); it must not change when unrelated native snapshots arrive. Stop the segment immediately when native state leaves listening; do not queue it after the work route.

Available low-cost actions reuse existing art: `drink` with cup; `stretch` with arms raising; quiet `sit` facing down for relaxed resting; `talk` facing a neighboring resting direction as ambient posture. None emits a message, envelope, notification, simulated response or native action. Do not pair synthetic chat histories or imply a real recipient. A sofa action is eligible only where the actual population-sized room supplies a real unoccupied sofa anchor with clearance. Otherwise use relaxed sitting at the stable rest slot and omit the sofa visit. Do not enlarge the room or overlap another member to satisfy a decorative variant. Drink/stretch/relax/talk suffice in a one-person small lounge; nobody needs a fictional partner.

If a real sofa route is included at this scope, its target and occupancy must stay within the existing room's shared fixture geometry, use the same HallMotion path/retargeting and immediately yield to a native work/inactive/error transition. It is optional decoration; no separate scheduler/reservation subsystem is justified. Prefer in-slot relaxed sitting for this candidate if it avoids disturbing exact stable rest targets and arrival guarantees.

Listening variety is suppressed while a person's reader is open or the scene is hidden/paused; use a neutral static waiting pose. Reduced motion uses static cup/rest, fixed doze+zzz, raised hand, X and work indicators at current true goal. No slow substitute animations, blinking error spam or replay burst on resume. Typing/rest art frame updates remain on the existing shared clock and respect the existing pause/reduced-motion semantics.

## Integration and finite acceptance

Status and terminal cues update immediately, even while the one existing path moves from rest→home or home→rest. For example listening→inactive walks home then sleeps, listening→active walks home then types, active→listening walks to rest, blocked/error remain at home. A rapid listening→active→inactive retargets to home once and ends with off/zzz; it must never finish an obsolete drink or briefly type after inactive is current. Labels/hit targets follow current sampled frame; home mail stays on desk. No state transition changes population dimensions or another member's seat.

Independent cases include all canonical statuses, launching timeout/native inactive context, active vs process-live distinction, listening activity stability across unrelated polls, fixed sleeping desk vs stopped cabinet, blocked with retained request error, model-error retry-start/cancel persistence and actual-success clearance, generic lifecycle/launch failure distinct from model request, raw unsupported state, rapid retargeting, status during open reader, deterministic low-frequency variety, and reduced-motion/pause static cues. Use disposable native status/model-hook paths where available and clearly labeled browser-only finite art fixtures for timing/combinations that cannot be produced in the selected host. No real user-Agent status mutation or real vendor outage is required. Confirm ambient talk generates zero messages/flights and never increments either inbox representation.

Combined candidate remains implementation-pending; this design update does not claim native field support, rendered activity execution or final acceptance.

## Glyph geometry correction before candidate freeze

Lumo identified the actual emote bitmap footprint in #26273: 12×12 art pixels rendered at scale2 with bottom-center anchor. The earlier feet−42world upper-right anchor can overlap an 80px name row. Adopt **x = displayed sprite x +30world, y = displayed feet y −24world** for primary emote. Its top is feet−48world, while name-row bottom is feet−48×zoom−4screenpx, leaving a consistent 4screenpx vertical gap at every zoom. The icon's left edge is x+18world, just beyond the 16world half-width of the standing sprite; both remain inside a 120world work cell at near zoom. This supersedes all earlier +22/−42 emote coordinates in this document. Retain inline crown/name composition and shared moving-frame anchors. Independent dense/long-label/browser observation must confirm the rendered separation; no screenshot verdict is inferred from this calculation.

## Executed anchor and error-context corrections

Independent acceptance F01 establishes that the DOM button bottom already denotes the sprite foot. Final label bottom is `foot − 48 × zoom − 4 screen px`; do not subtract the 44px target again. The tooltip must exclude the entire actor/button/label union on every flip/clamp route, while retaining pointer access to its text. Final glyph anchor is sprite foot plus `(30, −24)` world units, superseding the earlier `(22, −42)` proposal. Corrected r3 actual desktop and narrow-reader evidence is linked from [acceptance](acceptance.md).

Independent native observation F02 establishes that `lifecycle.rs::get_instance_status_stored` simplifies `model:request` to public effective context `model`; the storage context named above is not the public API value. Classify the request-specific visual only inside effective error state, from current-session `request_error.source == model_request` plus authoritative compatible effective context (`model`, or full `model:request` where supplied). Other effective statuses and competing error contexts keep native precedence. Retained request-error metadata alone must not imply current error, and every actual error keeps the visible screen X. No description parsing or native state rewrite is needed.


## Current lounge refinement

The [owned refinement Task](lounge-refinement.md#causal-model-and-bounded-design) supersedes the earlier regular rest-seat grid, historical fixed Lobby corridor and human desk composition. `model.ts` projects stable staggered standing groups from full population slots and per-actor indoor bounds/furniture/door geometry. `navigation.ts` selects direct visible segments or shortest furniture-corner paths inside that room; unreachable paths hold without wall crossing. `motion.ts` retains the existing Leg sampler, speed cap, reduced-motion and interruption contract. Reflow remains coordinate-frame relocation, not a room transfer. Only shared lounge fixtures are drawn; listening defaults to standing with existing seeded cup/stretch/talk accents. Long listening names ellipsize within a world-scaled label width while full identity stays in focus/hover facts and accessible names. `office-art.ts` draws the standalone ginger cat and wicker nest at the unchanged scope-human anchor; human feed, addressed envelope endpoints and absence of human read acknowledgement remain unchanged. Exact source/artifact/browser evidence and selected running version are owned by the Task, rather than inferred from this design.
