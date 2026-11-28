---
schema: task
status: completed
owner: Cloud Namespace integration
agents: [{"scope":"/home/long/works/longwork/agentx/.agentx","generation":"a6195a5b-1dd4-4a6d-87d4-1f18f1b664fb","participant":{"canonical_name":"zeru","incarnation_bits":4745317707326189698}},{"scope":"/home/long/works/longwork/agentx/.agentx","generation":"a6195a5b-1dd4-4a6d-87d4-1f18f1b664fb","participant":{"canonical_name":"luvi","incarnation_bits":4745317707326222879}},{"scope":"/home/long/works/longwork/agentx/.agentx","generation":"a6195a5b-1dd4-4a6d-87d4-1f18f1b664fb","participant":{"canonical_name":"garo","incarnation_bits":4745317707326246617}}]
---

# Retired compatibility paths

Cloud v2 uses only WS + ten ASCII alphanumerics for project IDs and 12 ASCII alphanumerics for cloud Agent incarnations and bearers. Scope generation, lease and claim fences and explicit request keys retain existing contracts. Keep v1 HTTP path names; both cloud descriptors report protocol version 2. New deployment database format; expose no runtime translation. Distinct modules own Rust validation and storage, browser/SDK consumers, and independent HTTPS verification.

- Native lane: format and version authority in src/cloud; strict project inputs, instance and token validation; fresh schema and canonical host registrations; focused native tests.
- Consumer lane: SDK source and Web; strict format and version admission, one cloud reader path, remove internal UIUX shims. Integration generates SDK sources from native; no generated edits by consumer lane.
- Acceptance lane: tests/cloud_scope, isolated HTTPS fixtures and independent runtime evidence; no live work.
- Integration: repo debt inventory and bounded removals, assemble and verify candidate, then fresh public test deployment with old resources retained for rollback. Recheck current Namespace and both host journals before choosing cutover.

## Native contract and SDK handoff

- [Rust identifiers](../../src/cloud/identifiers.rs) own `^WS[A-Za-z0-9]{10}$` for Workspace IDs and `^[A-Za-z0-9]{12}$` for Agent incarnations and Admin/Namespace bearers. Comparisons are exact and case-sensitive; there is no workspace alias resolution. Configured project root aliases remain valid only as Add Workspace inputs. Message DTOs still carry the reserved synthetic operator address `(~space, operator, ~human)`; it cannot stand in for a real Agent or be used as a bearer. Explicit request, installation, generation, lease and claim keys keep their existing validation and semantics.
- Both native descriptors (Admin overview and Namespace descriptor) emit and require protocol version 2 while the HTTP paths remain under `api/v1`. The generated schema can constrain v2 fields, but clients still verify protocol admission before acting.
- The [deployment catalogue and Namespace DB](../../src/cloud/storage.rs) use independent SQLite format fences (catalogue 2, Namespace 5); a fresh Namespace transaction creates all communication, Transcript and control tables before publication. Unsupported DBs are rejected without upgrade. The separate local control journal keeps its existing operation/lease evidence and uses a canonical project path key for new registrations, scoped by the existing journal namespace. A stored canonical export root remains the confinement anchor, independent of root alias changes. The retired path-derived registration table is left intact for rollback and is not read at runtime.
- Native cases cover rejection of retired identifiers, exact membership and control receipts, unsupported SQLite formats, and project registration replay after an export alias rename. Integration owns generated SDK changes and isolated HTTPS acceptance.

## Repository inventory

| Area | Decision and evidence |
| --- | --- |
| Cloud Rust, SDK, Space, CLI | Remove aliases and v1 readers at the v2 boundary; generated files follow the Rust exporters. No conversion of the old test database and no general old-peer fallback. |
| Workbench and Observation type generators | Delete both uninvoked `api/derive.py` programs. The build and [public SDK instructions](../../packages/agentx-client/README.md) use native OpenAPI. Correct the stale Management examples that still asked users to run the removed generator. |
| Inspector generated types | Delete the unimported `host/native.ts` output. The live inspector uses `host/live/native.ts` from the SDK; demo data is a separate, actively used surface. |
| Space UIUX compatibility modules | Switch in-repository callers to the UIUX export or owning geometry module, then delete trivial forwarding entries. Preserve published style and embedding queries that have current tests or documented callers. |
| Local Management and Domain SDK | Preserve selected native listener negotiation and fallback readers; standalone browsers and older deployed listeners are still documented. Retire those branches only after every supported listener advertises their replacements. Unknown/legacy membership in a persisted record is a data fact. |
| Native local DB, CLI and daemon | Preserve source-local migration/projection, retired-role endpoint reset, and explicit incompatible-daemon recovery. Their live callers and migration fixtures hold durable data and exact process ownership. |
| Provider and Relay integrations | Preserve provider phase handling and pending mailbox replay, Relay's peer capability and process fences, and its pairing keys; retirement requires a documented minimum provider or Relay version and a coordinated peer cutover. |
| Release workflow | Keep checks for explicitly supported historical image republication; its tag checkout is a current user-facing job and is separate from cloud v2 runtime support. Preserve archived delivery evidence in Git. |

Native production source is ready for the integration build. The OpenAPI changes are the descriptor version bounds, compact token/Workspace/incarnation patterns, and the documented `~human` allowance on the shared message Agent view. Regenerate from this source snapshot; no generated files were written in the native lane. The Space shell rejects malformed explicit identity on both URL spellings. Focused Linux native verification passed: [48 tests](../../.tmp/cloud-namespace/native-v2.7q1Fbc/native-final.log) from `cargo test --locked --bin agentx cloud::` in a source snapshot with empty embedded browser asset tables. This exercises native routing, OpenAPI, SQLite, the worker and peer fences; it does not validate assembled Web assets or deployment. `cargo fmt --check` and the owned Rust diff whitespace check also passed.

## Verification and deployment gates

The full embedded build emitted byte-identical Admin and Namespace OpenAPI with the packaged schemas. The SDK suite passed 37 tests; the full checkout passed 48 cloud Rust tests. Both actual Web bundles passed the isolated Cloud Space and Management Chrome fixtures, alongside Space route and geometry tests and the candidate's container smoke. The independent HTTPS rig also verifies actual Codex subprocesses against a deterministic local model and distinct worker installations. Only a passing result for the exact binary permits activation.

The public preflight found longser and xser01 and zero Agent, message, control, and fact records in the old test Namespace. Both workers had zero explicit project registrations and no unreported cloud operation. The activation script repeats the preflight before touching the proxy. The v2 candidate is staged at loopback `18758` with a new volume, new credentials and its own runtime generation. The existing listener at `18757` and both old enrollment files are rollback resources; only the AgentX virtual host and selected enrollments may change.


## Accepted public outcome and rollback

[Independent HTTPS evidence](/tmp/agentx-cloud-v2-accept-sFNGevLD/shared-workbench-r3/result.json) passed on binary SHA-256 `f23d882411aac02b0a057d281f0651fee671760c5d9fbd566aeff1e73702f331`. Native and both browser audiences refused protocol 1; old Workspace and Agent links returned 400 without resource traffic. The [same candidate's public verification](../../.tmp/cloud-namespace/v2/public-result.json) observed protocol 2, both host leases, the automatically discovered [longser / agentx project](https://agentx.longalong.cn/namespaces/personal/space/?host=longser&workspace_id=WSH0zNdctPlX), and correct browser permissions. Old Scope routes returned 404; the previous long Workspace ID returned 400 from the page and member API.

The public deployment is Compose project `agentx-cloud-v2` with `agentx-cloud-v2_data`, image `agentx-cloud:compact-v2-f23d882411aa` (Docker config digest `sha256:e84065ba8ab8398064f7ff912f7ed28cbf48b5a3a784cd57f9e4fb0ac85c4530`), deployment ID `d73b8901-b2f1-4062-ab4f-74dd26e13cff`, and `personal` generation `d492dc08-d51d-4a73-89fc-d8b744f32ac4`. Read the new Admin and Namespace tokens, respectively, from `/opt/agentx-cloud-v2/.env` and `/opt/agentx-cloud-v2/personal.token` on `root@192.168.10.3` (mode 0600). Token values are excluded from the evidence.

If rollback is needed, run `bash /opt/agentx-cloud-v2/switch-proxy.sh rollback` on the deployment host to restore the saved Caddy fragment and upstream `127.0.0.1:18757`. The previous container `agentx-cloud-namespace_cloud_1` and volume `agentx-cloud-namespace_data` remain available. Restore the original enrollment and executable from `~/.local/lib/agentx-longalong/before-cloud-v2-f23d882411aa` on longser and `/opt/agentx-longalong/before-cloud-v2-f23d882411aa` on xser01; restart only their `agentx-longalong.service` (user service on longser, system service on xser01). Both local runtime databases and project files remain in place. No current binary opens the rollback volume.
