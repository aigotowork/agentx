---
schema: document
status: active
owner: startup-acceptance
last_updated: 2026-11-10
description: Canonical fixed-input startup usability attempts, artifacts and resource cleanup.
links: [./startup-usability.md, ./tasks/startup-acceptance.md, ../../tests/startup_usability/README.md]
---

# Independent startup usability evidence

Current state: completed independent Linux local acceptance. [Final result and handoff](#completed-bounded-result-and-handoff) owns the supported outcome, retained candidate and cleanup. Earlier checkpoints below preserve the actual preparation and failed fixture attempts; they are historical, not pending execution gates.

The controller subsequently completed [same-byte repository promotion and exact user-runtime cutover](#controller-promotion-and-user-runtime-handoff). The accepted candidate now occupies project `.bin/agentx`; project daemon/workbench intentionally remain running for the user. This is separate from the independently cleaned disposable acceptance resources.

## Authority and fixed preservation

The [original Task](tasks/startup-acceptance.md) owns execution and fixture scope; [Delivery](startup-usability.md) owns outcome requirements and peer bindings. Native implementation remains owned by its original specialist. This record is the only startup attempt/artifact/cleanup ledger; raw receipts are linked rather than copied into competing summaries.

Evidence root: `/tmp/agentx-startup-acceptance-_5te_x9y` (0700). Independent [preservation receipt](/tmp/agentx-startup-acceptance-_5te_x9y/preservation.json) records `/home/long/works/longwork/agentx/.bin/agentx` copied to `preserved-r8-agentx` (0555), 25,449,688 bytes, SHA-256 `9697154848fa0d258dc61c9c66ed321b6d36459ac4394cf1a82712fc925a9479`. Source and copied bytes match the prior accepted [r8 receipt](/tmp/agentx-workbench-acceptance-lh863ck1/r8-final-artifact.json). Preservation did not execute the image.

No repository `.agentx` access, installed/PATH/default image write, protected demo18731 operation, real Provider/model session or nested delegation is authorized here. Only exact owned temporary processes/scopes will be operated. Candidate packaging occurs in a fixed temporary snapshot, not the live checkout `.bin`; controller owns any later promotion/cutover.

## Scenario model and execution gate

[Fixture cases](../../tests/startup_usability/README.md) bind cold acquisition, passive identity-free diagnosis, genuine incompatible-build recovery/data retention, scope isolation, exact-peer/stop refusal and short embedded workbench defaults to disposable execution resources. Native authored cases supplement the real product outcome; unchanged prior workbench business/browser evidence is reused.

Native callable skeleton received: `daemon status|restart [--scope PATH]`, early identity-free dispatch; scope defaults follow normal environment; workbench project defaults to cwd and local startup acquires. Compatible stop uses exact native Shutdown. Legacy Linux incompatible stop requires authenticated endpoint PID/incarnation, exact serve/scope argv, loaded executable evidence, PID-owned lifetime-lock inode and pidfd SIGTERM. Refusal/timeout cannot spawn a replacement. Other-platform incompatible recovery remains unexecuted and source-only.

At preparation, execution awaited coherent native source/case release with exact hashes and no formatter/compiler/Cargo/Node/product/browser check had run. `just --list` was used only to discover existing recipes. The selected execution path reused `tests/collaboration_workbench/execute_check.py`, exact prior `tooling/tooling.json` (Node22.23.3/npm10.9.9), existing `node_modules` and shared `target`; no install or dependency selection was needed.

## Preparation findings

The prior tooling helper is source-owned at `tests/collaboration_workbench/execute_check.py`; the evidence root runner alone does not select Node. The startup evidence root will link the existing tooling directory so the helper consumes the exact prior toolchain record. A missing presumed `tests/domain_daemon/README.md` was a discovery-only path miss; actual fixture/model ownership is under `src/daemon` and this Task. These findings caused no executable attempt or product effect.

## Owned resources

At preparation, only the retained evidence root and independent r8 copy existed and no runtime process, browser, listening port or managed disposable scope had started. The [completed resource disposition](#completed-bounded-result-and-handoff) supersedes that checkpoint.

## Released fixed source and selected native result

Native coherent release r1 was received directly; [release verification](/tmp/agentx-startup-acceptance-_5te_x9y/r1-release-verification.json) matched all 18 source/case/docs hashes. The [authored snapshot](/tmp/agentx-startup-acceptance-_5te_x9y/r1-authored-source-receipt.json) preserved 540 selected inputs with no capture race. Acceptance applied scoped Rust formatting only to the 12 released Rust files; [format receipt](/tmp/agentx-startup-acceptance-_5te_x9y/r1-scoped-format-receipt.json) and [seven-file formatting delta](/tmp/agentx-startup-acceptance-_5te_x9y/r1-format-delta.json) retain exact results. The [formatted candidate](/tmp/agentx-startup-acceptance-_5te_x9y/r1-formatted-source-receipt.json) is the immutable compiler input; existing repository `target`/`node_modules` are linked as caches only.

`r1-selected-native` passed: 17 selected authored cases, zero failures, 3,144 unrelated cases filtered out, 132.611 seconds including compilation and fresh embedded browser builds. [Exact command and receipt](/tmp/agentx-startup-acceptance-_5te_x9y/r1-selected-native-receipt.json) used the prior task-local Node22 helper and existing locked dependencies. Filters are the four original-owner releases; no broad suite or Provider ran. Native legacy evidence/refusal, passive status, startup serialization, drain timeout, parser/mismatch recovery, workbench defaults/explicit mode and closed HTTP cases pass within this Linux fixed-input execution.

At this checkpoint `r1-package` started `just package` in the immutable temporary candidate through the same exact tooling helper. Live checkout `.bin` was not a package output target. The actual completed package/outcome and resource release follow below.

## Packaged image and first actual outcome attempt

`r1-package` passed in 589.374 seconds and reports `agentx 0.7.31`. [Artifact receipt](/tmp/agentx-startup-acceptance-_5te_x9y/r1-package-artifact.json): `/tmp/agentx-startup-acceptance-_5te_x9y/r1-formatted-source/.bin/agentx`, 25,487,320 bytes, SHA-256 `9966776db839d9c667f9c7ddd68df19463ace45a95ebceb2ab125088943524f4`. All 540 frozen inputs were unchanged after package; repository `.bin` still matched preserved r8. No second package/build is needed for fixture-only corrections.

The first [actual product attempt](/tmp/agentx-startup-acceptance-_5te_x9y/r1-product-outcome-receipt.json) used exact [frozen harness r2](/tmp/agentx-startup-acceptance-_5te_x9y/outcome-r2-source.json) and passed SU1–SU5 before failing its first SU6 HTTP assertion at 2.983 seconds. [Exact outcome/resource ledger](/tmp/agentx-startup-acceptance-_5te_x9y/product-r1/resources.json) retains successful missing/status/cold cases, actual preserved-r8 incompatible peer and actionable diagnosis, explicit selected-scope replacement, unchanged durable manual-participant fields and owner.lock inode, untouched separate-scope incarnation, two serialized canonical/alias restarts, second-serve exclusion, held-lock/impostor refusal without spawn-log creation, and actual stopped-Hello refusal with the original peer retained.

SU6 failure is fixture-owned: the helper reused the printed base URL `http://127.0.0.1:46811/` as the Origin header, whereas HTTP Origin excludes that trailing path slash. Existing admission rejects it. The first helper did not persist the HTTP body/status before asserting, so that exact response is unavailable and is not invented here. The printed URL and fixture command are retained. All first-attempt resources were cleaned despite the assertion: impostor, host and three remaining native daemons stopped via exact pidfds, port46811 closed, disposable root removed; no browser had started.

The corrected [frozen harness r3](/tmp/agentx-startup-acceptance-_5te_x9y/outcome-r3-source.json) normalizes Origin, retains every HTTP response and selects independent SU6–SU7 continuation only. Native/product source and packaged bytes are unchanged; successful SU1–SU5 and 17 native cases are reused. No implementation defect or repeat native check has been selected.

## Workbench continuation and retained fixture corrections

`r1-workbench-outcome` used harness r3 and passed the new-host binding, embedded HTML/JS/CSS delivery and explicit default-readonly HTTP403 check. [Actual HTTP attempts](/tmp/agentx-startup-acceptance-_5te_x9y/workbench-r1/http-attempts.json) retain those exact responses. Its browser check timed out at 36.149 seconds because the new fixture used exact text `只读`, while the existing StatusLabel renders a supporting aria-hidden `!` glyph in that element. The existing accepted `browser_flow.mjs` already owns a correct `.status-label`/hasText locator; this was a bounded fixture-selector mistake, not a product rendering/source defect. [Failure receipt](/tmp/agentx-startup-acceptance-_5te_x9y/r1-workbench-outcome-receipt.json), [browser error](/tmp/agentx-startup-acceptance-_5te_x9y/workbench-r1/browser-command.json), source and browser PID records remain intact. That browser closed, both owned product processes stopped, port42425 closed and its root was removed. The first browser collector had no failed-state image; that limitation is retained rather than fabricated.

[Frozen harness r4](/tmp/agentx-startup-acceptance-_5te_x9y/outcome-r4-source.json) reuses that existing locator, adds failed-page capture, and consumes prior same-image successful HTTP receipts. [Final Web continuation](/tmp/agentx-startup-acceptance-_5te_x9y/r1-workbench-continuation-receipt.json) passed in 3.000 seconds: SU6 real browser and SU7 explicit scope/project/collaboration overrides plus HTTP scope-selection rejection. The [final outcome ledger](/tmp/agentx-startup-acceptance-_5te_x9y/workbench-r2/resources.json) binds actual descriptors, exact processes and cleanup; SU1–SU5, native checks and package were not rerun. A new host descriptor was read solely to bind the browser to its fresh owned project/scope before observation.

User-intent observation at 1280×800: [retained page](/tmp/agentx-startup-acceptance-_5te_x9y/workbench-r2/browser/default-page/viewport.png) shows the current `project` header, visible `只读` state, left resource/Agent/result/check navigation, and the empty work table with implementation/check/result/action columns in one workspace. This serves the selected short-startup question without fake tasks. [DOM/geometry](/tmp/agentx-startup-acceptance-_5te_x9y/workbench-r2/browser/default-page/dom.json), [semantic tree](/tmp/agentx-startup-acceptance-_5te_x9y/workbench-r2/browser/default-page/semantic.yml) and [browser result](/tmp/agentx-startup-acceptance-_5te_x9y/workbench-r2/browser/result.json) retain correlated actual state, no page errors and closure. Empty-project startup does not re-establish the earlier full business/keyboard/touch journey; that unchanged accepted evidence remains reused.

## Completed bounded result and handoff

| Outcome | Actual evidence |
| --- | --- |
| SU1 passive identity-free status and override | `product-r1` missing scope/explicit override receipts; no scope creation. |
| SU2 ordinary missing-daemon acquisition | `product-r1` cold `list --json` and kernel-bound candidate peer/status; no daemon-start ritual. |
| SU3 real different-build failure/diagnosis | Genuine preserved r8 owner; candidate ordinary call rejects exact compatibility before mutation and prints actionable scoped recovery; old unavailable facts stay unavailable. |
| SU4 explicit selected recovery/data retention | Old exact peer exits; candidate owns same retained lock inode; manual participant name/agent_id/created_at/tool/directory survive; separate scope PID/incarnation unchanged. |
| SU5 singleton and exact refusal | Second serve refused, concurrent canonical/alias restarts serialize, held lock and impostor fail without spawn-log creation, paused actual owner gets no replacement after bounded Hello failure. |
| SU6 short embedded entry | `workbench-r1` HTTP receipts plus `workbench-r2` real browser: environment scope, cwd project, local cold acquisition, loaded embedded assets, default readonly mode. |
| SU7 explicit finite mode/scope | `workbench-r2` explicit project/scope beat environment/cwd; collaboration true; HTTP cannot select/create the unselected scope. |

[Final source/resource audit](/tmp/agentx-startup-acceptance-_5te_x9y/final-resource-source-audit.json) examined only recorded identities: 21 product/browser/impostor process records have no same executing incarnation, and both intermediate serialized-restart PIDs have no executing image. All three attempt roots were removed. Ports46811,42425,41403,46681 were confirmed closed by their respective owned cleanup receipts; no daemon, browser, host, fixture child or data root remains active. Existing evidence/snapshots/tooling are intentionally retained. The impostor signal marker is written by its explicit pidfd cleanup after the no-signal assertion; it is not a signal from rejected restart.

The same audit matched all 469 selected shared compiler inputs to the fixed compiled source and matched the final candidate SHA-256 to its package receipt. Repository `.bin` still matches preserved r8. Retained candidate `/tmp/agentx-startup-acceptance-_5te_x9y/r1-formatted-source/.bin/agentx` (SHA-256 `9966776db839d9c667f9c7ddd68df19463ace45a95ebceb2ab125088943524f4`) and the immutable source/package receipts are released to the controller for its authorized promotion/cutover. No automatic promotion, user `.agentx` operation, installed/default change, protected demo18731 action or real Provider/model session occurred here.

The bounded Linux local startup outcome is independently accepted: 17 selected authored cases pass, actual SU1–SU7 pass across retained attempts, package passes and all owned runtime resources are closed. Both observed failures were acceptance fixture mistakes and required no product patch or new build. Linux legacy stop additionally depends on accessible `/proc` lock/process evidence and pidfd; absent authority must refuse. Other-platform legacy stop/runtime execution remains unsupported/unverified as documented by native, and no universal installation or Provider support is inferred. The original acceptance Task and all owned source/evidence claims are complete and released; controller alone owns final Delivery acceptance and live development/demo promotion.

## Controller promotion and user-runtime handoff

Controller-owned `/tmp/agentx-startup-live-cutover-gxqa9j8n/{before.json,cutover.json,final.json}` records the authorized project cutover after independent release. Candidate SHA/size matched the immutable package receipt; the old r8 image matched its independently retained `preserved-r8-agentx`. The controller authenticated the selected-scope kernel peer and exact old project daemon argv/PID/start tick/image inode, and verified the recorded project workbench metadata. The old host received pidfd SIGINT, the candidate was atomically promoted to repository `.bin/agentx`, and the old daemon received pidfd SIGTERM; both exited gracefully without a force kill or lock/data removal. These direct controller signals implemented the already-authorized live cutover; the new CLI restart method's behavior is independently established by SU4/SU5, not claimed as the live invocation method.

Ordinary new `status --name vivo` successfully cold-acquired the matching daemon. New PID3694922 loaded exactly the accepted candidate bytes; same-byte repository SHA is `9966776db839d9c667f9c7ddd68df19463ace45a95ebceb2ab125088943524f4`. Native list snapshots before/after retained all three prior participant rows' available name/agent_id/created_at/tool/directory identity fields; the lifetime-lock inode remained unchanged. Dynamic activity fields were not compared as immutable identity.

New workbench PID3694937/start929508937 runs from the project with `workbench --collaboration --host 0.0.0.0 --port 18947 --name vivo`, omitting explicit scope/project overrides. HTTP HTML200 and same-origin POST connection200 confirmed `available:true`, `collaboration:true`, project `/home/long/works/longwork/agentx` and scope `/home/long/works/longwork/agentx/.agentx`. Native descriptor and exact snapshots are retained beside the cutover receipt; `.agentx/.tmp/workbench-launch.json` records the running user host. The user URL is `http://192.168.2.178:18947/`.

Protected demo PIDs1497466/1497500 retain original executable/start ticks. The installed `/home/long/.local/bin/agentx` SHA remained unchanged. No default shell/PATH replacement, protected demo mutation, new Provider launch or source/compiler operation was performed during cutover. The project daemon/workbench remain explicitly handed to the user; all disposable acceptance roots/processes/ports were already cleaned. The [Delivery](startup-usability.md#accepted-local-outcome-and-project-handoff) is formally completed within this recorded scope.
