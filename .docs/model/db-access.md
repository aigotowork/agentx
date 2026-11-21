---
schema: document
status: active
owner: architecture
last_updated: 2026-11-17
description: Current production database dependencies and owners.
links: [./README.md, ./db-access.json, ./generate-db-access.py, ../architecture/acceptance.md]
---

# Production database access inventory

The current working tree contains 432 Rust source files; 51 external test files and inline test items are excluded. The inventory covers 102 affected Rust/resource files and 2086 syntactic candidates/references. It records 9 managed constructor sites and 5 Provider constructor sites. A constructor function reference is a site, not necessarily an executed open; in-memory constructors in the persistence adapter are included.

[JSON evidence](db-access.json) retains every candidate, source line, source hash, domain owner and per-site database boundary, plus the exact scanner bytes/hash. [Generator](generate-db-access.py) reuses the integration-owned scanner; run `python3 .docs/model/generate-db-access.py` to regenerate against the current working tree. No database, live data scope or Provider is opened. Multiple kinds at one line are separate dependency evidence, not separate operations.

## Boundary decisions

- Provider session lookups read external OpenCode/Kilo storage through the owning read-only adapter; managed AgentX storage remains daemon-owned.
- Managed AgentX KV reads remain daemon-owned. Read-only access does not make a managed database a Provider exception.
- `transcript/{hermes,opencode}.rs` and the embedded Hermes Python `state.db` reads are external Provider adapters. Preserve their explicit read-only contracts.
- AgentX archives are managed product data: archive connections belong to daemon maintenance/read handlers, even when opened read-only. Clients cannot select an arbitrary database path.

## Capability ownership

| Owning capability | Affected files | Constructor sites | Candidates/references |
| --- | ---: | ---: | ---: |
| CLI / product clients | 23 | 1 | 279 |
| Events / product clients | 6 | 2 | 119 |
| Executable composition / product clients | 1 | 1 | 3 |
| External transcript adapter / product clients | 3 | 3 | 61 |
| Messaging / consistency | 10 | 0 | 160 |
| Participant identity / consistency | 25 | 0 | 632 |
| Persistence / consistency | 14 | 4 | 443 |
| Provider / runtime hosts | 13 | 3 | 179 |
| Relay / product clients | 7 | 0 | 210 |

## Complete affected-file map

The owner is the source responsibility, not proof of current daemon composition. Paths link to current source; line numbers and content hashes in JSON refer to the recorded source snapshot. All candidate-only files remain in this map so migration cannot focus exclusively on explicit constructors.

| Source | Boundary | Owner | Constructor source lines | Sites |
| --- | --- | --- | --- | ---: |
| [src/agents/attachments_store.rs](../../src/agents/attachments_store.rs) | managed | Participant identity / consistency | — | 36 |
| [src/agents/binding.rs](../../src/agents/binding.rs) | managed | Participant identity / consistency | — | 51 |
| [src/agents/claude_store.rs](../../src/agents/claude_store.rs) | managed | Participant identity / consistency | — | 21 |
| [src/agents/claude_store/identity.rs](../../src/agents/claude_store/identity.rs) | managed | Participant identity / consistency | — | 10 |
| [src/agents/claude_store/output.rs](../../src/agents/claude_store/output.rs) | managed | Participant identity / consistency | — | 20 |
| [src/agents/claude_store/stop.rs](../../src/agents/claude_store/stop.rs) | managed | Participant identity / consistency | — | 33 |
| [src/agents/cleanup_store.rs](../../src/agents/cleanup_store.rs) | managed | Participant identity / consistency | — | 19 |
| [src/agents/context_store.rs](../../src/agents/context_store.rs) | managed | Participant identity / consistency | — | 7 |
| [src/agents/copilot_store.rs](../../src/agents/copilot_store.rs) | managed | Participant identity / consistency | — | 31 |
| [src/agents/hermes_store.rs](../../src/agents/hermes_store.rs) | managed | Participant identity / consistency | — | 34 |
| [src/agents/hook_store.rs](../../src/agents/hook_store.rs) | managed | Participant identity / consistency | — | 60 |
| [src/agents/identity.rs](../../src/agents/identity.rs) | managed | Participant identity / consistency | — | 18 |
| [src/agents/invocation_binding.rs](../../src/agents/invocation_binding.rs) | managed | Participant identity / consistency | — | 11 |
| [src/agents/launch_relations.rs](../../src/agents/launch_relations.rs) | managed | Participant identity / consistency | — | 26 |
| [src/agents/launch_store.rs](../../src/agents/launch_store.rs) | managed | Participant identity / consistency | — | 27 |
| [src/agents/launch_title.rs](../../src/agents/launch_title.rs) | managed | Participant identity / consistency | — | 1 |
| [src/agents/lifecycle.rs](../../src/agents/lifecycle.rs) | managed | Participant identity / consistency | — | 21 |
| [src/agents/lifecycle_resume_store.rs](../../src/agents/lifecycle_resume_store.rs) | managed | Participant identity / consistency | — | 33 |
| [src/agents/lifecycle_store.rs](../../src/agents/lifecycle_store.rs) | managed | Participant identity / consistency | — | 73 |
| [src/agents/model_request_store.rs](../../src/agents/model_request_store.rs) | managed | Participant identity / consistency | — | 5 |
| [src/agents/names.rs](../../src/agents/names.rs) | managed | Participant identity / consistency | — | 32 |
| [src/agents/records.rs](../../src/agents/records.rs) | managed | Participant identity / consistency | — | 1 |
| [src/agents/service.rs](../../src/agents/service.rs) | managed | Participant identity / consistency | — | 1 |
| [src/agents/service_store.rs](../../src/agents/service_store.rs) | managed | Participant identity / consistency | — | 35 |
| [src/agents/workspaces.rs](../../src/agents/workspaces.rs) | managed | Participant identity / consistency | — | 26 |
| [src/cli/client.rs](../../src/cli/client.rs) | managed | CLI / product clients | — | 1 |
| [src/cli/commands/config.rs](../../src/cli/commands/config.rs) | managed | CLI / product clients | — | 1 |
| [src/cli/commands/kill.rs](../../src/cli/commands/kill.rs) | managed | CLI / product clients | — | 1 |
| [src/cli/commands/list.rs](../../src/cli/commands/list.rs) | managed | CLI / product clients | — | 1 |
| [src/cli/commands/listen.rs](../../src/cli/commands/listen.rs) | managed | CLI / product clients | — | 1 |
| [src/cli/commands/listen_client.rs](../../src/cli/commands/listen_client.rs) | managed | CLI / product clients | — | 2 |
| [src/cli/commands/send.rs](../../src/cli/commands/send.rs) | managed | CLI / product clients | — | 3 |
| [src/cli/commands/stop.rs](../../src/cli/commands/stop.rs) | managed | CLI / product clients | — | 6 |
| [src/cli/output.rs](../../src/cli/output.rs) | managed | CLI / product clients | — | 9 |
| [src/cli/workspace.rs](../../src/cli/workspace.rs) | managed | CLI / product clients | — | 1 |
| [src/collaboration/store.rs](../../src/collaboration/store.rs) | managed | CLI / product clients | — | 18 |
| [src/db/claude_actors.rs](../../src/db/claude_actors.rs) | managed | Persistence / consistency | — | 25 |
| [src/db/collaboration.rs](../../src/db/collaboration.rs) | managed | Persistence / consistency | — | 25 |
| [src/db/events.rs](../../src/db/events.rs) | managed | Persistence / consistency | — | 17 |
| [src/db/instances.rs](../../src/db/instances.rs) | managed | Persistence / consistency | — | 83 |
| [src/db/kv.rs](../../src/db/kv.rs) | managed | Persistence / consistency | — | 11 |
| [src/db/launch_relations.rs](../../src/db/launch_relations.rs) | managed | Persistence / consistency | — | 5 |
| [src/db/message_acceptance.rs](../../src/db/message_acceptance.rs) | managed | Persistence / consistency | — | 10 |
| [src/db/mod.rs](../../src/db/mod.rs) | managed | Persistence / consistency | 201 (managed), 313 (managed), 576 (managed), 792 (managed) | 49 |
| [src/db/notify.rs](../../src/db/notify.rs) | managed | Persistence / consistency | — | 16 |
| [src/db/post_commit.rs](../../src/db/post_commit.rs) | managed | Persistence / consistency | — | 8 |
| [src/db/reset.rs](../../src/db/reset.rs) | managed | Persistence / consistency | — | 21 |
| [src/db/sessions.rs](../../src/db/sessions.rs) | managed | Persistence / consistency | — | 49 |
| [src/db/shared_runtime.rs](../../src/db/shared_runtime.rs) | managed | Persistence / consistency | — | 9 |
| [src/db/subscriptions.rs](../../src/db/subscriptions.rs) | managed | Persistence / consistency | — | 115 |
| [src/events/filters.rs](../../src/events/filters.rs) | managed | Events / product clients | — | 3 |
| [src/events/history_store.rs](../../src/events/history_store.rs) | managed | Events / product clients | 80 (managed), 199 (managed) | 24 |
| [src/events/listening_store.rs](../../src/events/listening_store.rs) | managed | Events / product clients | — | 33 |
| [src/events/observer_store.rs](../../src/events/observer_store.rs) | managed | Events / product clients | — | 31 |
| [src/events/predicate.rs](../../src/events/predicate.rs) | managed | Events / product clients | — | 2 |
| [src/events/store.rs](../../src/events/store.rs) | managed | Events / product clients | — | 26 |
| [src/inspection/recovery.rs](../../src/inspection/recovery.rs) | managed | CLI / product clients | — | 5 |
| [src/inspection/store.rs](../../src/inspection/store.rs) | managed | CLI / product clients | — | 25 |
| [src/integrations/hooks/claude/actor.rs](../../src/integrations/hooks/claude/actor.rs) | managed | Provider / runtime hosts | — | 7 |
| [src/integrations/hooks/claude/native.rs](../../src/integrations/hooks/claude/native.rs) | managed | Provider / runtime hosts | — | 22 |
| [src/integrations/hooks/codex.rs](../../src/integrations/hooks/codex.rs) | managed | Provider / runtime hosts | — | 1 |
| [src/integrations/hooks/copilot.rs](../../src/integrations/hooks/copilot.rs) | managed | Provider / runtime hosts | — | 1 |
| [src/integrations/plugins/hermes/__init__.py](../../src/integrations/plugins/hermes/__init__.py) | provider | Provider / runtime hosts | 310 (provider), 1357 (provider) | 7 |
| [src/main.rs](../../src/main.rs) | managed | Executable composition / product clients | 71 (managed) | 3 |
| [src/management/activity.rs](../../src/management/activity.rs) | managed | CLI / product clients | — | 8 |
| [src/management/human.rs](../../src/management/human.rs) | managed | CLI / product clients | — | 21 |
| [src/management/relations.rs](../../src/management/relations.rs) | managed | CLI / product clients | — | 19 |
| [src/management/store.rs](../../src/management/store.rs) | managed | CLI / product clients | — | 45 |
| [src/messaging/bundle_store.rs](../../src/messaging/bundle_store.rs) | managed | Messaging / consistency | — | 33 |
| [src/messaging/delivery/mod.rs](../../src/messaging/delivery/mod.rs) | managed | Messaging / consistency | — | 1 |
| [src/messaging/delivery_service.rs](../../src/messaging/delivery_service.rs) | managed | Messaging / consistency | — | 1 |
| [src/messaging/delivery_store.rs](../../src/messaging/delivery_store.rs) | managed | Messaging / consistency | — | 22 |
| [src/messaging/formatting.rs](../../src/messaging/formatting.rs) | managed | Messaging / consistency | — | 1 |
| [src/messaging/inbox_store.rs](../../src/messaging/inbox_store.rs) | managed | Messaging / consistency | — | 37 |
| [src/messaging/routing.rs](../../src/messaging/routing.rs) | managed | Messaging / consistency | — | 1 |
| [src/messaging/service.rs](../../src/messaging/service.rs) | managed | Messaging / consistency | — | 1 |
| [src/messaging/service_store.rs](../../src/messaging/service_store.rs) | managed | Messaging / consistency | — | 30 |
| [src/messaging/submission.rs](../../src/messaging/submission.rs) | managed | Messaging / consistency | — | 33 |
| [src/relay/backfill.rs](../../src/relay/backfill.rs) | managed | Relay / product clients | — | 29 |
| [src/relay/control.rs](../../src/relay/control.rs) | managed | Relay / product clients | — | 15 |
| [src/relay/control_store.rs](../../src/relay/control_store.rs) | managed | Relay / product clients | — | 49 |
| [src/relay/mod.rs](../../src/relay/mod.rs) | managed | Relay / product clients | — | 27 |
| [src/relay/pull.rs](../../src/relay/pull.rs) | managed | Relay / product clients | — | 53 |
| [src/relay/push.rs](../../src/relay/push.rs) | managed | Relay / product clients | — | 18 |
| [src/relay/service_store.rs](../../src/relay/service_store.rs) | managed | Relay / product clients | — | 19 |
| [src/runtime/bootstrap.rs](../../src/runtime/bootstrap.rs) | managed | Provider / runtime hosts | — | 8 |
| [src/runtime/hosts_store.rs](../../src/runtime/hosts_store.rs) | managed | Provider / runtime hosts | — | 14 |
| [src/runtime/launch_status.rs](../../src/runtime/launch_status.rs) | managed | Provider / runtime hosts | — | 48 |
| [src/runtime/launch_status_client.rs](../../src/runtime/launch_status_client.rs) | managed | Provider / runtime hosts | — | 1 |
| [src/runtime/notify/wake.rs](../../src/runtime/notify/wake.rs) | managed | Provider / runtime hosts | — | 20 |
| [src/runtime/process_host_store.rs](../../src/runtime/process_host_store.rs) | managed | Provider / runtime hosts | — | 14 |
| [src/runtime/process_tracking.rs](../../src/runtime/process_tracking.rs) | managed | Provider / runtime hosts | — | 30 |
| [src/runtime/resume_client.rs](../../src/runtime/resume_client.rs) | managed | Provider / runtime hosts | 658 (managed) | 6 |
| [src/service/application.rs](../../src/service/application.rs) | managed | CLI / product clients | — | 7 |
| [src/service/context.rs](../../src/service/context.rs) | managed | CLI / product clients | — | 1 |
| [src/service/listing_store.rs](../../src/service/listing_store.rs) | managed | CLI / product clients | — | 30 |
| [src/service/maintenance_store.rs](../../src/service/maintenance_store.rs) | managed | CLI / product clients | 12 (managed) | 33 |
| [src/service/queries.rs](../../src/service/queries.rs) | managed | CLI / product clients | — | 3 |
| [src/service/queries_store.rs](../../src/service/queries_store.rs) | managed | CLI / product clients | — | 38 |
| [src/transcript/hermes.rs](../../src/transcript/hermes.rs) | provider | External transcript adapter / product clients | 17 (provider) | 9 |
| [src/transcript/opencode.rs](../../src/transcript/opencode.rs) | provider | External transcript adapter / product clients | 127 (provider), 296 (provider) | 22 |
| [src/transcript/store.rs](../../src/transcript/store.rs) | managed | External transcript adapter / product clients | — | 30 |

## Final cutover and evidence limits

This is the complete syntactic inventory under the declared scanner coverage, with the mixed-store decisions above. It is not compiler-resolved SQL lineage or a final exclusivity pass. Alias constructors, macro expansions and arbitrary cfg expressions remain scanner limitations; nearest-function labels are lexical hints. Candidate SQL may include comments/strings or read-only text. Raw SQLite access inside a daemon-only domain adapter still requires ownership review; a file-name allowlist alone cannot prove process ownership.

Final source-cutover and fixed-binary process/FD/legacy evidence belong to [independent acceptance](../architecture/acceptance.md) and the [Delivery](../../.controller/domain-daemon/delivery.md). Existing clients and hosts are not declared migrated by this source inventory. Old binaries that ignore the singleton lock require an explicit supported cutover; a new lock does not exclude them.
