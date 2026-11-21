---
schema: task
description: Independently validate HTTP and browser management outcomes on a separately built candidate and isolated fixtures.
status: completed
---

# Acceptance

Read team/design. Own `tests/agent_management/` and `evidence.md` in the team directory. Prepare independent public-contract cases and a local Provider fixture before candidate availability. Do not open/write managed SQLite in product clients; test fixtures may seed through supported CLI/hooks. Verify launch/status/messages/transcript/stop/history/resume, idempotency/conflicts, stale references, scope isolation, request/response bounds, browser reload continuity and process cleanup. Candidate build is integration-owned. Do not spawn paid Agents for tests or alter live resources. Use only the provided candidate and disposable scopes. Report substantive failures with reproducible inputs and wait for repaired candidate rather than self-approving untested fixes.

Preparation and run status: [independent evidence](../evidence.md). Authored case map and runner: [tests](../../../tests/agent_management/README.md).

Completed on immutable r4 with native M01–M11 passing in `verification/pula-06` and actual browser B00–B07 passing in `verification/pula-07` under the same retained candidate SHA-256. Prior failures and harness corrections remain in evidence. Both final runs cleaned all owned resources and left the candidate unchanged. Scope and unexecuted capabilities are explicit in the evidence; integration owns Delivery acceptance and any installed/live rollout.
