---
schema: document
description: Complete, unexecuted task-search design instance and explicit Playwright/report mapping.
---

# Read-only task search: complete illustrative instance

**Design example, not actual product execution.** No example host, Playwright dependency, browser run, trace or screenshot was created. Parsing this example establishes only its construction; it is not E2E evidence for this scenario.

| Owner | File and meaning |
| --- | --- |
| Product meaning | [source.md](source.md): actor/need/assumption, requirement, scenario and criteria; sole example promise authority |
| Executable path projection | [flow.json](flow.json): versioned data, two guarded branches, four steps/assertions and manual observation |
| Native projection | [search.spec.mjs](search.spec.mjs): direct Playwright Test code; uses data values, IDs and fixture records without a generic interpreter |
| Consumer config projection | [playwright.config.mjs](playwright.config.mjs): explicit target/build/run selection, isolated contexts, native HTML/JSON/trace outputs |
| User reading view | [report.md](report.md): exact linked coverage and not-run/manual gaps; no fabricated artifact links |

The model evolves from ACT-1's assumed remembered-title characteristic through NEED-1 → REQ-1 → SC-1 and CR-1..3, then PAGE-1/actions and executable checks. That remembered-title characteristic is an illustrative analysis of ACT-1, not empirical research. The [semantic data basis](../../data-model.md) explains how these subjects remain connected while reusing the consumer's native identities.

| Criterion | Page/action → steps | Native assertions/test | Current evidence |
| --- | --- | --- | --- |
| CR-1 | PAGE-1 open + CTRL-SEARCH fill → ST-1,ST-2 | AS-0 + AS-1 / PW-1 / BR-1 | Not run |
| CR-2 | PAGE-1 open + no-match fill + CTRL-CLEAR click → ST-1,ST-3,ST-4 | AS-0 + AS-2 + AS-3 / PW-2 / BR-2 | Not run |
| CR-3 | Neutral task observation and participant explanation | OBS-1, manual; no Playwright usability verdict | Not observed |

AS-0 is a shared setup assertion, not another requirement or independent test. Each native test covers a complete branch. CR-2 requires both empty-state and restored-state assertions. This is stable traceability with many-to-many joins, not one click per test.

## Future consumer execution prerequisites

These instructions are a projection for an authorized future consumer; **do not run against an existing shared or live service**. First obtain a disposable host owned for this test, implementing source.md's exact controls and immutable FX-1. No host is supplied here. The future Coder selects its existing native dependency/runner version and browser; no machine cache import or automatic dependency download is prescribed. Existing CDP/Playwright scripts can be used instead if their mappings/evidence preserve the same guarantees.

Copy/adapt the explicit binding into that consumer's existing suite and inspect API compatibility with its installed version. Set `PX_EXAMPLE_OWNED=yes`, `PX_EXAMPLE_ORIGIN` to the exact owned loopback origin, `PX_EXAMPLE_BUILD` to its immutable build identity, `PX_EXAMPLE_RUN_ID` to a fresh unique run identifier and `PX_EXAMPLE_OUTPUT` to a fresh owned absolute directory. A caller-supplied build label must be backed by the consumer's actual build receipt; the sample does not attest host identity. The host must be read-only and self-contained: same-origin GET/HEAD restriction is defense in depth and cannot prove GET handlers have no side effects.

Using the already installed consumer runner, the native invocation shape is:

```sh
./node_modules/.bin/playwright test --config path/to/example/playwright.config.mjs
```

Resolve that runner from the consumer project, not this repository; this is an instruction example, not an executed command. No webServer setup is included. Fresh contexts reset browser-local state, while each page load supplies immutable FX-1. No domain writes or business reset are involved. Missing/reset-incompatible fixtures block execution rather than silently skipping assertions. If the future host introduces service writes, external authentication or another origin, revisit the owned fixture, authorization and reset before adapting this binding.

## Evidence association after a real run

The native output directory would contain `results.json`, `html/` and `artifacts/`; these are **planned names, not existing links**. Native reports identify test title, step, result, retry and attachment paths. The spec attaches actual source/flow bytes and their SHA-256, build label, fixture revision, run ID and attempt; the consumer retains runner/browser/config/test-code versions and build receipt too. CR/AS/ST IDs in titles/attachments let a reader join them to source.md and flow.json. No report assembler is implemented.

For each actual attempt, use the native result to associate `run ID + PW test + retry + ST/AS ID` with its concrete screenshot/trace path and raw status. Successful checkpoints have ST-2/AS-1, ST-3/AS-2 and ST-4/AS-3 screenshot attachments. ST-1 has setup assertions and trace; no separate setup screenshot is promised. On failure, native failure screenshot/trace and the failing step remain evidence even if the planned success checkpoint never ran. Missing/unvisited artifacts remain explicit. CR-3 needs a separate actual observation record and assistance accounting.

A hypothetical future result where PW-1 passes and PW-2 fails would support CR-1 and contradict CR-2 within the mock/UI boundary, leaving CR-3 unverified. This is reasoning about aggregation, **not an observed run**. Current report numbers remain zero executed and zero supported.
