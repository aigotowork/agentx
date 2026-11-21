---
schema: task
status: completed
agent: scene-verification-vetule
---

# Independent scene runtime regression execution

## Assignment

Read [original Task/design/source handoff](scene-runtime.task.md), [Space](../../web/agent-space/AGENTS.md), [scene ownership](../../web/agent-space/src/scene/AGENTS.md) and [test entry](../../web/agent-space/tests/README.md). Controller is space-responsive-zonu. Implementer scene-runtime-vidore / vidore has handed off integrated source and one passing scoped typecheck (#30974); no final tests/build/browser executed yet. You are a fresh project coder responsible for finite independent execution, not redesign or a general review. No delegation. Need for isolation: substantive lifecycle/shared-frame refactor requires an executor independent from its developer; do not repeat their typecheck as a separate command.

The consumer is the existing complete responsive Space. Establish whether the changed implementation preserves state semantics and actual Canvas/semantic DOM interaction. Use authored scenarios as a starting point, assess meaningful coverage and avoid private-field/source-text assertions. User #31003 has selected a default-enabled discoverable Canvas diagnostic/action facade. See the original Task selected extension for requirements. Implementer vidore owns it. Run baseline state checks first, then notify vidore/controller that baseline files are released for extension edits; defer final build/browser until expanded source handoff. Do not add diagnostics source yourself. Preserve passed baseline evidence; repeat only checks affected by subsequent relevant source changes.

## Work and authority

1. Run new `node --test web/agent-space/tests/scene-runtime.test.mjs` and affected existing `office-details.test.mjs` cases, ideally in one command. Record genuine failures; route source defects to vidore and inform controller. Source fixes stay with implementer.
2. Build once with scoped runner into a fresh external absolute output path; build includes typecheck. No Rust/native package or installed binary operations.
3. Use one executor-owned disposable static host on a free non-default loopback port, explicit `?supply=demo`, preserve all user listeners (especially :38023). Reuse available Playwright/Chrome. Existing camera/browser scripts may contain historical labels; adapt an executor-owned focused harness to current UI rather than weaken real guarantees or rerun obsolete cases blindly.
4. Finite actual-browser checks: drawing succeeds with no runtime error; Canvas draw actor coordinates and semantic actor anchor coordinates agree under camera projection; selection/pan/close retain camera and draft; desktop and narrow responsive rendering; pause/reduced-motion stops decorative progression; scene/chat or view remount does not accumulate RAF/subscription activity. Deterministic cases may establish low-level lifecycle cleanup where browser instrumentation cannot. Clearly distinguish measured browser facts from deterministic model coverage. Avoid per-frame screenshot/large logs. Capture minimal evidence; inspect images only for a named user-intent/layout question, not routine code verification.
5. Clean browser/host/resources you started; do not mutate native APIs or production resources. Stop after relevant checks pass; rerun only affected checks after repairs or if a harness error invalidated the result.

Allowed writes: this verification Task, focused new verifier-owned harness/test under `web/agent-space/tests/` (coordinate existing shared files), and temporary/`.bin/scene-runtime-verification/` evidence. No source edits; send substantive source defects to vidore. Preserve all unrelated dirty/untracked work. Keep write ownership explicit.

## Completion

Report actual commands/results, source/frame/browser coverage, defects/limits, build/output and resource cleanup here with links, then message controller. No claim of live adoption. If repair needed, coordinate directly with vidore on the concrete failing invariant while controller is informed. A passing independent report supplies the original Task's final evidence; controller closes it without repeating checks.

## Binding

Launch returned ready identity `vetule`, batch `e54a7c64`, project coder profile with explicit max reasoning override. Assignment explicitly accepted by scene-verification-vetule in #31017. Controller binding grants only the finite verification/resources described above.

## Expanded final browser acceptance

After vidore expanded handoff, use default global discovery with no query enablement and no hard-coded actor ID/source knowledge. Discover canvases, object model/list/relations, supported operations and their schemas/current availability; invoke an eligible real scene operation through the facade; inspect resulting state/presentation and capture actual correlated Canvas PNG. Validate detached JSON results, dimensions/frame metadata, unsupported/invalid/stale-target handling and disposal/unregistration. Preserve scope of original scene/camera/lifecycle checks; cover them in the same finite actual browser session rather than duplicate sessions. Verify screenshots are Canvas-only and documentation states DOM limits. Cross-origin host bridge/native actions/remote RPC are outside this outcome.

## Baseline execution — vetule

Executed before the selected diagnostics extension handoff:

| Check | Actual result |
| --- | --- |
| `node --test web/agent-space/tests/scene-runtime.test.mjs web/agent-space/tests/office-details.test.mjs` | Exit 1: all six office-details cases passed (capacity/stable seats/partial snapshots, native behavior/pause, indoor obstacle routing, lounge spacing). Runtime test file failed before executing cases: Node v24.22.2 strip-only loader rejects constructor parameter properties in `runtime/runtime.ts` (`ERR_UNSUPPORTED_TYPESCRIPT_SYNTAX`). This is test execution compatibility, not evidence of a failed runtime invariant. Sent the concrete loader failure to vidore/controller before continuing. |
| `node web/agent-space/scripts/run.mjs build /tmp/scene-runtime-vetule-20261115-build` | Exit 0; includes scoped typecheck, 72 transformed modules, baseline JS `index-D4B2J0sz.js`. This baseline build preceded scope-update receipt and establishes no diagnostics-extension evidence. Nonfatal WASI/config warnings only. |
| Disposable static host | Python host on `127.0.0.1:46309`, exec session 24466, stopped with Ctrl-C immediately after scope update. No browser started, no user service touched. |

Baseline files were released to vidore/controller (receipt #31066); verifier holds no source writes. Retain passed office-details evidence. Next: receive loader correction and expanded diagnostics handoff, rerun only affected runtime cases, then perform final expanded build/browser with the discovery contract. Browser/frame alignment, interaction, responsive, pause/remount and diagnostics checks remain pending. Baseline external artifact is retained as evidence and is not adopted.

### Baseline loader resolution

Controller #31072 and implementer #31075 confirmed Node24 transform mode as the supported execution contract; vidore owns documentation correction. Executed `node --experimental-transform-types --test web/agent-space/tests/scene-runtime.test.mjs` → exit 0, 10/10 cases pass. Coverage establishes deterministic scheduler/subscription/dispose/remount, pause/reduced/resume/time cap, hidden restoration, world/screen exact-key sample, arrival/reflow, selected hold/release, message batches/dedup/staleness/removed endpoints/rollover and exact browser-adapter listener removal. It does not measure real Canvas or mounted browser behavior. No source edit was required. Baseline source files are fully released for the selected extension; awaiting expanded handoff before final integration execution.

## Completed independent outcome

Expanded handoff #31260 is independently accepted for this finite Linux Chrome/static-demo scope. No substantive source defect remains from this execution. The source refactor and default-enabled diagnostics have real consumer evidence; this is not live-service adoption or general cross-browser/visual-parity acceptance. No source files were edited by the verifier. The only added executable is [the focused verifier harness](../../web/agent-space/tests/scene-runtime-browser-verification.mjs).

### Selected cases and actual execution

| Invariant/scenario | Input/event and expected result | Actual evidence |
| --- | --- | --- |
| Runtime/office baseline | Deterministic clock, exact identity, arrival/selection/message and real office/navigation semantics | Prior 10 runtime + 6 office-details passes above are reused. These source owners were unchanged by extension handoff. |
| New diagnostic model/lifecycle/actions | Empty/pre-frame/mount states; detached model; semantic furniture; valid/exact actions; invalid/stale/pending input; capture port errors; input/frame wait timeout/dispose | `node --experimental-transform-types --test web/agent-space/tests/scene-debug.test.mjs` → exit 0, 8/8. Port tests do not establish real PNG pixels; browser checks below do. |
| Expanded fresh artifact | Build includes typecheck; fresh external output, no native packaging | `node web/agent-space/scripts/run.mjs build /tmp/scene-runtime-vetule-expanded-20261115-build` → exit 0, 77 transformed modules, `index-BwaZBXwh.js`, CSS `index-qc9oCLWO.css`. One final build, no standalone repeat typecheck. Nonfatal WASI/native-config migration warnings retained in command output. |
| Unknown-ID discovery → real operation | Default global with only `?supply=demo`; discover model/canvas/objects/relations/actions, choose available unselected actor and invoke discovered zero-argument operation | Attempt2: opaque instance/actor/action IDs obtained through discovery, descriptor parameter/result promises checked, callback receipt remains `not-confirmed`, later frame/input inspected and exact actor selected through real App ownership. No fixture actor ID/name is hard-coded. Composite room furniture is discovered. |
| Canvas/DOM/shared camera projection | Actual Canvas `drawImage` arguments/current transform compared to presented actor bounds and imperative DOM CSS anchors in the same evaluation task | Eight drawn actors match at stable desktop checkpoints; six to eight match at narrow/short checkpoints depending on camera culling. Checks compare world sprite bounds, projected Canvas bounds and semantic anchor coordinates; sampled state alone is not used as proof of drawing. Runtime page errors: none. |
| Camera and exact draft continuity | Select → fill draft → close; discovered pan by `(23,-11)` → reselect → close | Attempt2: camera restores exactly on each close, pan moves camera by requested amount without zoom change, selected zoom retained, draft `independent scene continuity` retained on reselect. Selection is permitted to focus the camera; close restores its saved overview. |
| Actual PNG/frame correlation | Snapshot and capture in one synchronous evaluate; validate PNG signature/IHDR/backing and metadata | Attempt2 desktop and narrow4 390/320 captures have real PNG bytes, matching frame/input IDs and measured Canvas dimensions. Capture declares `canvas-only`; ordinary page screenshots retained separately. README truthfully excludes DOM labels/tooltips/chat. |
| Detached/error states | Mutate returned snapshot; invalid/unsupported/unavailable/missing operation, paused wait timeout | Continuation3: internal observed camera/exact identity unaffected by caller mutation; explicit `unsupported_action`, `invalid_arguments`, `unknown_object`, `unavailable_action`, `frame_timeout` results. |
| Browser pause/reduced motion | Pause true then compare after bounded observation; resume then emulate reduced preference and compare | Continuation3: same elapsed/frame/draw count, zero pending scene RAF while paused/reduced; browser preference transition works. Tooltip positioning loop was deliberately dismissed before global RAF measurement. |
| Real disposal/remount | Three scene→chat→scene cycles with pending waiters | Continuation3: empty canvas list, stale canvas `unknown_canvas`, waiter `disposed`; zero RAF/visibility/preference subscriptions unmounted; one pending RAF and exactly one visibility/preference listener after remount; each mount gets a fresh ID and retained camera. |
| Narrow/short responsive experience | Resize, discovered panorama, discover available actor, activate and inspect composer bounds | Narrow4: 390×844 and 320×640, both widths at height480; no document horizontal overflow, positive composer width, composer bottoms 777/582 at normal heights and 413 at height480. Actual actor drawing/DOM alignment and correlated PNG validated. |

### Browser runner, attempts and feedback disposition

One final static artifact was served by `python3 -m http.server 0 --bind 127.0.0.1 --directory /tmp/scene-runtime-vetule-expanded-20261115-build`, allocated `127.0.0.1:38161`. All browser runs used installed Playwright and `/usr/bin/google-chrome`, explicit `http://127.0.0.1:38161/?supply=demo`, no debug enablement query and no native API mutation. Runner command template: `AGENT_SPACE_URL='http://127.0.0.1:38161/?supply=demo' AGENT_SPACE_EVIDENCE='<absolute attempt directory>' [AGENT_SPACE_CONTINUE=pause|narrow] node web/agent-space/tests/scene-runtime-browser-verification.mjs`.

The original planned single uninterrupted browser flow needed focused continuations because executor assumptions were invalid. Earlier passed checkpoints are retained rather than relabeled as one all-passing run. No source or artifact changed between attempts; only the verifier harness timing/setup changed. The second attempt re-established the initial path because the first collector did not retain the failing anchor checkpoint. Later continuations skip completed selection/camera/draft and pause/remount groups, respectively.

| Attempt | Exit and disposition | Retained raw result |
| --- | --- | --- |
| `/tmp/scene-runtime-vetule-browser-attempt1` | Exit1 at reselect: immediate snapshot observed last presented Canvas while React had committed newer DOM anchor styles. Earlier discovery/select/capture/close/pan passed. Failure lacked the mismatched checkpoint payload, so capture was improved. | [attempt1/result.json](../../.bin/scene-runtime-verification/vetule-20261115/attempt1/result.json) |
| `/tmp/scene-runtime-vetule-browser-attempt2` | Exit1 at global pending RAF assertion after pause. Two presentation opportunities before checkpoint resolve prior DOM timing window, all earlier selection/draft/camera checks pass. Canvas/time already frozen; remaining RAF belongs to focused FactButton tooltip, a separate documented positioning owner. | [attempt2/result.json](../../.bin/scene-runtime-verification/vetule-20261115/attempt2/result.json) |
| `/tmp/scene-runtime-vetule-browser-continuation3`, `AGENT_SPACE_CONTINUE=pause` | Exit1 only when no eligible on-screen actor exists after large desktop→narrow resize with retained world camera. Dismissing tooltip proves quiescence; pause/reduced, errors/detachment, three actual disposal/remount cycles pass. No source defect established. | [continuation3/result.json](../../.bin/scene-runtime-verification/vetule-20261115/continuation3/result.json) |
| `/tmp/scene-runtime-vetule-browser-narrow4`, `AGENT_SPACE_CONTINUE=narrow` | Exit0. Uses the discovered panorama operation after resize before seeking a visible actor; all remaining narrow/short/PNG checks pass. | [narrow4/result.json](../../.bin/scene-runtime-verification/vetule-20261115/narrow4/result.json) |

Timing and tooltip findings were coordinated directly with vidore and controller; their replies #31309/#31316 confirm the last-presented/live-DOM boundary. They required no source repair. This is a documented asynchronous observation boundary, not a claim that last-frame DOM and current React DOM always coincide during commits. Finite stable checkpoints establish the shared-frame integration. The retained-camera narrow finding is handled through the existing discoverable panorama action, preserving its actual availability rather than bypassing offscreen guards.

### Evidence limits and cleanup

All raw attempt results, three actual correlated Canvas PNGs, two narrow page screenshots and artifact/source SHA256 manifest are retained under [owned evidence](../../.bin/scene-runtime-verification/vetule-20261115/manifest.json). Captures were not opened for routine verification; no aesthetic judgment, pixel-perfect historical parity or opaque-pixel/occlusion claim is made. Measured browser geometry/render calls, decoded PNG dimensions, public semantic state and deterministic lifecycle coverage are distinguished above. Favicon404 is incidental static-host behavior; no JavaScript runtime error occurred. Physical mobile keyboard, other browser engines, cross-origin bridges, native actions and selected LAN adoption remain outside this outcome.

Every launched browser was closed in the harness `finally`, including failed attempts. Static host session92011 was stopped with Ctrl-C; socket connection to `127.0.0.1:38161` then fails as recorded in the manifest. Earlier baseline host session24466 was also stopped. Temporary build directories and copied evidence are intentionally retained as artifacts, with no remaining owned process/listener. User services (including :38023), installed binaries, APIs and unrelated working-tree files were not changed. Verifier source-write/resource ownership is released. Completion conditions hold; controller may close the original source outcome using this evidence without repeating checks.
