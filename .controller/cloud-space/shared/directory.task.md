---
schema: task
status: active
agents: [{"scope":"/home/long/works/longwork/agentx/.agentx","generation":"a6195a5b-1dd4-4a6d-87d4-1f18f1b664fb","participant":{"canonical_name":"zatamo","incarnation_bits":4745317343520688855}}]
---

Implement selecting and creating a working directory in cloud Space launch through shared AgentCommandDialog and native control contract, within selected published host workspace root, prevent traversal and symlink escapes, preserve existing launches. Show actual host/workspace/directory, report validation errors, verify native and browser. Own launch-specific code and tests. Parent regenerates SDK; coordinate new fields early. No deploy. Record design and evidence here.

## Design

The shared dialog keeps the chosen host, published workspace alias and workspace-relative cwd together. Blank means the workspace root. A relative path selects an existing directory; an explicit option allows creating only a missing final segment during launch. Creation can survive a later Provider failure. The picker uses aliases and relative names; the separate Agent facts contract publishes the actual cwd after launch. Switching host or workspace clears the directory draft. Scope bearers and old hosts retain their existing capabilities.

The native [control contract](../../../src/cloud/control/types.rs) adds launch-only optional `working_directory` and `create_directory` fields. `HostCapabilities.directory_launch` gates the option. The host advertises support on its existing lease renewal; the deployment admits new fields only for a supporting lease. Omitting the fields retains the exact original serialized command for replay and old hosts. No separate effects, endpoints or receipt types are needed. [The uplink](../../../src/cloud/uplink/worker.rs) still journals and fences the exact host/install/lease before effects; it alone has access to the exported filesystem. Role selection stays rooted at the published workspace. Directory errors are safe codes displayed with the original launch receipt.

Feedback: browse/create commands were considered during handoff. The controller selected typed path plus opt-in creation to keep the full interaction inside the existing launch effect and avoid extra control and recovery workflows.

## Verification

- Space typechecking passed. The production-built [Cloud Space browser fixture](../../../web/agent-space/tests/cloud-space.browser.mjs) passed its existing root launch, receipt, stop and 320px reader paths. Screenshots: `/tmp/agentx-directory-browser.C3oqo4`.
- The four focused native directory tests passed: syntax bounds, no-follow traversal and leaf creation, capability negotiation with exact idempotent replay, and in-place upgrade of legacy leases and receipts. The same cloud test execution reported one separate stale OpenAPI path-count check after concurrent transcript/refresh changes.
- The [real HTTPS browser/native fixture](../../../tests/cloud_scope/shared_workbench.py) adds traversal and symlink denial, missing-directory feedback, creation in the browser, exact post-launch role/cwd, and a Linux supervisor cwd witness. Interim build failures were in the concurrent refresh/transcript slices and are tracked by their owners. No commit, installation or deployment.
