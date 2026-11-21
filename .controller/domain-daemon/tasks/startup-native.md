---
schema: task
description: Implement bounded native daemon diagnosis/restart and ordinary workbench startup defaults.
status: completed
---

# Native startup usability

## Working model and contract (2026-11-10)

Owner: `139a1946-7a3a-4908-8fb7-296099fd697a` on `srv_yDbm9zuqIMiM`; acceptance peer: `f28a4bc5-249c-494f-9d6b-c0f63115c383`. Source/cases/docs only; no executable verification or live scope ownership.

Native lifecycle method: model selected scope, kernel-authenticated peer, singleton lock and process incarnation separately from native compatibility. Existing `Hello` rejects an old cohort before Ready, so an old server cannot supply version/cohort facts through that exchange. Never infer its loaded cohort from its replaced executable pathname. Ordinary calls retain exact Hello/generation fences and no replay.

| State / action | Contract |
| --- | --- |
| `daemon status [--scope PATH]` | Pre-identity, read-only lookup; omitted scope follows normal `AGENTX_DIR`/home resolution. Missing scope/endpoint is stopped; compatible Ready is running; incompatible reply is running-incompatible with actual authenticated peer and client cohort, with old-server cohort explicitly unavailable. Other failures remain unavailable, not invented stopped. |
| `daemon restart [--scope PATH]` | Explicit scope-local lifecycle mutation. Hold the existing startup lock through stop and reacquisition. Compatible peers use exact-instance Shutdown; incompatible Linux peers require kernel peer PID, nonempty incarnation, exact `daemon serve --scope` argv, loaded executable evidence, and the exact owner.lock inode held by that PID in `/proc/locks`; open a pidfd and revalidate before SIGTERM. No PID-only kill, force kill, data deletion or lock unlink. Other platforms refuse incompatible recovery when equivalent authority is unavailable. |
| Stop / acquire | Bound stop observation, wait for singleton release before starting current executable. A failed/uncertain stop returns a substantive error and never spawns. Current acquisition retains ordinary cold auto-start and exact fences; a concurrent independently started incompatible peer is refused. |
| `workbench [--scope PATH] [--project PATH] [--collaboration]` | Scope defaults to originating environment, project to invoking cwd; explicit flags win. Local startup acquires missing service in either mode, HTTP remains fixed to existing selected service. Finite mutations require explicit collaboration flag; passive observe remains separate. |

Structure: `daemon/lifecycle.rs` owns status/restart and bounded process recovery; `transport.rs` exposes shared startup-lock guard, locked acquisition and same-instance shutdown, and enriches mismatch diagnosis; `platform.rs` adds passive ordinary endpoint connection; existing `scope.rs` owns endpoint derivation. `cli/commands/daemon.rs` retains relay functions and adds separately named native entry/help using clap. Pre-setup router routes native lifecycle commands while `daemon serve` retains the sole persistence constructor. `collaboration/web` resolves launch defaults and acquires locally before HTTP serving. Existing daemon/CLI/workbench native guides receive usage and source ownership.

Cases to author: passive missing/private scope and no DB effects, actionable mismatch including quoted absolute recovery, singleton held/unreachable, incompatible peer refusal when argv/owner evidence is absent, deadline stop refusal without replacement, scope/peer identity checks and native parser rejection; workbench defaults/relative environment/explicit override plus unchanged actor clearing/finite HTTP mode. Acceptance adds actual genuine-old-build recovery and process/data/singleton outcomes. Authored cases do not establish execution support.

Rejected: weakening compatibility to send ordinary operations to old code, build inference from image path, deleting owner locks, force termination, daemon-wide auto-replacement and a generic supervisor. Platform limitation is explicit bounded Linux legacy recovery, with compatible native Shutdown retained on existing transports.

The callable skeleton was materialized and published directly to acceptance/controller before behavior. Implementation now retains the existing native protocol, canonical scope and database fences. Passive native status has its own nonrepairing `connect_existing`; ordinary acquisition keeps its existing repairing connection path. Feedback: the historical `cli/commands/daemon.rs` manages Relay only; native lifecycle therefore uses a separate pre-identity entry to avoid that name collision. No role, Provider or external-contract change was needed.

## Authored cases and receiving scope

| Exact filter / owner file | Meaningful covered boundary |
| --- | --- |
| `daemon::lifecycle::tests::` / `src/daemon/lifecycle_tests.rs` | Missing/empty/private scope without creation or permission repair; held singleton with missing endpoint; bounded stop failure retains ownership/data; exact legacy argv/image and lock-owner evidence; real framed incompatible peer produces actionable diagnostics but receives no unverified signal. |
| `daemon::transport::lifecycle_diagnostic_tests::` / `src/daemon/transport.rs` | Client cohort/scope/kernel PID, explicitly unavailable old build and shell-quoted absolute recovery command, including a scope with spaces/apostrophe. |
| `cli::commands::daemon::native_tests::` / `src/cli/commands/daemon.rs` | Finite native grammar, absent actor requirement, rejected unsupported start/stop/name forms, serve escape and side-effect-free local help. |
| `collaboration::web::tests::` / `src/collaboration/web/tests.rs` | Originating environment/cwd defaults, home fallback, explicit relative overrides, explicit collaboration mode, actor clearing and unchanged closed HTTP refusal cases. |

These source-authored cases are unexecuted by this owner. Acceptance alone establishes compiler/formatter/test/build/package/process/browser results in [canonical evidence](../startup-evidence.md). Existing unaffected native generation/no-replay/Provider evidence can be reused; no broad suites or real model workload are required. The exact startup outcome fixture is acceptance-owned [SU1–SU7](../../../tests/startup_usability/README.md).

The stop/refusal fixture contract is `DaemonScope::prepare` + held owner.lock + no endpoint => unavailable/refusal immediately, without spawn; a same-user endpoint returning native `ServerFrame::Failure(Incompatible(...))` without exact argv/image/PID-owned lock => refusal without signal; an owner that cannot complete Hello => bounded 2-second handshake refusal, without graceful-signal fallback. Current compatible Shutdown reply loss is uncertain and never authorizes restart spawn. A verifiable legacy owner that receives SIGTERM but keeps the singleton beyond 35 seconds produces a drain timeout and no replacement. Do not use this owner's unit-test executable as a successful legacy daemon: its argv intentionally fails exact `daemon serve --scope` identity.

Linux legacy recovery additionally requires accessible `/proc` process/lock evidence and kernel pidfd support. Missing evidence or syscall support is a refusal. Compatible Shutdown is retained for current Unix/Windows native bindings; other-platform runtime execution and incompatible legacy recovery are not claimed. No new process-global authentication or runtime compatibility family is introduced.

Current contract/usage updates are merged into [daemon guide](../../../src/daemon/AGENTS.md), [CLI guide](../../../src/cli/AGENTS.md), [collaboration guide](../../../src/collaboration/AGENTS.md), [lifecycle usage](../../../.docs/architecture/daemon.md#operator-status-and-recovery), [workbench native usage](../../../.docs/collaboration-workbench/native.md#workbench-startup-and-defaults) and the existing README-linked [everyday startup entry](../../../.docs/collaboration-workbench/frontend.md#startup-and-everyday-use). Root/shared Delivery navigation remains controller-owned.

## Coherent source release r1 (2026-11-10)

Source/case/docs authoring is complete and all writes are released to independent acceptance. No Cargo/Node/formatter/compiler/test/build/browser/product process was executed. No execution resource, live scope, installed/default executable, protected demo, running workbench or `.bin` was created/changed/operated by this owner. No manifest, lockfile, DB schema, Provider or frontend implementation change is part of this release. Acceptance may apply required formatting to this released set and record the resulting frozen-source identity; return bounded semantic/compiler defects directly to this Task's author. Delivery acceptance and user-runtime promotion remain controller-owned.

Exact source/case/docs bytes before acceptance formatting (SHA-256):

```text
776d185e134d0fbe6d6346d3424760910dca2119412aa2792052deb4d54dd1db  src/daemon/lifecycle.rs
c0630734781647056ff73e58b82737cd3b14432df73ca61dd9b693ec64024d30  src/daemon/lifecycle_tests.rs
8626cbd7399c2da3bb7f23f8a853cb44fe954c8034955f2578f666ef6826e828  src/daemon/mod.rs
3020657f2be78046c2df19c52fca9c442b7849520e541f05314421272263e89a  src/daemon/platform.rs
c6dd533f3ccaea4048faa6492d6596a5a1ea67ebf1770075cd389ef8afc1f3df  src/daemon/scope.rs
a5aecfc79382ecae33da32afad9c01081da9fed97a3e6fb395e5f4cc6cfda032  src/daemon/transport.rs
278b30c0c803a33b1fedc70d0b7a2cfe3036501c5571f482d81efe65d26585d6  src/daemon/server.rs
344de6edced46d755504e2bf3064acabebc84adc7ae57e05c6d0d8c3d584869b  src/cli/commands/daemon.rs
3f26750f2f6f8372c3283694f9d1e7f1ada2a55a5c65214ac86217eb7b48620e  src/cli/commands/help.rs
6c62c6ddea103f9de167120619cf9c42d9df4a0885af45b05061a81a2808978e  src/cli/router.rs
9e5e811fa0c9fffa55a70e04296a1846a8872166cf868387e51c145b94965559  src/collaboration/web/mod.rs
dafd4113b245f647f59e0dc4477f51c00bd4f0be7d1bdf49e862c696bb8b652f  src/collaboration/web/tests.rs
eaee234cfec0984864edeecd581a10e263f3bbf223f9b632b45f95510ba248a7  src/daemon/AGENTS.md
327a915cdb583cbef5673f9d25f207f0e4bd641c759f68edd8153442319bea5d  src/cli/AGENTS.md
fc559dc9f1f2562f2e0796989a9232c6e59ea866fc1e48f961151498ddc13e81  src/collaboration/AGENTS.md
c507138759d9f82864de62ca929e7aafedebc35b6f1af36a697df4ab64f2aaa6  .docs/architecture/daemon.md
c42b5235622b0b97245c8502b5db8753ca5b6f724577ea1916c4a18eec2daf56  .docs/collaboration-workbench/native.md
9ffca35751b12b1e0973cf1d0487e78cb4dcc7a664254b9ba1d325cbe3d9f4f3  .docs/collaboration-workbench/frontend.md
```

## Independent receiving receipt (2026-11-10)

The acceptance owner released its [completed bounded result](../startup-evidence.md#completed-bounded-result-and-handoff): all 18 authored release hashes matched, scoped formatting changed seven released Rust files, 17 exact-filter cases passed, one fixed package passed, and actual SU1–SU7 established genuine r8 mismatch/recovery, data/lock retention, scope isolation, concurrent alias restarts, exact refusal and embedded workbench defaults/explicit mode. No product defect or author repair was required. The two retained failures were fixture mistakes; the canonical evidence owns their corrections, immutable post-format compiler inputs, candidate identity and complete cleanup. All authored source/resource claims remain released. The result establishes bounded Linux local support with `/proc`/pidfd requirements; other-platform legacy recovery and runtime support remain outside it. Candidate promotion and the user's running daemon/workbench cutover remain controller-owned.

Advance [startup Delivery](../startup-usability.md). Read its actual diagnosis, bounded contract and verification/resource allocation, root AGENTS, daemon/CLI/service guides and current lifecycle architecture. Restore native process identity and lifecycle semantics before implementation; record the concise contract/design and intended module/signature boundaries here, materialize the skeleton, then implement. Preserve ordinary automatic acquisition, exact compatibility fences, uncertain-effect behavior, data/singleton ownership and current mode boundaries.

Own `src/daemon/**` except immutable historical acceptance receipts, bounded `src/main.rs`/CLI router/help/native daemon command seams, workbench Launch/default/acquisition source/tests in `src/collaboration/web/{mod.rs,tests.rs}`, and corresponding lifecycle/CLI/workbench native usage/source guide receiving. Author meaningful isolated lifecycle/CLI cases. No unrelated domain, DB schema, frontend, manifest/lock, environment/installed binary or actual user scope writes. Shared Delivery/root entry remain controller-owned. If a specific outside seam is unavoidable, name it and its exact required effect.

Publish callable contract/skeleton and then coherent source/case/docs release to the acceptance owner from this Delivery's bindings; send substantive decisions/blockers/final release to the controller. You own professional implementation choices within the proportional outcome; avoid a generic supervisor or protocol framework. No nested delegation. Do not execute Cargo, Node, formatter, tests, builds, browser or product commands; independent acceptance is the sole executor. Complete on usable implementation/case/context handoff with all writes/resources released, and return concrete verification failures here.
