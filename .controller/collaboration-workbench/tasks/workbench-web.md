---
schema: task
description: Implement the selected professional task workbench and its contextual assignment, collaboration and acceptance interactions.
status: completed
---

# Web workbench implementation

Advance the [Delivery](../delivery.md) through the user-discussed SVG direction and released native/browser contracts. The user has authorized implementation and explicitly requested component development, Block assembly, then page state/method realization. React/TypeScript/Vite with Rust-embedded fresh assets is selected from the released engineering design. Actual runtime binding belongs to the [controller](../controller.md#implementation-dispatch). Keep this one cohesive Web Task rather than creating a Task per component or state.

The user broadly selected the revised resource-workspace direction and requested detailed components/operations/states before implementation. Consume the experience owner's actual detail release through the [design entry](../../../.docs/collaboration-workbench/README.md); four overview SVGs alone are insufficient. Implement defined component/state/layout contracts and meaningful unavailable/empty/pending/error behavior within this same Task, rather than splitting every component or small state into another workstream. Page-local drafts must not appear as persisted native work.

Implement structure and component boundaries before behavior. Preserve the main working subject through assignment, conversations/questions, outputs, independent checks and acceptance. Maintain meaningful pending/error/unknown states, keyboard use and narrow layouts without speculative feature branches. Use React if the design justifies it; build fresh assets for Rust embedding and keep external assets optional for development.

Apply the [shared interface design/review standard](../../../.agent-guides/scenarios/interface-design.md). Implement the user-discussed revised resource structure, useful information/action density and contextual disclosure; do not reintroduce the rejected explanation-heavy first offering while translating SVG to components. Necessary labels, real task content and meaningful errors remain visible at their point of use.

## Professional basis and component-first realization

Read [team entry](../AGENTS.md), [Web guidance](../../../.agent-guides/domains/webdevelopment/AGENTS.md), [actual design/details](../../../.docs/collaboration-workbench/interface-details.md), [experience](../../../.docs/collaboration-workbench/experience.md) and [engineering/module design](../../../.docs/collaboration-workbench/capabilities.md#engineering-structure-and-frontend-choice). Read relevant structure, component, design-system and interaction/state methods through that entry. All thirteen SVGs are design inputs; meaningful task content/errors/consequences stay visible and explanatory guidance is disclosed locally.

First materialize the module/layout/component skeleton and actual component contracts: data inputs, emitted intent, state ownership and important variants. Define the Page/native-client boundary before writing behavior; this does not require implementing Page behavior first. Then implement semantic leaf components, assemble coherent Blocks inside the resource layout, and finally wire Page reads, command methods and its finite selection/draft/pending/unknown/conflict transitions. Keep return statements structural and name calculations/handlers. Exported TSX modules/components require concise `@aiPurpose`/`@aiUsage` contracts.

Use the published component/state contract as a starting point, not a framework to expand. A private component may share its Block's file. No mandatory registry, Storybook, global state library, generalized form engine or router platform is needed. Start with React state/reducer, typed client functions and AbortController. Author critical transition/component cases beside the implementation, not every visual combination.

Build layout/leaf components from their own inputs while native contracts are being materialized. Import the native owner's actual `api/` closure when released; do not invent a second DTO, pretend source stubs are a real endpoint or fill the final product with illustrative fixtures. Native gaps go directly to the native owner with scenario and expected outcome. Routine compatible design/implementation choices need no further user approval.

## Assigned write boundaries

| Region | Owned work |
| --- | --- |
| `web/workbench/**` excluding `api/**` | React app, semantic layouts/Blocks/components, state/selectors/handlers, tokens/styles, HTML, configs/build authoring, owning AGENTS, meaningful authored cases. Native owner alone writes `api/**`. |
| Build/manifest | Root `package.json` scoped compatible package/script additions, `build.rs` fresh dual-app embedding, Cargo.toml source/archive include changes only if needed, bounded `justfile` recipes and `CONTRIBUTING.md` packaging instructions. Do not alter plugin dependency pins or add Rust dependencies. |
| Dependency lock | Publish exact package pins and manifest early, then transfer the released manifest region to the acceptance executor for lock resolution/install. Acceptance owns `package-lock.json` and any necessary Cargo.lock regeneration; do not run npm/Node/Cargo or race that writer. Resume manifest edits only after explicit bounded release. |
| Native asset seam | Native owner writes shared Rust asset/host/wire code; you own generated table/manifest/build output definitions. Agree actual table names/path/MIME/embedding declarations directly before dependent edits and notify the controller of the stable seam. Preserve existing diagnostics embedding. |
| Context | New `.docs/collaboration-workbench/frontend.md` if needed for actual frontend structure/state/build usage; source guides/cases and this original Task. Shared feature entry/catalog remains controller-owned. |

No unrelated Rust domain/HTTP edits, accepted SVG redesign, installed/PATH programs, `.bin` mutation, runtime/listener/provider or running demo writes. Pin mutually compatible packages through the existing root package manager/lock with no parallel lock system or unrelated upgrades. A source build must compile current assets into Cargo OUT_DIR and fail on frontend errors; packaged users need only the binary.

## Execution, communication and completion

The original acceptance owner alone runs install/lock resolution, formatter/compiler/Cargo/Node checks, app builds, tests, browser and package/product execution. You author source and meaningful cases without claiming they pass. Send an initial acknowledgement, the early tooling/manifest + component structure release, concrete contract needs, and the final coherent source/case/docs release. Normal progress stays here; no nested delegation.

Complete on the actual interacting component/Block/Page implementation and coherent build/source/context/case handoff, with all writes/resources released. Independent acceptance owns screenshots, real native effects, desktop/narrow/key/touch evidence and standalone packaging. Accepted result must come from native facts; no optimistic acceptance or inferred delivery/work completion.


## Implementation working record — 2026-11-09

- Read the accepted thirteen-scene interaction contract and professional frontend structure/state/component methods. Model and dependency direction are recorded in [frontend.md](../../../.docs/collaboration-workbench/frontend.md). Layout and build skeleton are materialized before behavior.
- Exact compatible package additions were released directly to acceptance (`8af2e306-f772-4e3a-b043-e589eecc6727`) for sole lock/install. package.json is frozen by Web until bounded release back. Registry peer/engine inspection only; no installs or executable checks by Web.
- Dual-app table/manifest seam proposed directly to native (`47fdbf85-11ed-4475-a96c-282a3293cbf9`): existing diagnostic table preserved, new `OUT_DIR/workbench_assets.rs`, flat hashed assets and exact existing `{files:[{url,file,content_type}]}` format. Await actual API closure before Page wiring; leaf/layout work proceeds independently.
- Source archive currently uses Cargo default inclusion (no explicit include list); no speculative Cargo.toml change needed. Build outputs stay in OUT_DIR; no new Rust dependency.
- Execution and browser evidence remain acceptance-owned and unexecuted by this Task.

### Coherent implementation release preparation

The structural skeleton was materialized before leaf/Block behavior: `contracts.ts`, named resource Layout and pure state/identity helpers established inputs, emitted intent and state ownership. Actual components now compose resource navigation/matrices, precise native participants, brief editing/validation, per-field conflict rebase, separate one-role replacement, exact questions/amendments, role-addressed notes and the real paged work thread, immutable result/check history and guarded acceptance. Page reads/commands are then wired through native-owned WorkbenchApi without production fixtures.

Direct native reconciliation supplied `source_reference`/`source_revision` byte limits, native OutcomeUnknown, frozen message basis and finite real Thread projection. Complete native bounded Detail needs no invented history pagination. Both command and unsaved criterion identity avoid secure-context-only randomUUID on HTTP LAN. Unknown replies retain original command identity; reload stores only finite reference metadata and cannot reconstruct input.

Acceptance reports exact Node22.23.3/npm10.9.9 dependency installation completed and returns package.json authority unchanged. Package-lock remains acceptance-owned; no manifest changes required after pins. Authored [cases](../../../web/workbench/cases.md) and [state cases](../../../web/workbench/tests/state.test.ts) are unexecuted by Web. No native/API/lock/live-demo writes or executable resources were taken.

Friction resolved within original Task: native source bounds were initially unavailable to editors and real Agent messages were not in operator history; native owner extended its actual contract. Small-screen resource navigation needed a visible disclosure rather than hover; the Layout now owns it. Source bounds, current/history labels and conflict/unknown states follow native facts. No additional feature/team/approval stage introduced.

### Source release web-workbench-r1

Coherent release: `web/workbench/**` excluding native-owned `api/**`, root package.json pinned additions, dual-app build.rs, bounded justfile recipes, CONTRIBUTING packaging instructions, [frontend model](../../../.docs/collaboration-workbench/frontend.md) and [authored cases](../../../web/workbench/cases.md). All Web writes/resources are released to the sole acceptance executor for freeze/format/compile/build/test/browser/product execution. No work is represented as executed merely because its source exists. Native `native-workbench-r1` is consumed directly, including Thread and exact source limits. No further implementation authoring remains absent a concrete independent defect; any such fix returns to this original Task.

Known evidence boundary: exact rendered desktop/narrow density, focus/touch/zoom and interacting/native effects remain unverified here and must be established by acceptance. HTTP LAN is supported in source without secure-context-only IDs. Clipboard failure reveals selectable references. No runtime/default service, installed executable, `.bin`, dependency lock or production example dataset was changed by this author.

### Bounded repair web-workbench-r1.1

Independent acceptance reported the scoped TypeScript check rejected `CommandFeedback` access to `change` after its earlier branches: the shared pending/unknown union member did not fully narrow by elimination. Added an explicit `committed` discriminant before receipt rendering in `web/workbench/workbench.tsx`; no casts, any, type weakening or domain behavior change. This one source file and this Task note are released for the frozen overlay. Acceptance already passed the five unchanged state cases and retains their evidence; only the affected typecheck/build proceeds under its sole execution authority. Web ran no check.

### Reopened tooling compatibility diagnosis

Acceptance passed the strict TS repair and reused five state cases, then isolated a zero-stderr SIGILL before any JS source or native Cargo compilation: importing Vite8.3.7 loads Rolldown1.2.18's Linux x64 native binding, whose ELF static initializer faults. Diagnostic builder passes in the same Node22.23.3 environment. No broad retry is justified until the actual offending instruction/version contract explains a correction. Web owns bounded manifest/pin selection, acceptance retains lock/install/probe/build authority; no new toolchain workstream is selected. Upstream source reviewed so far declares ordinary x86_64-unknown-linux-gnu builds and no explicit AVX2 baseline; missing AVX2 is still a hypothesis.

### Exact compatibility release web-workbench-r1.2

Causal evidence: acceptance's GDB catches BMI2 `shlx` at `0x4f336f` in Rolldown1.2.18's Linux x64 ELF static initializer, and host CPU lacks BMI2; static objdump confirms. No frontend or Rust source reached. Selected same-stack correction: add exact `@rolldown/binding-wasm32-wasi:1.2.18`; scope override `vite@8.3.7 → rolldown:1.2.18` (inside declared ~1.2.18). Vite/plugin/React remain pinned unchanged. Matching official WASI package supports exact Node22.23.3 and has no OS/CPU restriction. Loader source explicitly skips native binding under `NAPI_RS_FORCE_WASI=error`; builder sets force/flavor/version-check before dynamic import. Missing portable binding fails closed. No older native release was declared safe without evidence, no package code patched, no new bundler/stack chosen.

Released exact files: package.json (additive package/override only), web/workbench/build.mjs, build.rs (watch portable bundler inputs), CONTRIBUTING.md, frontend.md and this Task. Acceptance alone resolves/installs the lock, probes import and executes affected build; prior TS/state/storage evidence is reusable. All writes released for that bounded operation. Original failed native-binding evidence stays preserved.

### Independent compatibility receipt received

Acceptance reports r1.2 lock/install added six packages, preserved all 567 preexisting identities and removed none; package-lock SHA256 is `f98ef76c3f10abea1dfb1b49008c182c8448ea3faea87667d6ff9c25f3a8d09c`. Exact Node22.23.3 plus the three declared WASI environment values now imports the same Vite successfully in0.520s (expected Node ExperimentalWarning only). Receipt source: `/tmp/agentx-workbench-acceptance-lh863ck1/r3-wasi-import-receipt.json`, raw log `r3-wasi-import.log`, owned by independent acceptance. This discriminates and resolves the unsupported native-loader path; fresh frontend build is still running and native/browser/standalone outcomes are not inferred. No further Web source changes were made; all source authority remains released for the frozen r3-wasi candidate.

### Source assignment closed after independent builder result

The independent [same-version portable builder evidence](../evidence/acceptance.md#same-version-portable-builder-correction) now records both the corrected import and actual fresh frontend build, including its strict TypeScript check, passing on fixed r3. This satisfies the compatibility condition that reopened this source assignment; all implementation/case/context writes are released and no Web-owned work remains. Do not duplicate the executor's receipts or rerun passed checks. Browser/Provider/native-effect/standalone results are still outside this source completion and remain the acceptance Task's responsibility. Future concrete defects return to this original Task.

### Reopened for concrete O10 touch disclosure and final documentation receiving

Independent acceptance's actual Chrome hasTouch/390×844 case refreshed the accepted work but found no retained Tooltip (`browser-narrow-evidence`, harness-r17/browser_flow.mjs:204+, retained under `/tmp/agentx-workbench-acceptance-lh863ck1/`). Keyboard focus/Escape and the full native two-Agent outcome already passed and remain reusable. Scope is `components.tsx` plus authored cases/context; no manifest, runtime or lock change. Model: activation reveals the label/reason and invokes an available callback once; only actual mouse hover enters/leaves alter hover disclosure; Escape, blur and outside pointer dismiss. This avoids touch relying on browser focus or compatibility mouse events, including the focus-then-toggle race for blocked/information actions. Existing component signatures, focus ownership and native command/state contracts stay unchanged.

Final docs receive the independently observed Linux startup/use/support scope through canonical evidence, including explicit scope/project/nondefault port/collaboration and CLI-compatible `--name`. Final narrow result/touch, unavailable-state and cleanup/promotion observations are not claimed while still pending. No new executable check is authorized to this source owner; acceptance freezes the exact presentation overlay and owns affected TS/build/artifact/browser checks.

### Concrete narrow evidence layout correction

Independent r7 touch Tooltip/Escape/focus/reference actions passed; actual screenshot inspection then found long result/check UUIDs forcing a vertical evidence heading, clipped selector and approximately44px task identity column. The existing narrow layout used an intrinsic `auto` status column and let the select label retain its minimum content size. Its closed sticky action footer also overlaid the evidence reading area. This is the same Web Task, limited to CSS and authored case/context, with r7 native/Provider/task evidence unchanged.

Selected narrow structure: task and check columns receive explicit proportional space, long status text ellipsizes only in the clickable overview, and full report identity remains in local evidence. Heading stays on one line while the native selector shrinks inside its available width; its complete option and full result heading remain available. Evidence gets a larger finite share of the existing workspace, check findings precede long result narrative visually, and all-disabled decisions stay in document flow. No information, actions, actual IDs, component signatures, data or domain contracts are removed. Acceptance owns fresh build/embed and representative narrow observation, including title/check proportions, selector bounds, visible actual findings and full exporter reference disclosure.

### Coherent bounded release web-workbench-r1.4

Exact new compiler input is `web/workbench/styles/workbench.css` only, appended narrow evidence/overview rules; preceding r1.3 `components.tsx` is frozen and independently passed touch. Authored case receiving is `web/workbench/cases.md`; support/model receiving is `.docs/collaboration-workbench/frontend.md`, with existing startup routes in CONTRIBUTING.md and web/workbench/AGENTS.md retained. All owned writes/resources are released to acceptance for the affected fresh build/embed/narrow check. No runtime/config/manifest/lock/component/domain change or executable check was performed for r1.4. Source/case/docs assignment closes with actual final CSS/artifact/cleanup observations explicitly pending under the independent acceptance Task; any concrete defect returns here.

### Independent r8 layout observation received

The [canonical final narrow evidence](../evidence/acceptance.md#final-narrow-evidence-layout-correction) now records the r1.4 fixed-source build/strict TS and actual390px layout/touch discriminator passing. The run uses staged r7 host plus explicit fresh r8 external assets; it is not final embedded-artifact evidence. Task/title/role and check proportions, horizontal heading/contained selector, current result/b1/checker with C1–C3, touch-expanded real criterion finding and full exporter repair reference are independently observed with focus/selection retained and no apply. Earlier failed images/attempts and unchanged native/state/real Provider evidence remain retained by acceptance.

Docs-only receiving updates frontend.md, CONTRIBUTING.md and web/workbench/AGENTS.md to link that result and distinguish pending final r8 embedded observations, artifact digest and cleanup/promotion. Compiler/source inputs remain frozen and unchanged; this owner ran no check and holds no writes/resources. Original Task remains completed for source/case/context release. Acceptance owns final exact-Node22 package and subsequent artifact/cleanup handoff.

### Final artifact and cleanup receiving complete

The [canonical final handoff](../evidence/acceptance.md#final-artifact-evidence-and-cleanup-handoff) resolves the prior pending artifact/cleanup records above. Immutable r8 packaging under the explicit task-local Node22 environment passed; final `agentx 0.7.31` artifact/source identities are retained there. The same binary, launched from unrelated empty cwd without external assets, passed embedded workbench/diagnostic HTTP and exact bundle-byte identity, actual narrow geometry/current findings/exporter disclosure/tap refresh/Tooltip/Escape/focus with no apply, and original diagnostic module/style mounting. Original r5 native/state/actual two-Agent outcome is reused through the presentation-only derivative; no redundant outcome rerun is claimed.

Acceptance atomically promoted identical final bytes to repository-local `.bin/agentx`, preserved the old baseline and completed exact disposable product-role/browser/host/daemon/port/fixture cleanup. Protected demo process identities and installed/PATH/default programs remain unchanged. Current frontend.md, CONTRIBUTING.md and web/workbench/AGENTS.md now link the completed evidence and scoped support without stale pending claims. Historical attempts above remain chronological evidence. No source/config/manifest/lock or executable check was changed/run by Web in this receiving step. All docs writes are released; no owned work/resources remain. Task stays completed; formal Delivery acceptance belongs to controller/user.
