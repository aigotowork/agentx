---
schema: document
status: authored
owner: local-adapter
last_updated: 2026-11-08
description: Credential-free open-origin adapter cases; execution belongs to controller.
---

# Adapter case release

Current source follows the [browser contract](../../../.docs/observability/browser-api.md) and [LAN usability follow-up](../../../.controller/observability/lan-usability.md). Historical authenticated acceptance belongs to the original delivery; it does not describe the current source.

| Promise | Authored coverage in [tests.rs](tests.rs) | Independent process/browser coverage |
| --- | --- | --- |
| Independent visits/reloads | Repeated credential-free scope POST, no cookie, wildcard CORS, strict empty body | LAN page refresh and multiple simultaneous pages |
| Public browser boundary | IPv4/IPv6/DNS Host aliases accepted; absent/null/foreign/extension Origin and cross-site Fetch Metadata reach finite parsing; malformed/missing/duplicate Host and absolute target rejected; OPTIONS has empty 204 and CORS/PNA headers | Actual different-origin browser preflight/read/error, iframe and extension direct Fetch under [integration ownership](../../../.controller/web-sdk-adoption/integration.task.md) |
| Closed finite read surface | Unknown/mutation routes, nested fields, scope mismatch, media/body limits | Real native read evidence and unchanged safe disclosure |
| Shared bounded work | Eight native permits, Busy on saturation, scope discovery independent | Concurrent event pages; cancellation retains permit until native deadline |
| Precise evidence | u64/i64 marker edges, malformed/duplicate JSON, gaps/eras/unknown/withheld/local failures | Real daemon generation, configuration, requests/events |
| Finite assets and launch | Exact static paths/security headers, invalid asset overrides, optional assets, default numeric loopback and IPv4/IPv6 flags | Standalone binary from another cwd, embedded real application |

No Cargo/Node/browser/formatter execution by the HTTP author. Controller owns execution, exact artifact evidence and process cleanup. Client retains one outstanding event read per instance and cancels only that page's work on close; no global event poll lock or server session exists.
