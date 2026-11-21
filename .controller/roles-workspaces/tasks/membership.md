---
schema: task
status: completed
description: Persist collaboration metadata and enforce one default endpoint in native launch reservation.
---

Read [team contracts](../AGENTS.md). Own the membership/native endpoint slice. Preserve launch-context keys, incarnation fences and transaction rollback. Check ambiguity, changed default role, concurrent reservation, error state and definitive retirement. Publish the typed skeleton promptly. Add native unit cases; leave Cargo execution to controller scheduling. Report source handoff here.

## Native implementation model

Owner: rozi; integration receiver: bime; snapshot consumer: soba. Typed skeleton published through AgentX. The existing native launch write scope owns endpoint lookup/adoption, uniqueness recheck, participant creation, collaboration persistence, and endpoint binding; host process creation remains outside it. The endpoint key is the serialized workspace (including explicit global), and its value stores the exact role path and ParticipantRef. A changed role requires explicit reset after confirmed retirement. A missing row alone, inactive/error status, elapsed time, or unknown process observation never releases a binding. Exact stopped-event incarnation evidence permits retirement. Membership is frozen adopted data; incoming context captures preserve it.

| Case | Expected native result |
| --- | --- |
| Legacy/unassigned, explicit global, local member | Distinct snapshots; cwd changes do not change membership |
| One compatible existing role/workspace member | Adopt exact participant under write scope |
| Multiple compatible members | Conflict, no arbitrary recipient |
| Racing first reservations | One participant/binding; loser conflicts before host spawn |
| Bound pending, error or unknown participant | Preserve and return the existing participant |
| Role path changed | Conflict until explicit retired-endpoint reset |
| Missing/stale row without exact retirement evidence | Reject; never speculate about replacement |
| Exact retirement | Same-role replacement allowed; reset allows changed role |
| Required DB write fails | Roll back participant, context, endpoint and retained intent together |

Integration dependencies: bime confirmed the launch_context addition to db::get_instance_snapshot and preservation in binding::store_launch_context; soba owns ResumeSnapshot history extraction. No Cargo or child agents are permitted in this assignment.

## Source handoff — rozi

Implementation is released to bime in [workspaces.rs](../../../src/agents/workspaces.rs), [launch_service.rs](../../../src/agents/launch_service.rs), [launch_store.rs](../../../src/agents/launch_store.rs), [invocation_binding.rs](../../../src/agents/invocation_binding.rs), and the [module entry](../../../src/agents/mod.rs). Public contracts are LaunchCollaboration, collaboration(row), workspace(row), Reserve.collaboration, Request::EndpointLookup, Request::EndpointReset and Response::Endpoint. The duplicate reservation error is ServiceError::Conflict with `workspace endpoint reservation conflict: already reserved by '<name>'`, allowing the sender to reread the winner without spawning again.

The native transaction persists the adopted snapshot before any bootstrap, reserves the endpoint with the exact created_at bits, and retains the original launch result. Lookup adopts one compatible local participant; ambiguity, role-path drift and stale or missing ownership produce explicit failures. Bound startup failures keep a visible launch_failed row and occupancy. Explicit-name replacement cannot bypass a still-owned endpoint. Canonical session migration transfers a lease only inside the existing binding transaction, after validating old ownership and matching adopted membership. There are no process creation, signaling or wait effects in the new policy.

Nine native cases are authored under `agents::workspaces::tests`, covering snapshot/terminal/recapture preservation, legacy/global/roleless membership, compatible adoption, ambiguity, concurrent reservation with retained replay, rollback, pending/error occupancy, absent/stale incarnation fences, and retirement/reset replay. Executed here: standalone rustfmt on the four changed implementation files and git diff --check on owned paths. Cargo compilation and test execution are deliberately deferred to bime/independent acceptance; no executable support is claimed from this source handoff. The existing launch_store/acceptance_tests.rs Reserve literal needs collaboration: None; bime was notified because that file is outside the five-file assignment.

At source release, integration still needed the native filter and launch/binding/lifecycle checks, connection through the controller's send path, and the assigned snapshot changes. EndpointReset is a native contract; the roles Task adds its CLI command. Those checks and integration are recorded below and in the completed Delivery.

Compatibility follow-up: request #18128 identified that older epoch-2 daemons ignore the optional Reserve.collaboration field. Added Request::WorkspaceSupport returning Response::Changed without a DB transaction or state mutation; the controller probes this explicit variant before collaboration-dependent launches so older decoders reject rather than silently losing membership. Only launch_service.rs and launch_store.rs changed for this native probe. Standalone rustfmt and owned-path diff checks passed; no Cargo execution here.

Integration executed all nine native workspace cases and the launch/binding suite. Independent membership, exact parent delegation, ambiguous adoption and changed-role rejection passed; concurrent evidence already established a unique native endpoint/Provider, with remaining startup-message admission owned by CLI integration. Exact artifacts, scope and final CLI outcome are retained in [integration](integration.md) and [acceptance](acceptance.md). This native slice is complete; no live daemon or database was replaced.
