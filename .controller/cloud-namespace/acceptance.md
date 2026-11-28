---
schema: document
status: completed
owner: Isolated HTTPS integration verification
links: [./task.md, ../../tests/cloud_scope/README.md]
---

# Isolated Namespace acceptance

The [Task](task.md) owns the public contract. The existing `tests/cloud_scope` fixture owns the deployment, TLS transport, native process lifetimes and HTTP evidence; extend its shared Space runner with a second installation and retain older consumer regressions against the renamed contract. Only run against an explicitly handed-off candidate, on loopback, with fresh runtime roots, test credentials and a deterministic local model. No public service, real model endpoint or existing database is part of this lane.

| Starting state and action | Required evidence |
| --- | --- |
| Two enrolled hosts each scan a project named `shared`; the east project starts empty. | Both appear under their host in the browser, keep separate non-path identities, and can be opened as offices. Admin and Namespace paging and host filters agree; the empty project has no members. |
| Native east launches from a nested cwd; Space launches another east Agent with the selected project's role and a different nested cwd. | Both appear in the same exact east project only. The browser preselects that project, exposes its roles only, and opens an exact Transcript. Verify the native and cloud cwd independently of project membership. |
| Admin adds an existing deep directory and creates a new one; native daemon then restarts. | Browser operation receipts and public catalogue agree; original IDs and new IDs survive restart. An alias is only a parent root selector. |
| East daemon goes offline while west remains leased. | Public catalogue retains east's exact IDs and flips availability; same-name west stays online; a waiting roster reader wakes. |
| Namespace-only session reads a project and tries an Admin action; a retired route/client spelling is used. | Browser read path works without controls; the actual Admin operation is denied and has no effect. Old public paths and native enrollment flags are rejected inside the disposable fixture. |

The [final run](/tmp/agentx-ns-final-nODkXj-accepted/result.json) passed with embedded candidate SHA-256 `96e009cb5ea5a1630c6ade12f228394000f6852c30e53746b66d648a37a841fc`, Chrome 147.0.7727.101 and Codex CLI 0.171.0. It observed separate same-name offices, the empty west office, project-local roles, both native and managed members in the same east office with distinct nested cwd, command/Transcript continuity, explicit registration and creation persisting through restart, offline catalogue and lease notification, no roster invalidation on unchanged renewal, read-only actions denied, and retirement of all four old HTTP routes in the matrix. The fixture sends model traffic to a deterministic local model and cleans up its own services. Its TLS shim can log client EOF when Chromium cancels an outstanding poll during teardown.

Run from the repository root using a new evidence directory:

```sh
python3 -B tests/cloud_scope/shared_workbench.py --candidate /absolute/path/to/agentx --evidence /tmp/new-namespace-space
```

The browser uses a fixture-only leaf certificate pin; native and Python callers trust the private CA. Both host installations run on one Linux machine with different scope, project, Codex and home directories. This lane does not exercise different physical machines or public rollout.

Other checks in that run: 43 native cloud tests, 35 HTTP SDK tests, the isolated Management and Space browser fixtures, native OpenAPI parity for both generated artifacts, and the exact candidate container smoke passed. [Public cutover and rollback](rollout.md) records separate physical host evidence.

## Strict v2 follow-up

The [cleanup task](cleanup.md) supersedes the compact compatibility run. The acceptance matrix reuses the fresh loopback deployment and separates malformed input from a valid but unknown address.

| Boundary | Accepted case | Rejected case |
| --- | --- | --- |
| Deployment and Namespace | Both descriptors announce v2 on the existing /api/v1 routes. Provision and rotate short credentials, then replay explicit request keys across restart. | Reject old long credentials for startup, provisioning and rotation; reject a retired credential after rotation. An isolated old-format catalogue must be refused without migration. |
| Native and Space admission over HTTPS | A trusted v2 descriptor admits the real consumer; the same-name office, launch, Transcript, message and restart flows still work. | Advertise v1 on the authenticated Admin and Namespace descriptors through the private TLS fixture. The native direct client must refuse the Namespace descriptor and Space must reject either access mode before reading a roster. |
| Exact addresses and control | Short project IDs and Agent incarnations appear in registrations, catalogues and exact routes; an unknown well-formed address still follows the existing lookup rules. | Reject former long project links, filters and launch IDs, plus old UUID or numeric Agent incarnations. Existing claim, lease and generation fences keep their independent semantics. |

Integration handed off the frozen embedded candidate with SHA-256 `f23d882411aac02b0a057d281f0651fee671760c5d9fbd566aeff1e73702f331`. The [deployment](/tmp/agentx-cloud-v2-accept-sFNGevLD/deployment/result.json), [communication](/tmp/agentx-cloud-v2-accept-sFNGevLD/communication/result.json), and [Space API](/tmp/agentx-cloud-v2-accept-sFNGevLD/space/result.json) companion suites passed with their own isolated state and cleanup.

The [final shared-workbench run](/tmp/agentx-cloud-v2-accept-sFNGevLD/shared-workbench-r3/result.json) passed with Chrome 147.0.7727.101 and Codex CLI 0.172.0. It retained the two-installation launch, Transcript, message, offline catalogue and restart evidence. The native direct client and both Space access modes rejected protocol v1. Both retired deep links returned HTTP 400 without any API read. Native enrollment rejected the long credential without altering its configuration. Station and deployment runtime roots were removed and the private TLS listener closed. The run did not access production resources.

Two harness assertions needed correction against the frozen binary. The [first run](/tmp/agentx-cloud-v2-accept-sFNGevLD/shared-workbench/result.json) expected a page error where the shell route returns HTTP 400 before serving HTML. The [second run](/tmp/agentx-cloud-v2-accept-sFNGevLD/shared-workbench-r2/result.json) established both retired links returned 400, then failed because the fixture expected the token format diagnostic while the native file reader rejected the retired credential on length. The fixture now checks rejection at credential validation and verifies the enrollment file is unchanged.
