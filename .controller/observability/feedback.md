---
schema: document
description: Concrete observability design, implementation and team-workflow friction with owned disposition.
---

# Design feedback

## Completed capability hidden by stale discovery context

During the user's post-delivery Roadmap discussion, `.docs/model/README.md` still described observation instrumentation and consumers as undelivered, while the original accepted Deliveries and LAN Task already contained executable evidence. The bounded correction links those actual outcomes and leaves unverified platform, release and future-management claims separate. The [Roadmap](../../.docs/roadmap.md) now records new directions as candidates rather than confusing stale prose with missing production behavior or creating automatic implementation assignments. Original accepted evidence and completion states are unchanged.

## Unavailable local failure timestamp

Composition found that mandatory `LocalFailure.at` could not represent a failure when the fallible observation clock itself could not supply a timestamp. Its moving adapter substituted `u64::MAX`, fabricating temporal evidence. The [bounded architecture decision](implementation-tasks/native-contracts.md#clock-unavailable-local-failure-reconciliation) retains the existing shared clock and separates the known failure cause from availability of its capture time.

The [Runtime correction](implementation-tasks/runtime.md#clock-unavailable-local-failure-reconciliation) owns `LocalFailure.at: Option<ObservationTime>`, `ClockUnavailable`, report validation and the added authored case. A missing timestamp preserves independently known stage, kind and OS code, but establishes no order or age. Failure of the clock facility itself has neither a timestamp nor an OS code. Required report start/end/span times remain required. [Composition](implementation-tasks/composition.md) owns removal of the sentinel and actual adapter mapping; [native integration](implementation-tasks/native-integration.md) owns fixed-candidate execution. This is one concrete implementation correction with separate source releases, not a new design or review cycle; authored cases remain unexecuted until independent evidence exists.

## Team control path

The user clarified that development coordination uses AgentX itself. Follow [AgentX-native orchestration](../domain-daemon/orchestration.md) and the [implementation team contract](implementation-team.md#agent-platform). Earlier external-platform bindings remain historical execution evidence, not requirements or live assignments. Preserve the separate disposable product-test boundary and unrelated user services.

## Initial boundary

The [original user discovery](../domain-daemon/feedback.md#management-observability-discovery) supplies the need. The user has now explicitly selected a design Delivery and team. The completed daemon implementation remains the current baseline; centralized persistence does not imply universal observation of client connection failures, host effects or Provider internals.

Record only substantive ambiguity, contract incompatibility or workflow friction with the affected consumer, professional decision owner and resolution. Domain rules stay in their owning design rather than being duplicated here.

## Baseline documentation drift

Architecture reported that [the shared model entry](../../.docs/model/README.md#current-evidence-and-target) still described command/hook/PTY/Relay database constructors as current production behavior and the daemon as a future target. This contradicted the [accepted daemon outcome](../domain-daemon/delivery.md#accepted-local-outcome) and the current native service/storage guides, risking redundant migration work and incorrect observability ownership assumptions.

The controller made a bounded correction to that existing section after architecture released its separate observability route. It now links the implemented scoped client/daemon baseline, labels the old DB inventory as historical migration evidence, preserves exact archive/helper permissions by reference, and distinguishes the new observation design from delivered support. The matching [roadmap paragraph](../../.docs/roadmap.md) now delegates implementation/support claims to that accepted Delivery rather than repeating the same obsolete target-only statement. No native declaration, observation contract, production source or prior executable evidence changed. Independent design integration receives these corrected entries as inputs and includes their documentation links in its existing final checks.

## Observation disclosure across joined records

Architecture found that the initial Events design exposed fields which, when joined across records, reconstructed a native output-confirmation token. The [architecture Task](tasks/architecture.md#early-contract-release-and-decisions) records the source-grounded finding and Events owner's correction. The [reconciled contract decision](../../.docs/observability/contracts.md#reconciled-boundary-decisions) and [domain evidence projection](../../.docs/observability/events.md#domain-and-host-evidence-boundaries) own the exact selected rule: independent observation references and bounded confirmation projections preserve investigation without exporting native mutation evidence.

This is a resolved design defect, not an executed exploit or a production-code change. Independent integration consumes the corrected projection and checks cross-record disclosure as part of its existing contract walkthrough. Future changes must assess the combination of exposed fields and records; individually omitted token fields do not establish that the overall read projection is non-authorizing.

## Shared references and reported-origin provenance

Events' initial shared epoch/native-home draft conflicted with architecture ownership. The owner resolved this by importing the shared dataset epoch, keeping collector identity in its separate stream type, and using the selected `src/observability/` declaration home. The [reconciled decisions](../../.docs/observability/contracts.md#reconciled-boundary-decisions) own the exact identity rules; future consumers must not redefine these shared types for local convenience.

Runtime and Events also reconciled report intake so a historic client claim retains a separately authenticated current submitting peer. The [event envelope and validation](../../.docs/observability/events.md#envelope-payloads-and-correlation) import runtime's exact `AcceptedPeer` wrapper; identical claimed attempt UUIDs from different submitters do not erase that provenance or establish a server-observed request. Events' final release records both corrections in its [original Task](tasks/events.md#working-record). They are domain-design resolutions for independent consumption, with no production behavior or runtime evidence claimed.

## Provider high-demand interruption

The runtime and configuration design sessions errored with the provider's explicit high-demand message before their final releases. Original Task/source findings and the materialized configuration draft were preserved; the runtime's undeclared exact fields remain missing input until its owner publishes them. The controller confirmed ended turns and resumed the same bindings once with an explicit continue-from-files instruction; no duplicate owners, new model choice, source mutation or acceptance cycle followed. Architecture independently sent recovery prompts before receiving that notice. Further recovery belongs to the controller so several consumers do not repeatedly steer the same failed session. Domain owners remain responsible for the unfinished exact-contract handoff, and integration must not treat the environment error as either semantic failure or completed design.


## Direct-use correction selected by the user

The accepted browser design required a single-use launch URL and a separately specified asset directory, which obstructed ordinary LAN visits, reloads and standalone distribution. The user explicitly removed that requirement and authorized a real LAN demonstration. [The follow-up Task](lan-usability.md) owns this bounded correction and its executable evidence; historical acceptance remains unchanged. Default embedded assets and direct page connection replace user-facing bootstrap and deployment operands while finite read scope and same-origin boundaries remain enforced.


## Ordinary product-input sizing versus isolated row fixtures

The [LAN follow-up](lan-usability.md#real-use-blocker-ordinary-request-rows-exceed-detail-cap) found a substantive product-use gap: all66 ordinary CLI request details were dropped by the2048-byte cap, leaving the actual console empty despite successful business calls. Native scoped identities, full lifecycle timestamps and configuration refs must be sized together. The accepted correction uses one finite8KiB request cap consistently at decode/update/completion, preserves2KiB connection and shared2MiB history bounds, adds a full-shape regression, and is confirmed by actual CLI-generated request/config/event data. Earlier short/constructed acceptance fixtures remain historically passed but did not establish ordinary CLI detail retention. Do not mask coverage loss with synthetic rows or weaken identity/provenance to fit arbitrary budgets.
