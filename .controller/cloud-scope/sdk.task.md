---
schema: task
status: completed
agents: [{"scope":"/home/long/works/longwork/agentx/.agentx","generation":"a6195a5b-1dd4-4a6d-87d4-1f18f1b664fb","participant":{"canonical_name":"vani","incarnation_bits":4745316805032058411}}]
---
Read [the integration contract](AGENTS.md).
Implement generated HTTP SDK and admin CLI per shared contract. Own packages/agentx-client and cloud emitted specs. Coordinate exports with Web and both spec producers.

## Implementation model

The native deployment and communication exporters remain authoritative for schemas and operation names. The existing package generator projects each selected exporter into a separate generated directory. The browser entry binds a deployment-root admin client and a separate communication client at the complete scope API base; each sends only its own bearer, disallows redirects, and accepts HTTP only on loopback.

The Node CLI adds the `admin` command group over the public admin methods. Before dispatch, create/rotate retain the exact deployment URL and observed deployment ID, scope, request identity, expected version when applicable, and proposed credential in a private exclusive file. Status and normal receipts omit credentials; `admin pending reveal` explicitly discloses the proposed token. Explicit `admin pending replay` checks the current deployment ID and reuses the same retained intent. No retry, version refresh, or token replacement occurs automatically.

## Handoff and evidence

[Package usage and regeneration](../../packages/agentx-client/README.md#cloud-deployment-and-scopes) document the public browser methods, generated type maps and CLI grammar. Native scope paths are relative to the full scope API base; the SDK also accepts `scope_url` directly. Admin Agent pages expose the optional host filter.

Both checked-in OpenAPI files matched the respective native Rust document outputs (admin: 6 paths; scope: 11 paths). The standalone bootstrap compiled the actual modules and called their native document functions before the full application build. Generator integration exposed in-place projection of write-only fields; generation now gives the projector a clone and associates native input aliases with the writable TS variants. The on-disk admin spec also matches the deployment owner's final export, including the original token constraints.

The complete TypeScript build, 29 package tests and diff whitespace check passed. The new tests cover bearer separation, redirect refusal configuration, exact integer and identity handling, host-filtered pagination, credential redaction, and bounded loopback CLI loss/replay/reveal including mismatched URL or replacement deployment ID. Web and independent acceptance owners received the compiled SDK and grammar. Native process deployment, actual cross-host Provider send/reply, and integrated Web/TUI evidence belong to the [acceptance task](acceptance.task.md). No running service or installed binary was changed.

For the joined Cargo candidate used by acceptance, regenerated Admin from that executable; only the Cargo package description differed from the bootstrap. The Scope document matched unchanged. The [acceptance tarball](../../.tmp/cloud-scope/acceptance/sdk-final.vYrUqf/agentx-client-0.1.0.tgz) was built in an isolated package directory and contains both compiled cloud clients and the CLI. Its shipped and checked-in OpenAPI documents exactly match both exports from the joined candidate. SHA-256: `2115475d0598902ca0f6ed61d82afb4dc2abb236650331b102e44db8ff327d06`.
