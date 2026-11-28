---
schema: task
status: active
agents: [{"scope":"/home/long/works/longwork/agentx/.agentx","generation":"a6195a5b-1dd4-4a6d-87d4-1f18f1b664fb","participant":{"canonical_name":"nuzane","incarnation_bits":4745316876342861777}}]
---
Repair the linux CI failures for public AgentX. Use GitHub run 44959821752 and /tmp/agentx-ci-failed.log. Preserve public functionality and tests. Use your own isolated git worktree at /tmp/agentx-ci-linux from e761c46. Commit on your own branch there, send veto the commit hashes as soon as each is ready. Do not push main or edit the shared checkout. Diagnose subsequent CI failures in your lane through green. Windows owns cross-platform compile/build and Windows-specific behavior. Linux owns provider lifecycle, relay and other behavior regressions shared across platforms. Coordinator handles fmt, clippy, MSRV, npm audit and integration. Avoid formatting unrelated code. Record any verification limits.
