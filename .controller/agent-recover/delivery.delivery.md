---
schema: delivery
status: completed
owner: agentx
description: Let operators explicitly continue live local Agents after a recoverable model request failure.
stages: {implementation: {status: completed, tasks: [./tasks/implementation.task.md]}}
---

# Agent recover

## Outcome

Implement the accepted [AgentX recover design](../../.docs/proposals/agent-recover.md). Operators can name exact Agents, filter with existing labels, use a searchable multi-select in a terminal, or explicitly use `--all`. Only live local Agents with a current native model-request error are candidates. The command submits one durable request for that exact unresolved episode; the registered Codex PTY host submits it only after exact host and prompt-safety evidence. A stopped Agent, changed episode, unsupported host, approval prompt, typed input or uncertain prior submit is never reported as recovered.

## Ownership and boundaries

One implementation Task owns the coupled CLI, native recovery request and receipt, exact episode admission and managed Codex delivery gate because their shared idempotency and effect result must be designed together. It records focused native/CLI scenario cases and executes the minimum affected checks. The work does not add automatic retry, status clearing, Provider error-string parsing, resume behavior, HTTP management routes or a new service.

## Acceptance

For a disposable managed Codex participant, a reported failed request plus a later Stop can be selected and produces exactly one continuation in the same session after ready/empty/no-approval/no-recent-input checks. A later successful request clears the existing model error. Replaying a command, concurrent duplicate episode, same-name successor, changed error, in-flight request, exit, approval, occupied input and unsupported terminal host cannot produce a second or unsafe submit. Exact names, label AND filtering before interactive selection/--all, current-workspace default, all-workspace --all, cancellation and partial batch failures have truthful results. Delivery submission remains distinct from Agent recovery evidence.

## Release boundary

Source completion, isolated candidate execution, installed executable replacement and running daemon/Agent host adoption are separate. Preserve every existing workspace change and live user service.

## Verified Linux outcome

The [isolated debug candidate archive](../../.artifacts/agent-recover/agentx.zst) is AgentX 0.7.31, build `native-build-v1:119860e4-2e86-4650-871a-84f057236a3f`, SHA-256 `d01e30c2042f3636ebe7696e57da9a2f310b48c088cce17bac5f46d845afb3d3`. The [controlled process case](../../src/daemon/acceptance_tests.rs#L338) used the product launcher, real daemon IPC and hooks, a live headless PTY, and a local Codex substitute. It proved exclusion before Stop, label intersection, no input into an occupied composer, one attempt for repeated CLI requests, one continuation in the original process/session, no mailbox-cursor advance, and clearance only after a subsequent successful model request. Twelve focused unit cases cover parser behavior, picker behavior, launcher environment and native model error transitions.

This run does not claim a paid Provider request, physical interactive-picking end-to-end, all transport-loss and concurrent interleavings, other operating systems, or installed/live adoption. Installed executables, running daemons and user Agent hosts were left alone. Disposable fixture processes and the temporary diagnostic copy were cleaned up.
