---
schema: task
status: completed
agents: [{"scope":"/home/long/works/longwork/agentx/.agentx","generation":"a6195a5b-1dd4-4a6d-87d4-1f18f1b664fb","participant":{"canonical_name":"nuvu","incarnation_bits":4745316826458279513}}]
---
Read [delivery contract](AGENTS.md).

Own native cloud admin and scope API, store and exports. Deliver role/workspace capabilities, online-only typed launch and exact stop, poll/report operations with generation/lease fencing and idempotence. Publish wire shapes promptly in wire.md for peers. Own src/cloud/{admin,communication,catalog,control} and cloud router mounts. Coordinate shared mod.rs.

## Implemented and verified

The [wire contract](wire.md) links the declarations, routes and store. Admin capabilities distinguish a live daemon lease from legacy messaging heartbeats. The authenticated scope endpoints renew the installation lease, wait and claim atomically, and accept an idempotent exact report. Launch admission uses the current workspace catalogue and stop targets an exact incarnation; the store revalidates both before first claim. Lease loss distinguishes work that never left the queue from work that may have run. The same installation can reconcile a journaled claim under a replacement lease.

The [standalone Linux native test log](../../.tmp/cloud-daemon/api/native-tests.log) shows 11 passing tests across control, catalogue and communication. The [loopback HTTP result](../../.tmp/cloud-daemon/api/http-result.json) covers bearer separation, capability gating, bounded empty waits and early wake on submit, exact lease fencing, old-bearer wait revocation, outcome replay and receipt persistence across deployment restart. The [probe](../../.tmp/cloud-daemon/api/http_probe.py) used the actual deployment router and a disposable root. Both native OpenAPI exports (9 Admin paths and 14 Scope paths) matched the generated package input byte for byte. Integrated Cargo artifact parity and real daemon/Provider effects are owned by the controller and [acceptance](acceptance.task.md); no installed or live service was changed for these checks.

The embedded Management build consumes the generated SDK, creating a bootstrap cycle when new SDK symbols precede the native build. The [standalone exporter](../../.tmp/cloud-daemon/api/bootstrap.py) built the native declarations and actual API router before SDK generation. The authoritative export remains `agentx cloud-openapi`; the bootstrap is isolated evidence, not a second contract.
