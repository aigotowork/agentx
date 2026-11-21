---
schema: delivery
description: Recover ordinary development startup and make daemon compatibility diagnosis and workbench entry usable.
status: completed
stages:
  implementation:
    status: completed
    tasks: [./tasks/startup-native.md]
  outcome:
    status: completed
    tasks: [./tasks/startup-acceptance.md]
    depends_on: [{stage: implementation, condition: coherent bounded source/case release}]
---

# Development startup usability

Current disposition: completed and accepted for the [bounded Linux result and live project handoff](#accepted-local-outcome-and-project-handoff). Both original Tasks are completed; the independently verified artifact is promoted to project `.bin/agentx`, and the project daemon/workbench now execute that image. Earlier sections retain the actual diagnosis and preparation chronology. All isolated acceptance resources are closed; the user-requested workbench remains intentionally running.

The [controller closure](controller.md) records successful archival of both completed follow-up specialists. Future changes require a new bounded assignment; this Delivery has no remaining execution work.

## User outcome and current diagnosis

The user runs this repository through `.bin/agentx` with `AGENTX_DIR=$PWD/.agentx`. Normal commands must automatically acquire a missing daemon. A build replacement must not leave users with an unexplained compatibility error and no supported recovery command. The local workbench must have a short discoverable startup path. This is a bounded follow-up to the [accepted daemon design](../../.docs/architecture/daemon.md) and [completed workbench slice](../collaboration-workbench/delivery.md); their fixed-input acceptance remains historical.

Observed 2026-11-10: the selected repository scope was owned by PID227549/start913323439, executing an unlinked old `.bin/agentx`. Its loaded native cohort was `5e62d8d5-4208-4ceb-bfe6-bbc11c068956`, while the current r8 binary's cohort is `2856ddab-037e-4f68-8206-7c2b8c6755ea`. The exact current binary reproduced the user's failure. The controller verified exact PID/incarnation/executable/scope, requested graceful SIGTERM through a pidfd, and observed exit. No managed data or lock file was removed. `/tmp/agentx-startup-recovery-3_lmx8pk/recovery.json` owns that recovery receipt. The following current client automatically started its own compatible daemon; `start --name vivo` and `status --name vivo` succeeded. Installed binaries and the separate old LAN demo were untouched. Login-tool PATH ordering alone does not establish the user's interactive-shell resolution; recovery used absolute project paths.

## Bounded contract and proportionality

- Preserve ordinary missing-daemon auto-start and exact native compatibility/generation fences. Never retry a possibly applied operation or open managed SQLite in a client.
- Provide a documented native `daemon status` and explicit `daemon restart` recovery usable without participant identity and despite a build mismatch. Status must expose selected scope, actual daemon state and useful compatibility information. Restart must target only the exact authenticated/owned selected-scope peer, drain gracefully, respect bounded deadlines and the singleton, and report a substantive failure without deleting data/locks or killing unrelated processes.
- Report compatibility mismatch with an actionable recovery command and the relevant scope/client/daemon information available from trustworthy evidence. Do not invent missing version/build facts.
- Make `workbench` derive scope from the normal environment and project from the invoking cwd when flags are omitted, retaining explicit overrides. Local workbench startup may acquire a missing daemon; HTTP requests cannot select/start another scope. Keep `--collaboration` as the explicit finite-write choice and passive observe semantics separate.
- Do not automatically kill every incompatible live daemon. Explicit restart is the bounded operator recovery; successful cold ordinary startup still needs no daemon-start ritual. No generic process supervisor, updater, protocol migration, fake demo data or new authentication system.
- A usable native design/skeleton precedes behavior; the source owner may refine this bounded command grammar and identity evidence where concrete platform contracts require it. Only unresolved correctness or user-outcome issues need controller attention.

## Responsibilities and verification

The controller owns this Delivery, shared navigation and the actual user environment recovery/demo listener. A fresh native specialist owns lifecycle/CLI behavior and authored cases; an independent acceptance specialist alone owns formatter/compiler/build/test/package/fixture execution. These two roles are justified by native lifecycle/CLI context and independent process-safety verification, while the controller continues non-overlapping user recovery. No archived team is revived, no nested delegation or extra review stage.

| Responsibility | Binding on `srv_yDbm9zuqIMiM` / `wks_35140a9c49235329` |
| --- | --- |
| Native implementation and authored cases | `139a1946-7a3a-4908-8fb7-296099fd697a`; original [native Task](tasks/startup-native.md). |
| Independent fixtures, execution, package and evidence | `f28a4bc5-249c-494f-9d6b-c0f63115c383`; original [acceptance Task](tasks/startup-acceptance.md). |

Both assignments were created with the discovered `default` Paseo profile, `codex/gpt-6.1-sol`, full-access/max, and notify-on-finish. Send actual contracts/releases/defects directly between these bindings; original Tasks own state. Do not poll or restore archived workers. Final candidate promotion and exact user daemon/workbench cutover remain controller-owned so the delivered development commands and running demo use one coherent image.

Verify in disposable scopes: cold auto-start, explicit-scope isolation, readable status with no daemon and incompatible daemon, genuine different-build restart preserving data and singleton, exact peer/stop failure refusal, and workbench defaults plus explicit overrides with finite mode. Reuse unaffected native/workbench evidence; do not rerun real model work, old broad suites or install dependencies again. Use existing required task-local Node22 tooling for affected builds. Preserve current r8 `.bin` before any candidate promotion; installed/PATH binaries and protected demo18731 are outside this follow-up. Source authors run no Cargo/Node/browser/formatter/test/product checks. Acceptance releases exact artifact/source/cleanup facts; controller manages any later explicit development daemon/listener cutover.

The independent acceptance owner has completed preparation: [canonical startup evidence](startup-evidence.md) records the preserved r8 identity, reused exact Node22 tooling and absence of fixture runtime resources. [SU1–SU7](../../tests/startup_usability/README.md) bind the real cold-start/mismatch/restart/scope/default-workbench outcomes. Native's actual lifecycle contract and callable skeleton were consumed directly; coherent source/case release remains the prerequisite for compilation and product execution. The acceptance response ending is a preparation handoff, not Task or Delivery completion; callbacks, not polling, resume execution.

The native owner has now completed and released [coherent r1 source/case/docs](tasks/startup-native.md#coherent-source-release-r1-2026-11-10) directly to independent acceptance: eighteen exact source/context identities, four focused case filters, actor-free lifecycle grammar, bounded verified-peer restart and environment/cwd workbench defaults. All authored writes/resources are released; implementation stage is completed for source receiving. Independent execution and exact package/support remain unverified until recorded in [startup evidence](startup-evidence.md). The currently running user image is still preserved r8 and does not yet contain these new commands/defaults; promotion and live project cutover remain controller-owned.

## Acceptance

The user's development commands work on the selected scope without manual daemon startup; a stale-build failure has useful diagnosis and a supported restart path; the short workbench command starts a usable embedded page. The native method/ownership guide and everyday usage reflect verified support. Every execution resource is either explicitly handed to the user or cleaned up. This Delivery closes on those concrete outcomes, not a count of checks.

## Current user runtime handoff

The controller recovered only the project `.agentx`, using the absolute current `.bin/agentx`; ordinary acquisition automatically spawned the compatible daemon. `start --name vivo` and `status --name vivo` now succeed, with the selected project data retained. A user-requested collaborative workbench is running at `http://192.168.2.178:18947/`, current host PID3647661/start929249802. `.agentx/.tmp/workbench-launch.json` records its exact argv/executable/scope/log for any later controller-owned cutover. HTML returned200 and the actual same-origin POST connection descriptor returned200 with `available:true`, `collaboration:true` and the correct project/scope. Earlier unframed GET descriptor attempts returned403 then405, consistent with the strict Origin/POST boundary; they were controller smoke-request mistakes, not product defects.

This runtime is handed to the user, not an acceptance fixture to delete. Independent acceptance must keep its processes/scopes/ports separate and preserve the current r8 `.bin` as the genuine prior-build comparison. Source completion alone does not authorize replacing a running user image; final controller cutover must restart only this exact project daemon/host as needed, never the installed binary or protected18731 observation demo.

## Accepted local outcome and project handoff

The controller accepts this Delivery on 2026-11-10 using [independent SU1–SU7 outcome/cleanup evidence](startup-evidence.md#completed-bounded-result-and-handoff) and the subsequent [exact project promotion/cutover](startup-evidence.md#controller-promotion-and-user-runtime-handoff). Seventeen selected authored cases, genuine different-build recovery, retained data/lock identity, unrelated-scope isolation, serialized singleton/refusal behavior and short embedded-workbench startup passed on the fixed packaged candidate. No new implementation defect or second build was required; failed fixture attempts remain preserved. Unchanged full workbench business and Provider evidence remains at its prior accepted scope.

The exact verified `agentx 0.7.31` image (25,487,320 bytes, SHA-256 `9966776db839d9c667f9c7ddd68df19463ace45a95ebceb2ab125088943524f4`) now occupies repository `.bin/agentx`. The controller stopped only its recorded project host/daemon gracefully through pidfds, atomically promoted the preserved candidate bytes, and ordinary `status --name vivo` automatically acquired the matching daemon. The three pre-cutover participant identities and owner.lock inode were retained. No managed data or lifetime lock was removed.

The current collaborative workbench is `http://192.168.2.178:18947/`, using the simplified command's environment scope and cwd project. Its HTML and native connection returned200 with the correct `.agentx`/project, `available:true` and `collaboration:true`. `.agentx/.tmp/workbench-launch.json` owns exact running host metadata. This is an intentional user runtime, not an uncleaned fixture. Protected demo18731 retained its original PID/executable/start ticks; installed/default/PATH binaries were not replaced.

Normal usage needs no manual daemon-start command. The documented native status/restart path provides explicit old-build recovery while preserving exact compatibility and uncertain-effect semantics. Linux legacy stop requires accessible authenticated `/proc`/owner-lock evidence and pidfd; unsupported authority or other-platform legacy-stop cases refuse or remain unverified. This result does not claim universal Provider/hooks/OS support, publish a release or modify user shell resolution. Use `./.bin/agentx` explicitly in this development checkout when PATH resolution is uncertain.


## Open diagnostic: gala launch and operator fencing (2026-11-15)

User reported that `agentx kill gala` rejects with `stale participant, execution, attachment or database generation`. Read-only inspection confirmed the live Codex child PID1813644 matches the recorded Linux process incarnation; launch event17766 and `.agentx/.tmp/logs/background_1794672489_5092.log` show it waiting at `TERM=dumb` / `Continue anyway? [y/N]:` before a Provider session exists. `agentx term gala --clean` also rejects with the same stale error.

The control failure is explained by float identity loss: stored `created_at=1794672489.9629567` has bits4745314209997103381, while a serialize/deserialize round trip using the current project serde_json rlib produces bits4745314209997103382. The CLI [terminal path](../../src/cli/commands/term.rs) derives ParticipantRef from the transported record timestamp, while [daemon terminal admission](../../src/service/queries_store.rs) and [kill admission](../../src/agents/lifecycle_store.rs) compare its bits to storage. [Cargo.toml](../../Cargo.toml) currently enables `raw_value` without `float_roundtrip`. Future correction should preserve exact identity across transport (evaluate exact native ParticipantRef consumption and/or round-trip-safe float decoding), and independently normalize unusable TERM for managed Codex PTYs. No source, database, process, daemon or installed binary was changed during diagnosis. The standalone diagnostic was compiled in a temporary directory and removed. This is an open defect, not a revision of earlier acceptance evidence.


### Authorized bounded correction

The user authorized fixing gala and the recurring dumb-terminal launch prompt. Keep existing participant and database identities unchanged: enable serde_json `float_roundtrip` so transported timestamp-derived fences retain exact bits; do not weaken identity comparisons or rewrite timestamps. Normalize TERM only at the managed PTY child boundary, shared by Unix and Windows: last explicit override precedes inherited TERM, valid values survive, and absent/empty/dumb becomes xterm-256color. Regression cases cover the observed timestamp plus neighbors and TERM precedence. Validation and package/live recovery are pending; existing user daemons and unrelated agents remain running.


### Focused validation and live cleanup

The three focused checks passed: `just test --bin agentx participant_timestamp_roundtrip_preserves_incarnation`, `just test --bin agentx managed_pty_term_defaults_and_preserves_valid_overrides`, and `just test --test cli_smoke pty_defaults_unusable_term_and_preserves_valid_term`. The CLI case executes real Unix PTY children for absent, empty, dumb and valid TERM values. Windows shares the normalization function but was not executed on this Linux host. An initial invocation redundantly supplied `--locked` to the already-locked recipe and was rejected before test execution; corrected invocations above passed.

The original launch owner had already terminated only the verified gala processes; see [roles integration cleanup](../roles-workspaces/tasks/integration.md). Independent read-only confirmation found neither PID1813640 nor PID1813644, and the updated debug CLI reports gala stopped/killed by pty. No additional signal, database rewrite or daemon restart was necessary. Package output is reserved at `.agentx/.tmp/agentx-gala-fixed`; the concurrent roles artifact `.bin/agentx-roles` is separate and preserved.


### Deployment coordination pending

The concurrent roles release revealed and repaired two unrelated roles defects after this task's queued release had started. To avoid installing that older snapshot, the redundant release was stopped (Cargo PID1905586; recipe exit143), and deployment will consume the roles owner's corrected artifact only after its independent acceptance. This does not invalidate the three passed timestamp/TERM source tests. The normal `.bin/agentx` remains unchanged; its backup is `.agentx/.tmp/agentx-before-gala` (SHA256 f5246648e6712ac00b5b729d3522134a76800dc67d69e20bc0053312392df02e). Relevant source hashes/last-write times are in `.agentx/.tmp/gala-recovery-source.json`. Bime owns the corrected artifact acceptance/handoff; taro owns final TERM artifact verification and normal CLI replacement under the user's explicit fix authorization. Preserve `.bin/agentx-roles` and all live user daemons.


### Completed local CLI deployment (2026-11-14T16:55:53.262029+00:00)

Consumed bime's independently accepted final roles artifact `.bin/agentx-roles` (version0.7.31, SHA256 `6b67fb228008699cf0ac8cd1bc705ab192e9777311f81bcd43eda9c2c6704418`) and preserved that source path. This is a command-local opt1/no-LTO/16-codegen-units release build, not the default full-LTO package recipe. Before installation, four isolated actual PTY child checks against these exact bytes passed: absent/empty/dumb TERM becomes xterm-256color, and screen-256color remains unchanged. Those checks used disposable HOME/config/data paths, inherited AgentX identity was removed, and temporary resources were removed. The earlier three regression tests remain the source-level timestamp/TERM evidence.

Atomically replaced normal `.bin/agentx` with those verified bytes under the user's explicit fix/deployment authorization. Installed version output is agentx0.7.31 and SHA256 matches the accepted artifact. Normal live listing succeeds against the existing daemon; gala is stopped/killed by pty, with no successor launched. No daemon restart or additional process termination was performed. Rollback bytes remain `.agentx/.tmp/agentx-before-gala`. This closes the user-requested local fix; Windows normalization is shared source but still not executed here.
