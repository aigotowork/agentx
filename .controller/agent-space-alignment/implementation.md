---
schema: task
status: completed
description: Implement the accepted AgentX office/role/Agent semantics in the offline visual frontend.
links: [./task.md, ../../.docs/agent-space/concept-alignment.md, ../../web/agent-space/AGENTS.md]
---

# Offline AgentX semantic adaptation

## Authorization and scope

The user approved the concept-alignment plan and requested proceeding. They also asked whether workspace/role are missing Agent metadata: native launch records already retain adopted collaboration membership/role; current management projections omit those facts. This assignment implements slice 1 of the accepted design, with complete offline office/member/post behavior, and leaves production API supply and integration with the independently owned API work. Do not edit API/native source, call real endpoints, start real Agents from UI, deploy or replace user services.

Preserve the user LAN Vite preview `0.0.0.0:18967`, PID 3050612 (exec session 32599), and its existing hot reload. Do not stop/restart it. Reuse source art, scene/chat and camera/draft fixes. Original migration/provenance and verification remain historical evidence; new behavior needs its own scoped evidence.

## Responsibility and finite writes

One qualified coder owns cohesive frontend design/skeleton/implementation and ordinary repairs. The domain concept design is already usable; do not create another domain/API design. Work is delegated because this Controller owns coordination, while the qualified coder owns nontrivial UI/state modeling and implementation. No delegation.

Own `web/agent-space/**`, `.controller/agent-space-alignment/frontend-work.md`, and design updates that distinguish selected rules from implemented slice-1 behavior in `.docs/agent-space/concept-alignment.md` (do not alter native semantic conclusions). Keep .controller/agent-space-alignment/implementation.md and verification.md controller/executor owned. Root manifests/dependencies should remain unchanged; coordinate genuine root changes with zumi. Preserve unrelated edits and all original license/provenance files.

## Usable result

Apply the accepted design's field/feature dispositions and concrete UI plan proportionately:

- Default large-office overview of known canonical directory offices and one separate global-member area. Exact directory membership drives grouping/counts; cwd and inherited/global Role origin do not. An empty known office and a nested distinct office remain visible. Display scope, sample/coverage provenance compactly.
- Named actual Agent people (`@name`), with exact synthetic scope/generation/current-or-stopped reference keys, adopted role/provenance separate from current available Role definitions; confirmed no Role and unknown Role/membership are distinct. Unknown membership never becomes global. Same name/different incarnation must not share drafts/history/actions. Keep confirmed current members separate from explicit stopped history; inactive alone does not mean stopped/success.
- Local member/posts/Skills/history navigation for selected office. Roles are available posts with origin and profile details, zero/multiple instances; selecting a post never creates/selects a fake person or launches anything. Skill information is honestly unavailable without supplied facts. No inert simulated-success operations.
- Include meaningful illustrative cases: two actual named members of one inherited role, global-origin role in local office, local member with cwd in a different/nested directory, an empty marked office, global member, stopped/namesake history, no-role and unknown facts. All are synthetic examples, not observations of live Agents.
- Separate AgentX communication from Provider model records in the reader, including actual authored sender/recipient labels and honest unavailable branches. Local notes remain separately identifiable, never pretend to deliver or fabricate replies.
- Preserve one controller, per-exact-reference drafts/selection, scope-specific camera/return memory and existing no-zoom-on-select, resize/mode/close/panorama guarantees. A window or tab change never sends. Narrow/short composer stays reachable. Keep interface compact and source recognizable.

Use source-derived presentation-local declarations and a replaceable supply; do not publish competing HTTP schemas or copy native internals into a second backend. Record concise structural design/cases before behavior implementation, then implement coherent boundaries. Keep module README/AGENTS/current-model entry accurate so a fresh agent can discover implemented versus future slices.

## Verification ownership

Implementer may run scoped development typecheck after coherent edits, author meaningful state cases and update browser scenarios where changed promises require it. Do not repeat final build/browser execution: a separately bound independent coder executor will own final typecheck/build, semantic state cases and desktop/390/320/short browser behavior after handoff. Provide exact interaction targets, changed promises, authored cases and readiness to zumi. Retain ordinary repairs through final execution, then release writes. No broad review/polish gate.

## Acceptance

The live preview reflects slice-1 AgentX semantics with no legacy domain leakage or unsupported native claims; final scoped build and meaningful state/browser checks pass at the declared boundary. Existing camera/draft/short-screen regression guarantees hold on affected paths. Source/service/API integration and durable real delivery remain deferred, not silently simulated. Controller accepts the implemented slice and independent evidence; there is no per-file approval gate.

## Binding

Implementer: kazu, role `coder`, profile `codex-gpt-6-1-sol-high`, runtime ready in batch `470ad5a1`; assignment accepted in message #21461; source ready for independent execution in #21894. Owns the finite frontend/design writes above; no delegation. Controller zumi owns acceptance and executor binding. Kazu released source writes in #22326; pura completed independent execution and cleanup in #22382 via [verification](verification.md), batch `ff113645`. Both bounded assignments are closed.

## API assessment request

The user explicitly authorized sending the factual gaps to gabi for assessment. [Consumer requirements](../../.docs/agent-space/api-supply-request.md) collect priorities and decisive cases, referencing the canonical concept design rather than defining new DTOs/endpoints. Gabi owns the API disposition; frontend slice 1 proceeds independently.

Gabi returned the [API-owner assessment](../../.docs/agent-management/office-supply-assessment.md) in #22093. It supports the semantic plan and distinguishes member read projections, bounded catalogs/pure routing preview and later actions. This is consumed as future integration guidance, not a claim of implemented supply or a new native assignment under this offline Task.

## Accepted offline outcome

Zumi accepts slice 1 against the approved concept design. Directory offices, exact membership and separate global/unknown areas, available/adopted posts, named current/stopped Agent identities, unavailable Skills, separate communication/model/local-note readers, exact-key draft isolation and per-office camera continuity are implemented. The inherited-role, empty/nested-office, cwd/membership, unknown/no-Role and stopped namesake cases are synthetic and explicitly offline.

The [source work](frontend-work.md) and [completed independent execution](verification.md) are authoritative for design, one substantive scene placement repair and final evidence. Pure state passed 7/7; initial browser S1–S8 passed 9/9; repaired final scoped build/typecheck and affected browser/camera/room-placement checks passed. No remaining substantive source defect was reported. Executor-owned resources were cleaned and the user LAN preview remains available. No additional checks or API implementation are required to close this offline slice.

Future native supply/integration remains separate: [API assessment](../../.docs/agent-management/office-supply-assessment.md) owns the member-projection, bounded catalog/pure endpoint inspection and later action recommendations. This acceptance does not establish live Agent delivery, durable storage, real Provider Skill use or physical-mobile keyboard support.
