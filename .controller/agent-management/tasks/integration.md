---
schema: task
description: Own management native contracts, durable operation semantics, HTTP and daemon composition, and candidate integration.
status: completed
agent: gabi
---

# Integration

Own `src/management/` except worker.rs, additive service/daemon/CLI/build wiring and native-derived Web contracts. Publish design and skeleton before parallel implementation. Run required scoped Cargo and package checks; integrate bounded repairs from independent acceptance. Update current docs and evidence before closure.

## Integration record

The native service leaf, journal/query store, isolated worker, daemon listener lifecycle, early CLI `daemon web` entry and generated Web bindings are implemented. Canonical command admission remains separate from worker invocation, so identical replay does not depend on the current filesystem or project setting. The daemon bounds 16 connections, 8 active HTTP calls and 4 execution workers; a 120-second worker deadline reports uncertainty without claiming child cancellation.

Frontend owner handed off strict TypeScript/build and isolated desktop/mobile browser checks. Runtime owner handed off 9 focused native journal cases. Independent validator owns the public HTTP/browser run on a fixed candidate. Existing live daemons, installed programs and protected demonstrations remain untouched.

`python3 scripts/check-db-ownership.py` passed. Initial packaging reached frontend compile and found a React 19 `useRef` initialization error; Web owner corrected it. A later native compile found a SQLite parameter `usize` conversion in pagination; integration corrected it to `i64`. The first release LTO run was stopped before completion to prioritize final-source native checks and a reviewable candidate. The exact final build/check evidence will be linked here after execution; none of these in-progress records constitutes acceptance.

Native checks passed: [9 journal state cases](../verification/native-journal.log), [2 CLI grammar/help cases](../verification/cli.log), and [11 existing browser boundary cases](../verification/browser-boundaries.log). The native-derived management TypeScript was regenerated to a temporary file and compared byte-for-byte with the checked-in projection. Two long foreground package attempts received SIGTERM without a compiler error. Final packaging uses a detached `just package` with `CARGO_PROFILE_RELEASE_LTO=false CARGO_PROFILE_RELEASE_CODEGEN_UNITS=256 CARGO_PROFILE_RELEASE_OPT_LEVEL=0` for this local functional candidate; source profile defaults remain unchanged. The candidate is not a release performance benchmark.

First independent candidate `c39a26a8…` exposed a management configuration bug before Provider launch: `toml::Value::from_str` parsed a value rather than a complete TOML document. Replaced it with `toml::from_str`, matching the existing configuration owner. The failed [pula-01](../verification/pula-01/) run remains retained and cleaned; r2 is built separately before resubmission.

The fixed r2 passed configuration/embedded resources, then [pula-03](../verification/pula-03/report.json) found a successful native launch missing from the HTTP list. The management SQL treated only NULL origins as local, while native instance creation also stores an empty origin string and `InstanceRow::from_row` normalizes both to None. The list predicate now follows this existing native convention before pagination. Runtime owner adds the focused local/remote projection regression; integration will execute it and package a distinct r3 candidate. r2 is retained unchanged. The intervening pula-02 failure was the validator's private `qodercli` alias setup and required no product change.

The [local-origin and pagination regression](../verification/native-projection-r3-receipt.json) passed, and [r3 packaging](../verification/package-r3-receipt.json) completed to `.bin/agentx-management-r3` with the same documented local functional profile. Independent acceptance received this immutable candidate. Management native files pass rustfmt; the documented native-to-TypeScript command matches the committed projection byte for byte. Current acceptance remains pending.

R3 passed launch, messaging/reply/transcript, duplicate/conflicting requests, bounds and pagination. [pula-05](../verification/pula-05/report.json) then exposed F03: an in-flight Qoder hook recreated a retired launched identity under a new name before resume. Native `invocation_binding` allowed missing launched process bindings to enter orphan creation, and ordinary status callbacks could race the killed marker. The design now requires launched-session binding to preserve the live native process binding and status callbacks to reject killed rows. Integration owns the causal native repair; toku authors independent cases for retirement, killed marker, allowed adoption and live session switches in the existing native hook module. No fixture behavior or exact resume expectation is relaxed.

F03 repair includes both creation and non-creation launched binding paths, while preserving explicit soft-stop recovery. HookSession also rechecks the current killed marker after retained-result lookup, preventing a cached callback reply from admitting an already-killed participant. Six independent regression groups are handed off; integration executes them with affected existing hook retirement, operator kill, binding and session-switch cases.

Rendered integration inspection used the Web owner’s retained 1280×800 and 390×844 fixture images to answer whether selecting an Agent, reading messages and composing work remain in one workspace. Desktop exposes the resource rail, selected identity/status/tool, message context and composer together, with launch/stop/transcript/recovery controls directly reachable. Mobile replaces the rail with a local picker and retains the same selected identity, tabs, content and composer; it does not stack unrelated pages. No substantive layout blocker was observed. These are fixture image observations; actual HTTP/browser lifecycle remains independently pending.

[Native F03 execution](../verification/native-retirement-r4-receipt.json) passed all 26 selected cases: six new session-admission groups and existing binding, operator kill, canonical retirement and session-switch rollback cases. Final-source r4 packaging is started under the same documented local functional profile. No native or Web source changes are made during this build.

[Fresh embedded r4 package](../verification/package-r4-receipt.json) passed. Immutable candidate `.bin/agentx-management-r4` has SHA-256 `1a928f91f4dde4f19eebcf6ec64f75419688560948f6689de907b398be2b1d1e`. Independent public HTTP/browser acceptance received this candidate, including CLI enable/autostart/reconfiguration M11. No installed/default binary or running user daemon was replaced.

R4 [pula-06](../verification/pula-06/report.json) passed every public native HTTP case: M01–M11 plus transcript, exact stopped/successor history isolation, reset/scope, restart retention and CLI autostart/hot reconfiguration/deleted-directory replay. Cleanup passed. The run status remains failed solely because browser launch selection used a label-text locator that included nested option text; native ARIA exposes the expected combobox name. Validator repairs its role/name locator and runs only the browser on a fresh fixture with this unchanged candidate. No passed native cases are repeated.

Integration accepted the independent unchanged-r4 result: all public native cases passed in pula-06 and actual browser B00–B07 passed in pula-07 after a harness-only locator correction. Both final manifests show cleanup success. [Delivery](../delivery.md#accepted-local-outcome) owns the final supported scope, immutable artifact and limits. All integration work is complete; installed/live rollout remains separate.

Post-delivery consumer assessment: zumi relayed the user-authorized visual office supply question. [Assessment](../../../.docs/agent-management/office-supply-assessment.md) checks current native/HTTP source, recommends the existing AgentView collaboration projection first, identifies legacy/corrupt decoding and pure endpoint-preview query gaps, and separates future catalog/discovery/actions. Read-only source/document work; no frontend/runtime/build or LAN changes, no additional implementation acceptance claimed.
