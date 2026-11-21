---
schema: delivery
description: Make explicit environment file configuration work consistently for real local AgentX invocations and child launches.
status: completed
stages:
  implementation:
    status: completed
    tasks: [./tasks/native.md]
  outcome:
    status: completed
    tasks: [./tasks/acceptance.md]
    depends_on: [{stage: implementation, condition: coherent callable source and authored case release}]
---

# Unified local environment loading

The user authorized planning and immediate implementation of the [accepted Proposal](../../.docs/proposals/environment-loading.md). Build one coherent early loading contract, persist/configure it through the ordinary interfaces, retain existing source compatibility and demonstrate actual local operation. [Controller](controller.md) owns assignments and final artifact receiving; original Tasks own work state.

Current disposition: native implementation, 20 new authored cases, a strengthened workbench case and named model/usage updates are coherently released. [Detailed native design/usage](../../.docs/architecture/environment.md) and [exact native release](native-release.json) are the source-of-truth receiving inputs. Independent execution/package/outcome acceptance and controller promotion are complete.

The first candidate exposed one substantive passive-help defect: root `--help` could trigger the update-check side effect and create a missing scope. Native supplied the narrow correction; acceptance reran the affected cases, rebuilt the fixed package, completed cleanup, and released the exact artifact. The controller then promoted those bytes and cut over the project daemon/workbench coherently.

## Accepted local outcome and project handoff

Independent acceptance completed 54 native checks plus the bounded ENV-01–ENV-12 and INS-01 scenarios. The final artifact is `agentx 0.7.31`, 25,586,200 bytes, SHA-256 `34d9f0e54e6b9f83af5ba7095abad6f85777695ae464b539b42e95c5d5a38f37`; the exact package and cleanup evidence remain in [acceptance evidence](evidence.md) and [acceptance release](/tmp/agentx-environment-loading-acceptance-20261110/acceptance-release.json).

The verified bytes were promoted to the project [`.bin/agentx`](../../.bin/agentx). The project daemon runs from `/home/long/works/longwork/agentx/.agentx` as PID `177054`, and the collaboration workbench runs as PID `177063` on port `18947`; HTML and connection checks returned HTTP 200 at [the LAN workbench](http://192.168.2.178:18947/). The controller cutover receipt is [/tmp/agentx-environment-loading-live-cutover-20261110/controller-cutover.json](/tmp/agentx-environment-loading-live-cutover-20261110/controller-cutover.json).

The accepted support boundary is Linux local finite execution, including the documented environment-file selectors and bounded workbench HTTP bootstrap. The installed `/home/long/.local/bin/agentx`, the separate older LAN demo, and unrelated user services were preserved. Browser rendering, universal Provider/OS/window support and remote secret distribution remain outside this Delivery.

## Work model

1. Native refines source selection, bootstrap/scope, overwrite and existing-source seams, publishes native contracts/module skeleton and updates the environment design/usage page; it then fills behavior and authors meaningful cases.
2. Independent acceptance prepares disposable scenarios and preserves the current artifact/user-runtime identities concurrently. Execution begins only after an exact coherent source/case release; selected dependency lock resolution can occur earlier on a released manifest region.
3. Acceptance runs the smallest relevant native/static/product checks and one required exact-toolchain `just package`, returns a fixed usable artifact, actual limitations and cleanup. Actual defects return to the native owner within its original Task. Controller receives/promotes bytes and keeps the project runtime coherent.

## Completion conditions

An actual packaged CLI can select ordered files via ENV/CLI/TOML; explicit source precedence, overwrite/empty behavior, relative project paths, bootstrap scope selection and scoped redirect refusal behave as documented. Explicit bad inputs fail before managed state/child creation and do not leak values. Configuration is inspectable with source information, help/version remain passive, legacy env and authorized stdio direnv remain correct, and a real local child receives the environment in meaningful foreground/background/nested compositions without paid Provider work. Do not claim every Provider/OS/window system from one Linux fixture.

The source/current-model/usage entries must distinguish actual support from target. Disposable scopes, processes and ports are cleaned; the user runtime remains preserved or intentionally handed over coherently. Source Task closure is not Delivery acceptance. No profile system, new UI, broad refactor, remote environment transport or universal secrets platform is needed for this outcome.
