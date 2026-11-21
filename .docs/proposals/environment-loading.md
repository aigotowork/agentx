---
schema: proposal
description: Load explicit dotenv sources into the originating AgentX environment consistently across local entry points.
status: accepted
models: [../model/README.md, ../architecture/runtime-hosts.md]
deliveries: [../../.controller/environment-loading/delivery.md]
---

# Unified environment file loading

The user selected implementation on 2026-11-10 after discussing the [current loading gaps](../../.controller/domain-daemon/feedback.md#environment-loading-discovery-2026-11-10). The outcome is an explicit, predictable way to load shared/project/local dotenv files when a terminal, background host or external application has not already prepared the environment. Accepted design does not establish executable support; [Delivery](../../.controller/environment-loading/delivery.md) owns actual acceptance.

## Selected contract

| Concern | Accepted intent |
| --- | --- |
| Sources | Repeated `--env-file`, comma-separated `AGENTX_ENV_FILES`, and `[environment].files` as a TOML string array select one ordered list. Explicit CLI replaces ENV, which replaces config. An explicitly empty environment selector disables the lower-level list. No recursive includes or wildcard discovery. |
| Overwrite | `[environment].override`, `AGENTX_ENV_OVERRIDE` and a reversible CLI control select one boolean; default false. Later files win over earlier files; inherited variables win over all file values unless override is enabled. Empty values are real values. |
| Project | Resolve relative file paths against the actual selected local project, not the daemon cwd. Preserve explicit project selectors and existing Codex stdio project hints. Shared files can use absolute/home-expanded paths. |
| Timing | Resolve the complete effective originating environment before consuming its dependent configuration, Provider executable or invocation snapshot. External explicit files may supply `AGENTX_DIR` before scope selection. Files selected by a scoped config must not redirect that scope. Selector declarations are frozen and cannot recursively load selectors found in file contents. |
| Process ownership | Per-invocation environment belongs to the originating host. Carry it through foreground/stdio/headless/new-window composition and explicit invocation capture. A nested-launch shell reconstruction must retain the explicitly loaded values. Do not mutate a shared daemon's global environment. |
| Files and errors | Dotenv is data, not shell. Use a suitable existing parser where proportional; explicitly selected missing, unreadable or invalid files fail before acquisition/launch. Diagnostics identify the source/path/line as useful, without printing secret values or raw source lines. |
| Existing sources | Integrate the authorized Codex stdio direnv preparation without executing `.envrc` as dotenv or importing twice. Preserve or explicitly reconcile the legacy scope `env` precedence; document the difference rather than silently changing existing launches. No automatic direnv authorization. |
| Inspection | Expose loaded-source status and variable provenance through an appropriate local configuration diagnostic, with values withheld by default. Help and ordinary version probes stay lightweight and side-effect free. |

The implementation owner refines exact native types, reversible flag spelling, config editing grammar and source ordering at the direnv/legacy seams before behavioral implementation. Compatible implementation decisions are authorized and do not need another user confirmation. Native declarations and the source-linked environment usage/design page become the canonical detailed contract.

## Scope and proportionality

This slice covers local AgentX entry points, meaningful source selection/override combinations, ordinary configuration consumption, local child launches and the existing Codex stdio path. Shared/project/local recipes are ordered file lists; no separate preset/profile system, automatic parent-directory dotenv scan, environment hot reload, universal secret manager or remote file distribution is selected. Existing AgentX identities/capabilities remain runtime-owned. General environment keys do not establish identity authority.

The current project `.bin` and user daemon/workbench, installed/default programs and the old separate LAN demo are distinct resources. Independent execution is disposable. Same-byte repository artifact promotion after acceptance is authorized; preserve the prior image and do not leave the user runtime in a build-mismatch state. The controller owns any necessary project-only cutover and its evidence.
