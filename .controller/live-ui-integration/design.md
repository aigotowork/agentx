---
schema: document
status: active
owner: integration
last_updated: 2026-11-15
---

# Integration model

## Domain and dependency direction

Native participant/messaging/transcript and observation facts → native management/inspection projections and durable operation contracts → generated TS plus bounded consumer adapters → each existing UI system. The daemon owns durable facts; originating workers own Provider filesystem/process effects; a browser owns selection, drafts, view state and local preferences. Inspector-derived collection/read/recovery/report facts require explicit native ownership rather than browser mock state.

| Consumer | Required actual outcome | Decision boundary |
| --- | --- | --- |
| Agent space | Known offices/posts, exact current/stopped members, real communication/model records; deliberate Role launch, send, stop/resume and directory contact | Reuse completed office contracts; preserve scene/chat/identity drafts; remove invented people or Skill authority |
| Inspector | Actual sessions/evidence/events, coverage-aware statistics and Token observations; supported collection/analysis/recovery actions with real receipts | Existing assessment is stale about office supply; inspect current source before adding facts. Retained history starts at collection enable time; no backfill |
| Hosting | Both maintained UIs can read the selected scope over LAN and use one authoritative API | Prefer same-origin finite routes/assets; no unrestricted proxy, arbitrary shell/RPC or duplicate service identity |

## Invariants and acceptance

No display-ID-to-incarnation invention; current/stopped namesakes separate. Reset invalidates pending reads and target operations. Partial paging/discovery/collection is explicit. Message acceptance and model completion differ. Stop/resume/recovery cannot repeat uncertain effects. Preserve optional Provider facts and explicit unsupported states. UI concepts are retained only when native facts or implemented owned behavior support the user task; merge redundant navigation rather than reconstruct legacy Bots/Teams services.

Office scenarios reuse exact membership/global/legacy/unavailable, effective adopted/current Role differences, inherited entry, empty office, directory send, lost receipt/reload and actual reply. Inspector scenarios must distinguish live/closed data, absent/partial transcript, token units/coverage, collection enabled time, analysis jobs and sustained-error recovery guards. Each owner records concrete mappings and gaps before behavior.

## Open decisions

Inspector current selected retained feature-to-operation mapping; minimum coherent collection ledger and analysis execution; sustained-error recovery ownership and configuration; embedding routes/build provenance. Resolve from consumer/source evidence in Tasks before committing contracts. No approval gate is inferred from historical offline restrictions.


## Selected hosting and recovery mechanisms

Serve maintained existing applications under `/space/` and `/inspector/` on the selected daemon management listener, reusing same-origin `/api/v1` admission, scope and generation. Existing management root remains available; no proxy or extra business daemon. Each application supplies a fresh isolated build.mjs output; native packaging recursively embeds a bounded allowlisted bundle including original licenses/assets, never existing preview dist. Diagnostic/workbench embedding remains unchanged. Source input watches exclude generated dist/node_modules.

Recovery uses an atomic native mailbox continuation request to a still-current failed instance/error episode, with episode reservation, accepted message receipt and audit in one native write transaction. UI says request continuation/queued, never recovered on mailbox acceptance. Only successful native model observation establishes error clearance. Auto policy defaults disabled, continuous-error duration is configurable, and one episode cannot silently repeat an uncertain attempt. No process stop/restart is inferred from this mechanism.
