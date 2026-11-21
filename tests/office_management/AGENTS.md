# Independent office acceptance

Read [case map](README.md), [office design](../../.docs/agent-management/offices.md), [native public DTOs](../../src/management/contract.rs) and [owned Task](../../.controller/office-management/tasks/acceptance.md). Only tests and that Task/[evidence](../../.controller/office-management/evidence.md) are owned here. Reuse [existing management fixtures](../agent_management/AGENTS.md) without altering prior cases or receipts.

Only run an explicitly handed-off immutable candidate. No builds, paid Providers, managed database clients or user-service changes. Config files and candidate-generated real hooks create participants. HTTP/browser receipts prove public behavior; storage corruption and exact KV/event zero-write require separately owned native witnesses. Never label missing witnesses accepted. Every execution uses a new result directory and exact disposable-process cleanup.
