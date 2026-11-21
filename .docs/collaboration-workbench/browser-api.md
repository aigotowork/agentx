---
schema: document
status: active
owner: collaboration-workbench-native
last_updated: 2026-11-09
description: Closed workbench wire and native-derived browser consumption.
---

# Browser contract

Import [WorkbenchApi](../../web/workbench/api/client.ts) and the [derived closure](../../web/workbench/api/native.ts); [native declarations](../../src/collaboration/web/api.rs) remain authoritative. The compatibility `native.ts` entry aliases public `WorkbenchTypes`; native Service → emitted OpenAPI → `@agentx/client` is the maintained generation chain. The page adapter owns only lifetime/read cancellation; public SDK owns transport/codecs/unknown input. Historical derive.py does not generate the current entry. See [SDK adoption](../../.controller/web-sdk-adoption/web.task.md).

POST `/api/v1/connection` with `{}` returns `Descriptor`. POST `/api/v1/read` consumes `ReadCall`; POST `/api/v1/apply` consumes `ApplyCall`. Reads and applies return native `Response`, including `Rejected(Failure)`. The selected project never comes from HTTP. OperatorCommand excludes Agent reports, arbitrary recipients, launch/reset/configuration/SQL/file readers and any generic native request. Scope/project/generation/work operands are checked natively as well as at the adapter.

Every integer uses the existing `{"$integer":"decimal"}` codec, becoming bigint in memory. Unknown fields, duplicate JSON keys, numeric literals and noncanonical integer markers are rejected. Connection limits are provided by native `Descriptor.limits`; field sizes count UTF-8 bytes. Response availability is independent from domain state.

An apply disconnection throws `OutcomeUnknown` with the original command ID. Lookup queries that ID with the retained generation. Absence is unknown; an explicit retry must reuse the exact original call/actor/command ID. A refresh may persist a bounded pending reference (command ID, scope, project, generation, optional work ref), without work payloads or authorization. Do not construct a replacement command from that reference. Read cancellation and page close do not cancel submitted native work.

The work detail retains bounded complete historical evidence and explicit eligibility. WorkMessage freezes its assignment IDs, brief and optional result basis as well as admission. Native `Read::Thread` separately maps the exact retained work thread through existing Events Query into safe `ThreadMessage` values with a bounded integer cursor; no browser thread or predicate selector exists. Resource and thread lists are paged; client filters cover loaded rows only. Structured message receipts provide durable admission and exact recipients, with delivery unknown. Thread text is ordinary attributed conversation and cannot create a structured result/check/acknowledgement. Neither projection claims Provider delivery, runtime-start, native test execution or authenticated human acceptance.

On ordinary numeric-LAN HTTP, command identities use UUIDv4 bytes from `crypto.getRandomValues`; the secure-context-only randomUUID method is not required. A command ID stays fixed for the explicit intent. Native/HTTP reply loss raises `OutcomeUnknown`; native definitive failures remain `Rejected(Failure)` and HTTP admission failures are `AdapterError`.
