---
schema: document
description: Maintain executable flow data, native Playwright projections and honest user-facing evidence views.
---

# Executable paths and evidence views

## Flow data and native projection

After prototype readiness, maintain a small structured projection beside the canonical scenario. Reuse existing scenario data when it already carries these meanings. The data must specify supported actions such as opening a page, clicking and entering input, plus their expected results; unsupported human or host actions must be explicit. A prose-only test wish list is not an executable handoff. Data does not imply an installed interpreter: bind it to explicit native code or an existing supported adapter.

| Link or field | Required meaning |
| --- | --- |
| Requirement / scenario / criterion references | Stable IDs and exact source revisions from the original owner; not a parallel catalog |
| Flow and branch | Own version, selected criterion IDs, ordered stable step references, condition/guard, expected outcome, adverse/return continuation |
| Precondition and fixture | Actor/authority, target/scope, state, clock/data version, prerequisite output identity, fixture construction, reset and cleanup owner |
| Step and action | Stable step ID, page/control identity, native binding, supported action and supplied value, expected feedback/state/effect; selectors alone are not meaning |
| Assertion | Stable assertion ID, criterion and step links, observable predicate, UI/service/manual boundary and native check reference; unsupported predicate and reason remain visible |
| Native test binding | File/export or test title, supported runner/version/config, mapped steps/assertions, fixture/reset and branch selection; many-to-many joins are valid |
| Run and artifact | Unique run and attempt IDs, exact source/model/flow/fixture/build revisions, tool/browser/environment, start/end, outcome, failure location and actual retained evidence |

A step sequence includes meaningful conditions and branch edges. Decide important refusal, no-result, conflict, unknown or recovery paths from the actual promise; do not invent every possible failure. Guard decisions must be controlled by fixtures or observed state, not by silently skipping assertions when controls are absent. Give each branch an isolated setup or an explicit dependency on a previous output. Do not make independent tests share mutable session order. Stop a dependent path if its prerequisite failed/has unknown effect; retain the cause and unvisited steps.

Agree deterministic data, identity, time and reset with the supplying owner. Fresh browser context clears browser-local state; it does not reset a database, daemon, queue or remote operation. Reset only owned fixtures/resources, verify its postcondition where needed and record cleanup failure separately. After timeout on a possibly effective operation, observe/reconcile using the native contract before retrying. A retry is a new retained attempt, not erasure of a failure.

## Native Playwright scope and execution boundary

Use the consumer's installed version and engineering entry. Check declarations/help before relying on APIs. Map meaningful steps into `test.step` with their IDs, use semantic locators and auto-retrying `expect` assertions, and include criterion/assertion IDs in titles or attachments. Keep domain setup in the existing fixture/Mock boundary. Use native fixtures, config, HTML/JSON reporters and trace capture where supported; do not write another locator engine, scheduler or generic flow interpreter.

| Playwright can assess, if explicitly exercised | Separate evidence still needed |
| --- | --- |
| Navigation, form input, clicks, visible state/text, focus, URLs and declared request/result bindings | Whether representative people discover the path and understand its meaning |
| A keyboard path or rendered viewport under named browser/data conditions | Comprehensive accessibility or all devices; taste, hierarchy and motion judgment need contextual observation |
| Supported service effect checks through an authorized test boundary | Durable concurrency, permissions or production outcomes beyond the boundary actually exercised |
| Screenshots, traces and native test status | Business value, long-term adoption or final user acceptance |

Bind real runs to explicit target, actor, data, allowed effects, output location and cleanup owner. Localhost alone does not establish safety: it may host live user resources. Prefer isolated candidates and owned fixtures. Existing authorization covers its stated reversible actions; request missing authority only for consequential/out-of-scope actions. Do not send real messages, purchase, publish, delete shared data, restart services or install tools just because the path mentions them. Use approved simulations or leave that branch blocked/manual with a reason. Read-only browser work may still load sensitive data; retain only necessary authorized evidence and protect credentials/context in traces and screenshots.

The complete [mapping example](examples/task-search/README.md) requires an explicitly selected disposable host and an installed Playwright Test dependency in the consuming project. The example supplies no product host or dependencies; source publication is not an executed browser test. Existing project browser scripts can remain the selected runner; preserve their identifiers/results and document any absent report association rather than forcing migration.

## Run records and outcomes

Preserve the native runner's raw statuses and attempts. For readable projections, distinguish planned/not-run, blocked prerequisite, running, passed, failed, skipped, interrupted and unsupported/manual; these are presentation descriptions, not the consuming system's native state enums. A skipped/blocked/unsupported assertion cannot count as passed. Native retries may produce a flaky result; retain all attempts and report the initial failure. Required missing artifact or assertion evidence leaves the corresponding claim unverified even when the process exited successfully.

Record a planned run separately from actual execution. Actual records need the selected revisions, environment, fixture/reset receipt, target and actor/scope, run/attempt time and status, per-step/assertion results and artifacts with media type, stable relative path/URI and integrity/retention when available. Link screenshot/trace to the exact step/assertion and attempt. Include console/network/effect evidence only when needed for the criterion. Never fabricate a run ID, screenshot path or expected result as an observed artifact. Redacted/unavailable/expired artifacts stay labeled; historical sources remain retrievable at their original revisions.

## Report projection

A report is a view over authoritative sources, bindings and actual runs. A Markdown page linking native reports is sufficient; no new platform is required. Derive views from the same records, preserving the current selection when following links:

| View | User can inspect |
| --- | --- |
| Need / requirement | Original promise and revision, contributing scenarios, satisfaction supported/contradicted/unverified with reasons |
| Scenario / branch | Actor, goal, preconditions, normal/adverse path, relevant page entry and version, execution and manual gaps |
| Step / assertion | Action and expected result beside actual result, predicate boundary and exact evidence; before/after or failure screenshots when retained |
| Run / attempt | Candidate/build, source/flow/fixture revisions, environment, all attempts including failures, native HTML/JSON/trace links and missing evidence |
| Finding | Original criterion, observation, interpretation, severity/consequence, owner/disposition and versioned recheck |

Offer links to original sources, exact candidate routes, native test code and actual artifacts. Clearly distinguish a runnable preview link from an evidence link. Do not create clickable links to files that were never produced. Show compact requirement/scenario rows first, with step details/screenshots on demand; preserve useful labels and accessible controls. A UI implementation, if later assigned, follows the project's interface standard.

## Coverage denominators

Obtain the selected requirement/criterion inventory independently from its canonical owner; obtain page/actions and native operations independently for completeness claims using [interaction coverage](../interaction-modeling/sources-and-coverage.md). A flow cannot prove its own omissions absent. Freeze scope, actor, channel, build, environment and applicable criteria before joining. Unselected/future scope can be excluded only with explicit reason; missing tests and blocked runs stay in the selected denominator.

Report separately: linked criteria / selected criteria; native-automatable criteria / selected criteria; executed criteria / selected criteria (also name eligible automated denominator if useful); evidence-supported criteria / selected criteria; visited branches / selected branches; and manual observations completed / selected manual observations. State mixed manual/automated criteria and all-of/alternative composition. One criterion requiring two assertions is supported only when its required assertions and branches have adequate evidence. Screenshots, test counts and clicks are not requirement denominators. A failed assertion may still have good evidence, so evidence presence and satisfaction are different measures. No run means zero executed/supported, even if every mapping exists. A partial scope is never whole-product coverage.
