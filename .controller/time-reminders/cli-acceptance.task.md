---
schema: task
status: completed
description: Independently verify time reminders against an isolated daemon.
agents: [{"scope":"/home/long/works/longwork/agentx/.agentx","generation":"a6195a5b-1dd4-4a6d-87d4-1f18f1b664fb","participant":{"canonical_name":"rubo","incarnation_bits":4745317738052926039}}]
---

# Isolated CLI acceptance

Read [implementation boundaries](implementation.task.md) and its linked approved plan. Own only `tests/time_reminders/` and this assignment file. Author a finite, repeatable Linux CLI acceptance against an explicit candidate binary and a temporary Scope; no live scopes, Provider, installation, or package build. Cover CLI grammar and results, inactive traffic and bounded timing under sustained native requests if practical, cancel, restart overdue and no duplicate, target replacement, legacy subscriptions, help. You may seed disposable fixture identity through the actual schema as needed; document exact evidence limits. Report script usage when ready; parent owns source and build. If candidate is not ready, finish authoring and hand off immediately.

The [acceptance runner](../../tests/time_reminders/acceptance.py) and [usage and evidence limits](../../tests/time_reminders/README.md) are ready for integration. Run `python3 tests/time_reminders/acceptance.py /absolute/path/to/candidate/agentx` once the integrated candidate is available.

Authoring verification: `python3 -B tests/time_reminders/acceptance.py --help` passed. The available project binary predates the reminder source, so the candidate suite was not run. Completion here records the script handoff; the parent owns full execution and the resulting evidence. Fixture isolation and the distinction between observed native load and provable queue saturation are documented with the runner.
