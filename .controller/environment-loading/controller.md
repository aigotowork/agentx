---
schema: controller
scope: ../../
deliveries: [./delivery.md]
tasks: [./tasks/native.md, ./tasks/acceptance.md]
---

# Environment loading coordination

Current state: completed. The user accepted explicit environment files plus overwrite control and asked for rapid implementation. [Proposal](../../.docs/proposals/environment-loading.md) owns selected semantics; [Delivery](delivery.md) owns the completed outcome; Tasks retain the actual source/execution records.

## Bindings

Paseo self host `srv_yDbm9zuqIMiM`, existing local workspace `wks_35140a9c49235329`. The discovered `default` profile selects codex/gpt-6.1-sol with full-access/max. Actual worker IDs are recorded after creation.

| Responsibility | Binding | Current work/resource authority |
| --- | --- | --- |
| Native | `f747da8b-9eb8-485a-9ce1-2390457b11ae` on `srv_yDbm9zuqIMiM` (completed) | tasks/native.md; source/model/usage/cases only, no executable checks. |
| Acceptance | `25c5003e-6da1-43a6-8b68-0027bdf29109` on `srv_yDbm9zuqIMiM` (completed) | tasks/acceptance.md; independent fixture/evidence/lock/execution/package only. |

Both creation receipts report running in the selected workspace/full-access mode; provider/model/max were materialized from the current default profile. Initial assignment acknowledgements remain independently observable worker messages, not inferred completion.

Both actual owners completed their assignments. [Canonical evidence](evidence.md) binds the source, final package, bounded outcomes, cleanup and protected-resource limits; [ENV-01–ENV-12 and INS-01](../../tests/environment_loading/README.md) supply the consumer-owned outcome expectations. Source support, independent execution and controller promotion are recorded separately so preparation and acceptance remain distinguishable.

## Coherent native release received

Native released `2026-11-10-native-1` directly to acceptance and completed its original Task for source handoff. [Manifest](native-release.json) SHA-256 `10427940ca2bdfaf667e10eabea24ed06014bb56388a8ad80a7951f5f491d4bf` binds 30 source/case/context paths, 20 new authored cases, the strengthened workbench case, exact filters and the minimal pinned `dotenvy = "=0.15.7"` selection. All native writes are released; acceptance owns lock resolution/fixed-input format/compile/outcomes/package. Existing binary and user runtime have not been changed. Concrete defects and final current-model support receiving return to this same native owner.

[Environment design/usage](../../.docs/architecture/environment.md) owns the refined selector-scope rule: freeze list/overwrite from the pre-dotenv caller/control scope, then allow external file data to choose final scope without a selector reread; TOML-selected files may not declare `AGENTX_DIR`. Both owners resolved this and the legacy fixture's variable naming before execution, without a new user decision or task. Native’s completion is not a compiler/test/support claim; actual attempts belong only to [canonical evidence](evidence.md).

## First fixed-candidate outcome and returned defect

Acceptance's packaged r1 candidate `d4f71209…` passed 52 scoped native cases and package creation, then passed ENV-01–08 and INS-01. ENV-09 found that a root `--help` with an absent selected scope still reached the automatic update notice path; `get_update_notice` caused the existing update-check/flag side effect and created the scope. The help command itself returned zero and did not load dotenv, but the overall passive metadata boundary was violated. Acceptance cleaned the disposable attempt and returned the exact receipt and router region to Native. Only the affected lazy update boundary and ENV-09 path should be rechecked; prior passing cases remain valid. No project binary, daemon, workbench or installed resource was changed.

Native has now released correction manifest [`native-correction-env09.json`](native-correction-env09.json), SHA-256 `e81e576137e45f5f0ea8de5eb5f2326bb634ca13791035180edcb006682628b2`. It changes only the router gate plus two callback-effect cases and named documentation. The corrected gate excludes help/version/local environment metadata before update cache/worker access while preserving ordinary notices and explicit update behavior. Acceptance owns the affected recheck and derived package; no Cargo.lock or protected runtime change is involved.

## Source and artifact flow

Native publishes declarations/design and then exact coherent source/case release directly to acceptance. Acceptance freezes actual compiler inputs, runs only scoped relevant checks, preserves failures and requests causal corrections. No broad baseline suite or repeated passed check without a changed input. One sole executable owner avoids compile races and duplicate work. Existing accepted source and large dirty tree are retained; there is no Git reset, stash or automatic commit.

The promoted project `.bin/agentx` is `agentx 0.7.31`, 25,586,200 bytes, SHA-256 `34d9f0e54e6b9f83af5ba7095abad6f85777695ae464b539b42e95c5d5a38f37`. The project daemon is PID `177054` in scope `/home/long/works/longwork/agentx/.agentx`; the collaboration workbench is PID `177063` on port `18947`, with HTTP 200 HTML and connection checks. Exact promotion and runtime identity are in [/tmp/agentx-environment-loading-live-cutover-20261110/controller-cutover.json](/tmp/agentx-environment-loading-live-cutover-20261110/controller-cutover.json). The old separate demo, installed defaults and unrelated user services remain outside this work.

## Formal completion

The final artifact was promoted atomically and the current project runtime was cut over to matching bytes. The accepted Linux/local finite support and explicit limits are recorded in [Delivery](delivery.md#accepted-local-outcome-and-project-handoff) and [canonical evidence](evidence.md#controller-promotion-and-live-runtime-handoff). Both Paseo owners have completed and released their resources; no executable or runtime work remains in this Delivery.
