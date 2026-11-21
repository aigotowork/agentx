---
schema: task
status: completed
description: Independently verify the integrated roles/workspace behavior in disposable native scopes.
---

Read [team contracts](../AGENTS.md). Use exact released source/artifact, minimal fixtures and real CLI/native APIs. Cover shadowing, prompt/profile composition, stable membership/global visibility, parent default routing, concurrency and failure without mutating live scopes. Report actual evidence and limitations; do not infer support from source or mocks alone.

Current outcome: independently accepted for the documented Linux local native CLI/fake-Provider scope. Nine case groups pass through explicitly recorded staged artifact coverage; see [final acceptance](#final-independent-acceptance) and [aggregate evidence](../evidence/acceptance.json). Earlier failures and fixture corrections below are retained history.

## Independent acceptance plan

Owner: dodo. Controller: bime. The controller confirmed `.bin/agentx-roles` as the dedicated artifact; execution and Cargo remain deferred until release. No production edits, subagents, live scopes, installed programs or live daemon changes are authorized. This task owns the acceptance runner and retained results.

The executable runner is [tests/roles_workspaces.py](../../../tests/roles_workspaces.py). It uses fresh temporary scopes, an isolated home/environment, a local fake Codex executable, real CLI operations and read-only SQLite inspection of their durable effects. The fake Provider establishes only local process/session facts; it cannot establish real model or provider compatibility.

| Case | Starting state and action | Required observation / invariant |
| --- | --- | --- |
| Discovery | Global, parent and child definitions; empty child marker; explicit selectors | Whole-definition shadowing, canonical origin, relative prompt file, correct local/global selection; no daemon or Provider for inspection |
| Invalid definition | Malformed selected file, missing profile, duplicate defaults | Source diagnostic, no fallback and no Provider execution |
| Composition | Role plus ambient profile, explicit override, separate cwd/membership | Declared and selected profiles remain distinguishable; builtin, role and explicit instruction additions survive |
| Unsupported paths | New role/workspace selectors with remote launch or shared Codex stdio | Explicit rejection before Provider or remote dispatch effects |
| Membership | Global-role A member, roleless B member, explicit global member; altered caller cwd/environment | Stable persisted membership, local+global visibility, explicit all; role origin does not grant global membership |
| Delegation | Child shadows parent default name but has no default; two child sends | Exact parent definition and parent endpoint, requested child context retained in the event and delivered Hook header, unchanged message body, unique accepted messages |
| Concurrent creation | Two first sends to an unbound endpoint | One native endpoint/Provider instance, two durable messages with different event identities |
| Failure | Multiple compatible instances, changed endpoint role, invalid default | Actionable conflict, no arbitrary adoption or speculative duplicate |
| History | Launch role, edit/delete definition, stop and resume/fork | Adopted role/profile/membership survive; instructions remain frozen; fork gets new identity and no copied endpoint lease |

Native unit filters `config::roles` and `agents::workspaces` are supplemental evidence after controller scheduling. They cover transaction rollback, stale incarnations and retirement/reset semantics that a fake Provider cannot independently prove through ordinary CLI flows. Unexecuted cases and real-provider/remote limitations remain explicit in the final evidence.

Coordination update: bime owns Cargo, including resolver and membership unit filters, and the final package. Acceptance needs only the released executable and SHA. The controller reported 15 resolver/CLI checks passing before release; that report is not independent executable acceptance.

## Candidate acceptance — 2026-11-15

Executed the released `.bin/agentx-roles` (version 0.7.31, SHA256 `85d8632a3d284943842bd747dfa0f51043157f437e70bba31e0859326c7c4a4a`). Seven case groups passed; two substantive failures prevent acceptance. [Consolidated executable evidence](../evidence/acceptance.json) retains exact commands and links the separate attempts. No Cargo or production edits were made.

Passed: discovery/shadowing/literal prompts, invalid definition diagnostics, profile/cwd composition, remote/shared-stdio rejection, stable membership and global visibility, parent default delegation including unchanged event body plus requested-directory metadata and delivered Hook header, and ambiguous endpoint/role-drift rejection.

| Finding | Executed observation | Requirement and next owner |
| --- | --- | --- |
| Concurrent first sends lose one message | One sender exits 1: `No instances launched (0/1): conflict: workspace endpoint reservation conflict: already reserved by 'romi'`; its command never returns a durable acceptance | Both sends must use the winner and retain two distinct messages. Controller owns send orchestration/error handling. See `concurrent` in [runtime evidence](../evidence/acceptance-runtime.json). |
| Resume loses explicit fixed instruction addition | Initial launch used `--agentx-system-prompt ADOPTED_ADDITION`; resumed native `developer_instructions` retains `GLOBAL_ROLE` and original model but omits `ADOPTED_ADDITION` | Preserve the full adopted fixed instruction composition in history, not only the role text. Controller/launch owner must correct persistence. See `history` and the captured native argv in [runtime evidence](../evidence/acceptance-runtime.json). |

The history case did establish role/profile/workspace preservation across role-file deletion and profile editing, fresh fork identity, cleared fork endpoint flag, and retained stopped collaboration snapshot before failing the final explicit-addition assertion. This partial evidence does not mark the whole case passed.

Fixture corrections are separate from product findings: initial fake Codex lacked the existing `initialize`/`hooks/list` preflight; the runner now implements that JSON-RPC handshake. External sender is checked through native event `from=bigboss` and `sender_kind=external`, not the internal `ext_bigboss` storage key; plain explicit sends supply existing `--from` identity semantics. [Initial fixture-limited attempt](../evidence/acceptance-fixture-preflight.json) and [delegation fixture correction](../evidence/acceptance-delegation-fixture.json) remain retained. Only affected groups were rerun.

Next: await corrected dedicated candidate and controller release, rerun the two failed groups plus any specifically affected checks. Real Provider/model execution, cross-device role support, and mutation of existing user services remain outside this acceptance scope.

Corrected-candidate plan: bime reported source fixes for duplicate reservation errors and frozen `instruction_additions`; execution still awaits the new exact SHA. Rerun `concurrent`, `history`, `composition` and `membership` only. The history check now verifies the original addition on initial launch, fork and resume and checks that the fork's native instructions carry its own name. Preserve all original-candidate evidence and report the new artifact separately.

## Corrected candidate — 2026-11-15

Executed version 0.7.31, SHA256 `f76a1a9ec6c4b09cdba8ae7a295c8a6764d72dbc40ab85d1c0b62bafd04147ed`. [Four-group correction report](../evidence/acceptance-corrected.json): history, composition and membership passed; concurrent first sends still failed. [Consolidated acceptance](../evidence/acceptance.json) now names coverage per artifact and preserves the [original candidate summary](../evidence/acceptance-original.json). Latest combined case coverage is eight passed and one failed; this does not claim that every case was rerun on the corrected artifact.

The fixed instruction addition now survives both fork and resume after role-file deletion and profile editing. Native instructions identify each fork by its own new name, while workspace/role and original profile remain adopted history. The original fixed-addition failure is resolved in this artifact.

Remaining correctness gap: concurrent sender loses the message while the exact endpoint reservation is startup-pending. Its error is `workspace endpoint @reko is inactive (new); recover or explicitly retire it before sending`. Durable evidence contains one bound endpoint, one Provider, and only `race-0`; `race-1` has no acceptance or event. The winner later reaches `listening`. Source correlation: [usable_recipient](../../../src/cli/commands/send/directory.rs) rejects all `inactive` records, including the initial reserved `inactive/new` state. This state is distinct from failed/uncertain startup. Controller was notified immediately; next action is its correction and another targeted concurrency release.

Next concurrency attempt uses a fixture startup gate: both CLI calls begin together, and fake Codex waits before `start`/SessionStart until acceptance observes at least one durable message with no registered session. The gate then opens and the test verifies one Provider/endpoint and two distinct durable events. The captured `pending_startup_messages` and `pending_startup_instances` prove the startup window was exercised, rather than relying on scheduling luck. No passed unaffected cases need rerunning.

## Final independent acceptance

Accepted on 2026-11-15 after the final concurrency correction. Exact final artifact: `.bin/agentx-roles`, version 0.7.31, SHA256 `6b67fb228008699cf0ac8cd1bc705ab192e9777311f81bcd43eda9c2c6704418`. [Final gated concurrency report](../evidence/acceptance-final-concurrent.json) passes and closes the remaining correctness gap. [Aggregate evidence](../evidence/acceptance.json) records all nine latest case outcomes, their exact artifact hashes and original attempts; it does not imply all groups were executed on one artifact.

Decisive concurrency facts: participant `garo` remained `inactive/new` with `session_id=null` behind the fixture startup gate while event #2 durably accepted `race-1`. After releasing startup, event #5 durably accepted `race-0`. Both receipts address the same participant, with one native endpoint and one fake Provider process. Thus the reservation/startup race was exercised directly; no second recipient or replay was needed.

| Artifact | Independent coverage and retained disposition |
| --- | --- |
| `85d8632a…c7c4a4a` | Discovery, invalid definitions, composition, unsupported rejection, membership, delegation and conflicts passed; original concurrency/history failures retained. |
| `f76a1a9e…04147ed` | History including role deletion, profile edits, frozen additions and fork identity passed; composition/membership passed after instruction snapshot changes. Remaining pending-startup concurrency failure retained. |
| `6b67fb22…704418` | Strengthened gated concurrency passed after the controller-reported sole production delta narrowed `usable_recipient` to admit native launching placeholders. Other unaffected passing cases were not rerun. |

No substantive correctness gaps remain within this acceptance scope. Evidence establishes filesystem role resolution, local CLI/native launch and durable mailbox behavior using an isolated fake Codex. It does not establish real model/provider compatibility, remote role execution or unsupported shared app-server workspace support. Controller-owned unit/regression results and build-input receipts remain separate evidence. No production code, installed/live binary, user scope, live daemon or service was changed by acceptance; no Cargo or subagents were used. Fixtures and their scoped processes were cleaned up. Result returned to bime through AgentX.
