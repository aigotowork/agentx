---
scope: Native cloud deployment
status: implementation
owner: tina
related: [task.md]
---

# Deployment Workspace storage and APIs

[Task contract](task.md) owns the public URLs and DTOs. The control adapter serves the same Workspace catalogue under Namespace and Admin bearer guards; the control store owns the durable current lease and command journal. Page by the exclusive ordered pair (host, workspace_id), and derive online state from control lease expiry. Retain the most recent catalogue across expiration. Compare the normalized catalogue and availability in the lease transaction, then publish roster invalidations only after commit. A Namespace monitor observes lease expirations independently of heartbeats.

The communication store owns immutable launch membership per exact registration. Store it separately from lifecycle and transcript facts so existing records without evidence remain unexposed. Every exact identity check still uses host, name and incarnation. A launch receipt resolves that exact registered identity and compares its stored project ID with the command; a stop receipt still retires the exact instance. Add Workspace accepts a root alias and display name, and confirms the returned Workspace ID and name against the host's published lease.

The selected cutover uses a new deployment volume. Earlier alias-based control JSON does not identify canonical directories and cannot be converted to Workspace IDs. Existing local runtimes and old deployment volumes are outside this lane.

The isolated deployment harness passed all 24 cases. It compiled the actual Admin, catalogue, changes, communication and control modules and mounted the production bearer middleware on an ephemeral loopback port with a temporary SQLite directory. The HTTP case verifies shared Workspace reads, Namespace token rejection at Admin operations, expiry notifications after a simulated deployment restart, offline catalogue retention, renewal without a roster change, and notification on catalogue rename. Store cases cover composite paging, exact Agent membership and launch receipt checks, with existing stop, replay, mail and Transcript checks. This is native deployment evidence; the approved two-host HTTPS and browser run remains an integration acceptance step.
