---
schema: task
status: active
agents: [{"scope":"/home/long/works/longwork/agentx/.agentx","generation":"a6195a5b-1dd4-4a6d-87d4-1f18f1b664fb","participant":{"canonical_name":"divona","incarnation_bits":4745316878394846208}}]
---
Repair baseline non-ignored Rust unit/integration test failures that appeared after fixing CI's fmt gate. Use isolated /tmp/agentx-ci-tests worktree off 804ef94. Coordinator is running cargo test --no-fail-fast in /tmp/agentx-ci-recovery with output at /tmp/agentx-ci-tests.log; use it for initial evidence, isolate env and limit jobs/threads for local resource use. Handle stale assertions after contract changes and genuine behavior regressions, preserving intended exact identities, security/authorization and existing callers. Linux owner nuzane covers ignored real-tool lifecycle tests and shared startup/relay fixes; Windows owner kavema covers Windows specifics; coordinate with them. Coordinator owns clippy, npm, MSRV, integration. Commit small batches and send veto hashes early; do not push main or touch shared checkout. Record remaining failures explicitly.
