---
schema: task
status: completed
owner: space-responsive-vaneda
related: [../../web/agent-space/AGENTS.md, ../../.docs/agent-management/web-context.md, ../office-details/space-uiux-rollout.task.md]
---

# Space routes and reproducible navigation

## Authorized outcome

On 2026-11-16, @bigboss requested routes throughout Space: entering any panel should produce a copyable URL that opens the same meaningful scene elsewhere. Navigation must work through normal UI, initial direct links, reload, a new browser tab and browser Back/Forward. Scope includes the existing complete responsive Space, not a separate app. Existing `live/context.ts` only reads a one-shot focus query; the missing capability is ongoing bidirectional URL/state navigation.

## Responsibility and consumers

One qualified Web coder owns the navigation domain design, skeleton, implementation, compatibility with existing context/iframe callers, meaningful scoped checks and developer handoff. This requires specialist state/lifecycle work beyond the Controller role and warrants a fresh focused context. The owner inventories all actual navigable surfaces before choosing contracts: office, presentation, resource tabs/details, exact current/stopped conversation and record channel, filters and visible mailbox/read panels where applicable. Distinguish stable navigable state from transient actions, drafts/uploads, private text, hover, animation and camera state; explain exclusions and any unresolved scope in the design. Do not put secrets or message drafts into URLs or execute mutations on route restoration.

The professional owner selects the simplest URL mechanism compatible with native `/space/` hosting and existing embedding clients, defines parsing/validation/canonicalization/history policy, and handles asynchronously loaded/missing/stale resources without guessing another same-name incarnation. Preserve unrelated query hints and safe context bridge behavior. Supply refreshes and draft edits must not flood history or overwrite a pending deep link; invalid URLs must show useful bounded feedback. Preserve exact per-conversation drafts and current scene/camera behavior during navigation.

Controller consumes the design/handoff and assigns one independent browser execution after a usable contract/skeleton exists. Acceptance exercises real UI-to-URL-to-restored-state transitions, including async supply and exact identity, beyond codec-only tests. Stop after appropriate checks pass. Native contract or service changes require a concrete dependency report to Controller, not speculative expansion.

## Context and ownership

Start from root `AGENTS.md`, module `web/agent-space/AGENTS.md`, local coder/interface methods, existing `src/App.tsx`, `src/offline/controller.ts`, `src/live/context.ts`, responsive/context docs and affected panel owners. Canonical UIUX files live under `web/agent-space/uiux`. Preserve the extensively dirty shared workspace; no commits or unrelated cleanup.

Implementation write scope: `web/agent-space/**`, `.docs/agent-space/routes.md` (canonical new usage/design), bounded route updates to `.docs/agent-management/web-context.md` if needed, and implementation sections here. Controller owns this entry/root guidance and coordination sections. Verification writes `tests/space_routes/**`, isolated evidence and its Task section, not implementation files. Implementation may author local codec/state tests. No Cargo/native builds, replacement of installed programs or service restart. Use isolated non-default ports and disposable scopes for evidence; existing selected :38023 and other previews remain protected.

## Completion

Deliver a source-linked navigation model and coverage matrix, implemented full consumer integration, documented copyable URL examples, actual scoped check results, explicit remaining limits, and clean ownership of temporary resources. Source/build/browser evidence and selected live adoption are separate outcomes. Escalate actual live deployment dependency after the reviewable artifact is ready; do not imply this source task updates :38023.

## Bindings and current state

- Controller: space-responsive-vaneda; coordinates scope, independent verification and acceptance.
- Implementation: actual identity `space-routes-dune`, batch `e8a52f5b`, local `coder` profile; assignment accepted in #36162. Owns the implementation write scope above with no delegation. Next condition: publish navigation model/skeleton and actual consumer scenarios; continue implementation through handoff.
- Independent verification: actual identity `space-routes-verify-gila`, batch `356fcfee`, local `coder` profile; [assigned Task](verification.md). Assignment accepted in #36240; preparing scenarios and coordinating directly with implementation for source release. Owns final product build/browser evidence, tests and isolated resources, with no product repairs or delegation.

Design/skeleton handoff #36193 is usable: [canonical navigation model and surface/scenario inventory](../../.docs/agent-space/routes.md). Query + History API fit the current host and existing callers. Complete source handoff #36326 is released to independent build/browser execution; implementation retains ordinary repairs under the same assignment. Codec/state scoped checks passed 10/10 and are reused; final product build/typecheck/browser evidence stays with verification. Next condition is the independent consumer result and any substantive repairs, then source acceptance and rollout handoff.

## Accepted source outcome and rollout handoff

On 2026-11-16, Controller accepted implementation #36326 and independent final result #36401 at this Task's source/browser boundary. [Canonical usage/model](../../.docs/agent-space/routes.md) maps actual navigable resources to the existing query/History API; this serves the original copy/open-same-scene goal without adding a parallel router/UI or native contracts. [Independent execution](verification.md#completed-independent-outcome) establishes final build/typecheck, finite R1–R9 real UI/direct-link/reload/history/async-identity/iframe/responsive paths and GET-only restoration against explicitly simulated native-shaped API supply. Existing route/context cases passed 10/10 and were reused. No actionable product defects remain; no additional verification is assigned.

Reviewable final Web artifact: `/tmp/space-routes-gila-final-r1`. [Evidence summary](../../.artifacts/space-routes-verification/summary.json) links unchanged source/artifact hashes, build output, traces, traffic and retained corrected harness assumption. Owned browser/server resources are closed, including isolated :19087; source owners release their assignments. Linux Chrome finite consumer support does not establish cross-browser or live native service adoption.

Selected :38023 remains on the prior accepted artifact. Native embedding/package and supported same-scope rollout are a separate integration handoff to @bigboss, using the retained build/source manifest and [current service adoption contract](../office-details/space-uiux-rollout.task.md#completed-actual-selected-service-adoption). This completed source Task does not claim the running service has routes; a deployment owner must establish package identity and actual served behavior before announcing live availability. No installed/default executable or running user service was changed.

## Observed issue

Existing query handling applies `view`, `office` and exact Agent focus once at mount. The remaining scene, resource and reader state is page/controller-local; UI actions leave location unchanged. This is the cause of unshareable navigation, rather than missing native resource identity.

## Implementation-owned handoff

Actual identity: `space-routes-dune`; assignment accepted to Controller in AgentX. Design recorded before skeleton in [canonical navigation](../../.docs/agent-space/routes.md), including complete actual-surface inventory, history policy, asynchronous exact resolution, privacy/effect exclusions and consumer scenarios. Skeleton then implemented in `src/navigation/{route,browser}.ts`; App and actual scene/reader/resource/UIUX consumers are integrated. Existing context bridge remains independently exact and revision-fenced; no native API contract or service change is required.

Scoped route/context cases passed 10/10 once; initial scoped typecheck passed before final read-adapter/consumer updates. Independent `space-routes-verify-gila` owns final product build/typecheck and browser consumption. Build/browser evidence and final acceptance belong to [verification](verification.md), separately from selected :38023 adoption. No Cargo, service restart, delegation, commits or unrelated cleanup performed. CBM daemon discovery timed out; source-only inspection informed the inventory, recorded in the canonical design.

### Implementation completed and independently consumed

The promised Web source outcome is usable and independently consumed. [Final verification](verification.md#completed-independent-outcome) passed the production build/compile and finite R1–R9 browser scenarios against unchanged source, reusing implementer 10/10 checks. No product repairs were requested; the retained initial browser failure was a harness disclosure assumption only. [Canonical design/current usage](../../.docs/agent-space/routes.md#verified-source-outcome) links the immutable source/artifact and actual browser evidence.

Implementation ownership is released at this handoff; no temporary service/browser resources were created by the implementer and the verifier cleaned up its own resources. Controller owns Task acceptance/closure and any separately authorized native packaging/live adoption. Current :38023 remains unchanged. Limits: Chrome/Linux finite scenarios with simulated native-shaped API supply; no native/backend/live rollout claim.
