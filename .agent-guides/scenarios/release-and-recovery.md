---
schema: scenario
description: Prepare, distribute, deploy and recover fixed artifacts through the consuming project's actual release contracts.
tags: [release, deployment, recovery]
---

# Release a fixed result and verify the target

Use when preparing a versioned package, deploying a Web/Worker application, changing an existing service or evolving the corresponding release workflow. Start from the project's actual scripts/CI, artifact and target contracts, current authorization and success condition. Preparation and reversible checks proceed under existing scope; external publication or environment changes require the authority applicable to that action, without adding repeated approval rituals.

## Distinguish the effects

Version selection defines what is delivered; distribution creates and publishes identifiable bytes; deployment selects an artifact/configuration for an environment; rollout selects who receives behavior; operation verifies outcomes and restores service where needed. One command can cross several boundaries. A Git tag does not prove an installable artifact, registry upload does not prove a running service, and a returned URL does not prove a user's task.

Record only what the actual route needs:

| Object | Required facts |
| --- | --- |
| Intent | Change, consumer/target, success condition and scope |
| Candidate | Source revision/working changes, locks, build configuration and relevant checks |
| Artifact | Actual bytes/version/digest, location, build inputs and required resources |
| Target | Account/project/environment or installation root, selected configuration/resource bindings and owner |
| Receipt | Exact attempted action/target, operation or deployment ID, confirmed result or unknown outcome |
| Evidence | Which artifact/deployment was checked, observation, limits and outstanding conditions |
| Recovery | Affected layer, known irreversible effects, prerequisites, trigger and available recovery action |

Promote the same immutable artifact where configuration allows it. If a build embeds environment values, record the distinct derived artifact instead of claiming byte identity. Credentials remain with their actual host/provider; release records identify the source/role without copying secrets.

## Package or local tool distribution

Use the owner's build and packaging contract, preserving independent dependency/lock boundaries. Inspect real contents: runtime files and dependencies, assets, exports, executable modes, relative resources, license/notice and platform/ABI requirements. A tarball that requires later dependency downloads is not a closed offline runtime. An SDK-only package needs no fabricated CLI.

Install the actual candidate into a clean consumer outside the checkout. Exercise the public entry with an explicit fixture and output root. For a directory tool bundle, verify relocation, execution through its intended link, read-only installed resources and caller cwd/argv preservation. Entry symlinks and shims must resolve package resources while keeping the documented invocation name. Build/test success in the author checkout can conceal missing runtime files.

Where the installer manages files, compare plans and original receipts before updating. Preserve project-owned configuration and unknown local modifications. Check repeat installation, a meaningful update and relevant conflict/recovery path when the installer changed. Preserve fixed old releases and report content identity separately from a version string.

Publish or link only the prepared fixed artifact to the authorized destination. Retain the actual receipt and verify a consumer retrieves or resolves that artifact. A missing external runtime should name the affected capability, supported version/platform and trusted installation recipe; never silently remove a selected component or call the whole system ready.

## Protect current outputs and fixed history

Before replacing or retiring generated output, identify the admitted current roots and exact affected leaves. Keep fixed releases, historical runs, nested archives and unrelated temporary work outside that set; a recursive match under dist or candidate does not establish ownership. Preserve affected original bytes in owned recovery storage when rollback is required.

If an operation changes a historical artifact accidentally, stop that mutation, preserve the remaining evidence and distinguish exact restoration from reconstruction. Decide whether the lost item is required source/user data, a required fixed release, a needed current build or obsolete generated history. Block only outcomes whose requirements depend on it, apply the user's explicit disposition, and report loss or reconstruction accurately. A current successful build cannot prove that historical bytes were restored.

## Web or Worker deployment

Confirm the selected framework adapter, build mode, target account/project/environment, runtime bindings and configuration. Use the current platform's CLI/API contract; [Cloudflare](../domains/cloudflare/AGENTS.md) owns its platform methods and official Skills. Preview and production are distinct targets. A static site, an SSR server and a Worker have different runtime/resource requirements.

Build and inspect the candidate, exercise local or preview behavior where it answers the risk, then deploy to the authorized exact target. Retain the platform deployment ID and artifact/configuration relation. Verify routing/access, required assets and a representative real task, with independent effects where relevant. A homepage health response cannot prove authenticated navigation, writes or bound services.

If deployment returns an unknown result, query the original operation/target through the platform contract before retrying. Do not create another deployment merely because a client lost its reply. Keep unsuccessful verification distinct from deployment rejection: deployed code may already be serving traffic.

## Existing service and data change

Map affected code, configuration, routes, state/schema and external effects. Select rollout only as the real risk and infrastructure require: direct replacement, rolling, blue/green, canary or feature flag. Do not add Kubernetes or an elaborate flag system to a simple package task. State admission/drain behavior, mixed-version compatibility, observed signals and stop/continue decision.

For data evolution, check old/new readers and writers, in-flight work, background jobs and retained records. Expand/contract or dual-format stages must preserve the original guarantees and have explicit retirement conditions. A code rollback does not undo migrated data, secret changes or external notifications. Choose restore, compensating action or forward repair according to actual reversible boundaries and authority.

Verify a backup by its supported restore path when recovery is claimed. Name consistent restore point, acceptable loss/time with basis, replay restrictions and the owner who can reconcile unknown effects. Do not erase current user data by blindly restoring an entire directory. Retain partial receipts and inspect actual state after interruption before deciding the next action.

## Verify, operate and finish

Use signals that can trigger a concrete response: affected task outcomes, latency/error/unknown rates, capacity or contract invariant failures, under a defined population/window. Keep normal business rejection distinct from technical failure. [Quality and delivery](../domains/product-design/quality-and-delivery.md) covers these observation decisions. A log line is not the authoritative effect record.

Record preparation, publication/deployment, rollout and user-visible verification separately. Report the actual version/digest/target/receipt, representative checks, unsupported platforms or missing dependencies, cleanup and remaining work. When a failure occurs, stop the affected continuation, retain evidence and follow the original recovery contract; loss of a reply is never universal retry permission.

If only preparation was requested, finish with a concrete reviewable artifact and the remaining external action. If deployment was already authorized, continue through its verification. Future improvement returns to the owning source or runbook; do not rewrite historical evidence to claim a smooth release.

The standalone [release Skill](../skills/release/SKILL.md) offers a compact entry to the same responsibility boundaries. Use the current project/platform contract for executable commands; no generic publication endpoint is implied.
