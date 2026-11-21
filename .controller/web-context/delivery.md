---
schema: delivery
description: Existing embedded conversations with durable text, image and file context.
status: completed
stages:
  implementation: {description: Design and implement the integrated native and Web path., tasks: [./implementation.md]}
  acceptance: {description: Independently exercise the real cooperating path., tasks: [./acceptance.md], depends_on: [{stage: implementation, condition: Usable contracts and integrated candidate are handed off.}]}
---

# Embedded conversation delivery

## Outcome and acceptance

Implement the [selected Proposal](../../.docs/proposals/web-context.md) in the existing Agent Space conversation and daemon management service. Evidence must exercise iframe hosting, query focus/compact layout, parent-to-child text and binary context, file selection/drop/paste, real disk persistence, browser preview/download, and deliberate native message delivery containing readable attachment references. Verify exact-recipient isolation, clear upload failures, attachment-only messages, and absence of mutations on query navigation/reload. A local file reference is not proof of native Provider image ingestion.

## Responsibilities and workflow

| Responsibility | Owner | Result and consumer |
| --- | --- | --- |
| Integrated professional design, native contracts/storage/HTTP, Web composer/embedding and documentation | One qualified coder, [implementation](implementation.md) | Consumable model and integrated implementation for existing users and independent acceptance |
| Scenario-driven real native/browser acceptance | Separate qualified coder, [acceptance](acceptance.md) | Bounded observations of the promised consumer path, plus actionable defects returned to implementation owner |
| Scope, coordination and truthful closure | nube | Recoverable source links, runtime bindings and accepted actual outcome |

The integrated implementation stays with one owner because native uploads, send normalization, preview and composer state form one cohesive feature. Independent acceptance is justified by the new durable file/message behavior and the project's complex-task requirement; no additional review tier is planned. Implementation may run necessary focused development checks; the acceptance owner runs combined runtime/browser verification once the candidate is usable.

## Resources and bindings

Preserve current live services, installed binaries, older previews and all unrelated dirty source. Candidate packaging and tests use isolated output, disposable scopes and non-default ports. No service cutover is included.

| Instance | Professional binding | Resource scope | Last observation and next condition |
| --- | --- | --- | --- |
| tuli | coder, integrated implementation | Implementation Task write scope, including canonical design and affected native/Web code | AgentX batch `62da7774` reported ready on 2026-11-15; recover assignment and publish usable design/contracts, then continue implementation |
| muvu | coder, independent consumer acceptance | Acceptance Task and `tests/web_context/`, isolated runtime/browser resources | AgentX batch `4f9f360e` reported ready on 2026-11-15; derive independent cases now, execute combined checks when implementation candidate is usable |

Both launches use project coder profile (`gpt-6.1-sol`) with explicit `model_reasoning_effort="max"` override. No delegation. The owners communicate actionable integration dependencies directly; nube receives design/handoff/defect/completion notifications, rather than polling.

## Remaining work

No required work remains in this Delivery. Implementation documentation/Task closure, independent consumer acceptance and separately owned selected-service adoption/read-only checks are complete. [Feature entry](AGENTS.md) records user-selected limits; exact semantics belong to the canonical design and native contracts.

## Accepted local outcome

On 2026-11-15, nube consumed muvu #28508 and accepted the integrated web-context contribution on r3 SHA-256 `4f175b5d39ed02852dca6f493a336a5bb1d7f6e596f1e3c826eee7f4adff005a`. [Final judgment](../../tests/web_context/evidence/acceptance.json) and [case/run evidence](../../tests/web_context/evidence.md) establish real public-HTTP/generated-hook delivery, durable bytes and message projections, cross-origin iframe context, file/image/Blob intake, deliberate send, exact-target fencing and repaired original-operation pointer recovery at 450px and 360px. All seven fixture runs cleaned their owned resources. Original failures and the harness-only trailing stale-ref failure remain preserved; they do not invalidate the scoped passing product evidence.

The local result supports the selected existing-conversation feature. It does not establish paid Provider image ingestion, OS-native clipboard/drag behavior, a packaged browser extension, exhaustive cases or optimized performance. The nonblocking popup text-separation observation does not warrant another build. Tuli closes documentation/implementation ownership only; no further test or polish gate is imposed. Web-context prerequisite for the separately authorized single combined service adoption is released to zomu/libu; gabi retains binding post-adoption observation. Accepted source/local behavior does not itself claim running service adoption.

## Separately owned actual adoption

Libu #28583 and zomu #28604 confirm the exact accepted r3 now serves `http://192.168.2.178:38023/space/`, retaining the existing scope and generation. Their authoritative [adoption receipt](../office-details/verification/lounge/rollout-after.json) owns actual process/service evidence. Gabi #28629 confirms the [read-only binding live check](../file-task-bindings/evidence/space/live-r3/report.json) passed on that artifact, with no POST or page errors. Actual native binding count was zero and remains an honest empty state; fixture evidence owns populated/error/unknown variants. Libu retains scene/asset live smoke and its own closure. This service adoption is consumed from the named owners, not a second restart or full native acceptance performed by web-context.

## Formal closure

Libu #28665 confirms final scene/API/all-14-assets/resource read-only smoke passed and its [scene assignment closed](../office-details/lounge-refinement.md#completed-outcome-and-limits). Tuli #28677 completed the [implementation Task](implementation.md) and reconciled [canonical design/usage](../../.docs/agent-management/web-context.md) to accepted actual support, without additional source edits/build/tests or live actions. Nube accepts the scoped contributions and their composition, closes this Delivery, and releases both professional bindings at their declared boundaries. No further work, service action or review gate remains; future changes start from module guidance and the current design, preserving the documented limits and original evidence.

## Integrated candidate handoff

Tuli's #27584 handed off `.bin/agentx-web-context-r1`, SHA-256 `68f2fcd9f5828cc5f7f2b55da7be681c612dff133fccfb6a4727315257b8e444`, with `.source.json` affected-source manifest and `.receipt.json` embedded asset/scene-source receipt beside it. [Implementation](implementation.md) owns focused test/package evidence; [acceptance](acceptance.md) owns actual cooperating native/browser evidence. Muvu is released to execute against this immutable candidate in disposable resources and return actionable defects directly to tuli. Tuli remains assigned for ordinary repairs. Zomu/libu received the artifact boundary for their single selected-service rollout; this handoff is not yet adoption acceptance. No web-context live services were started or changed by implementation.

## Acceptance disposition

Muvu #27691/#27706 established native path delivery, existing-text/attachment-only sends, operation replay/conflict and disk/reference persistence across owned-daemon restart. Browser WC01–WC12 and lost-admission/no-resend also passed. One substantive compact receipt defect remains: at 450px child width, the expanded `.live-receipts` panel extends outside the iframe and prevents pointer access to original-operation recovery. Evidence and exact execution remain in [acceptance](acceptance.md), including r1-b screenshot/trace. Tuli owns layout-cause correction under its current assignment; muvu consumes the repaired candidate for remaining and affected-only checks. Selected-service adoption remains pending. The independent executor reports both disposable daemons cleaned with no cleanup errors; existing live services remain unchanged.

Repair progression: muvu #27811 completed unaffected remaining browser evidence and cleaned all a/b/c disposable processes/roots. Tuli #27831 reports a CSS-only correction passing Chrome pointer layout checks at 390/450px; native and lounge scene inputs are unchanged. R2 packaging then reported signal 15 before version-checked handoff (#27867), with a PS1 diagnostic in `/tmp/web-context-package-r2.log`. Tuli owns reconciliation of build/process/output/receipts and safe recovery; no duplicate build or service action is dispatched. R2 is not yet a usable acceptance artifact. Retain established r1 evidence; only real repaired popup recovery/layout remains to execute once an immutable usable candidate is handed off.

## Design contribution received

On 2026-11-15, tuli published the [canonical design and public boundary model](../../.docs/agent-management/web-context.md) with native upload/message references, exact query targeting, correlated parent `addContext`, composer behavior and durable file lifetime. This matches the selected scope and is usable by muvu for consumer-case preparation. It establishes design/skeleton readiness only; native/browser behavior remains under implementation and independent acceptance. Controller returned one bounded documentation clarification to the existing owners: iframe structured clone and browser-extension runtime serialization have different binary transport behavior, so wrapper examples must preserve that distinction.

Packaging friction: tuli's first integrated build observed an unrelated scene import before its concurrent `humanCatNest` export existed. On receipt of #27283 the controller found the export already present in current `office-art.ts`, returned that source evidence and released the affected packaging retry without editing another owner's scene. This observation establishes disappearance of the reported source mismatch, not a successful build; candidate/acceptance remain owned by tuli/muvu.

## Shared artifact and service coordination

On #27467, zomu reported the lounge refinement candidate shares the current working tree and Cargo target with web-context. Web-context owns no selected `:38023` restart or rollout; zomu/libu retain that operation under their [existing lounge assignment](../office-details/lounge-refinement.md). Tuli freezes only its affected implementation inputs for the final candidate build, not the whole working tree. Muvu's independent consumer execution remains a prerequisite for adopting a combined artifact containing this new native functionality. Controller returned the ordering to all affected owners: hand off immutable candidate/source evidence, finish the bounded web-context acceptance, then let the single lounge rollout owner select an artifact that contains the accepted combined inputs. A differently named lounge executable is not evidence that web-context changes are excluded. No competing restart, overwrite or duplicate broad acceptance is authorized by this coordination.

Gabi #28071 added the separately authorized [Agent Space task-bind UI](../file-task-bindings/space.md) to the shared artifact coordination. Tuli remains sole current candidate producer and must establish actual captured BoundWork assets/source inclusion with gabi, rather than infer inclusion from current working tree or launch time. Gabi owns affected binding UI smoke/read-only live evidence; muvu retains web-context consumer acceptance, and zomu/libu retain the single selected-service adoption. Reuse one combined artifact where its actual included inputs and bounded acceptance support it; missing inputs justify only the necessary coordinated subsequent build, not parallel packaging or an invented deployed claim.
