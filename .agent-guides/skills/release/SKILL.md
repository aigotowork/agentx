---
name: release
description: Prepare and carry out an authorized release of an existing package, web or Worker application, or running service with exact artifact identity, target checks, observable acceptance and recovery.
---

# Release an existing system

Select the owning project's established release path. Determine the requested outcome, fixed candidate, target environment and current authorization. Reading this Skill does not authorize publication, deployment or changing credentials. Complete the already authorized preparation before requesting a genuinely missing final approval.

| Delivery | Preserve and check |
| --- | --- |
| Package or portable directory | Version, archive/file digests, runtime dependency closure, installation ownership, clean-consumer invocation and update conflicts |
| Web or Worker application | Built assets, API/config compatibility, selected account/project/environment, routing, preview/production identity and actual served result |
| Existing service | Configuration and schema compatibility, readiness/drain behavior, rollout sequence, retained previous artifact and data-compatible recovery |

1. Build through the original owner and fix the candidate identity. Record source, dependencies, platform and build inputs; do not silently rebuild a different artifact between acceptance and release.
2. Verify in the nearest useful isolated consumer. Test installation/startup and one meaningful user result. Keep secrets and local bindings outside immutable artifacts. Preserve unknown or locally edited files on update.
3. Establish how success will be observed and how partial or unknown effects will be recovered. A previous binary may not undo a data migration; state the actual compatibility boundary.
4. Execute only the authorized target operation using its official CLI/API and explicit identity. Retain its receipt separately from independent target verification. If the reply is lost, inspect the original attempt before any retry.
5. Record actual artifact, target, observations, unresolved conditions and recovery material. Release temporary resources owned by this attempt. Do not report a preview, process-only test or static CI configuration as a production release.

Use provider-specific installed Skills and current official commands when needed. There is no universal deploy command or mandatory new pipeline in this Skill.
