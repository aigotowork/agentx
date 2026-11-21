---
schema: delivery
description: Implement and verify local roles, stable workspace teams and default directory messaging.
status: completed
stages:
  implementation:
    description: Compose role definitions, launch and native workspace endpoint behavior.
    tasks: ["./tasks/roles.md", "./tasks/launch.md", "./tasks/membership.md", "./tasks/integration.md"]
  acceptance:
    description: Independently exercise the integrated binary in disposable scopes.
    tasks: ["./tasks/acceptance.md"]
    depends_on: [{stage: implementation, condition: Integrated source compiles and scoped checks pass}]
  discoverability:
    description: Teach role and directory messaging through injected guides and self-contained help.
    tasks: ["./tasks/guidance.md"]
---

# Roles and workspace delivery

Authority: [accepted Proposal](../../.docs/proposals/roles-workspaces.md). Design: [selected sketch](../../.sketches/role-workspace-team.md). Implementation contracts and single-writer ownership: [team entry](AGENTS.md). Controller: bime. Preserve installed executables, running daemons and other agents' changes. Build to `.bin/agentx-roles`.

## Accepted local outcome

Implemented and independently accepted for disposable Linux native CLI/daemon and local headless process/session execution with a fake Codex Provider. The dedicated artifact is `.bin/agentx-roles`, version `agentx 0.7.31`, SHA-256 `6b67fb228008699cf0ac8cd1bc705ab192e9777311f81bcd43eda9c2c6704418`. It uses a command-local release profile with opt-level 1, no LTO and 16 codegen units; repository profile settings are unchanged. [Selected build inputs](evidence/final-build-inputs.json) record source stability and the exact artifact.

Delivered: passive local/global role discovery and inspection; one-pass role/profile launch; adopted fixed instructions and explicit additions across resume/fork; stable workspace membership and scoped/global listing; exact parent-default delegation with original requested directory; native endpoint adoption, atomic reservation and retired-binding reset; durable first-message submission and safe concurrent reuse. Startup-pending recipients can receive messages in their existing mailbox; failed or uncertain recipients do not trigger speculative replacement.

[Independent acceptance](tasks/acceptance.md) records nine latest case groups passed and zero failed, with explicit per-artifact attribution in the [aggregate report](evidence/acceptance.json). Discovery/invalid definitions, unsupported-mode rejection, delegation and ambiguity/drift cases passed on the original candidate; history/composition/membership passed on the instruction-corrected candidate; final startup-gated concurrency passed on the final artifact. The final case proves a durable message while the recipient is still `inactive/new` with no session, two distinct accepted events, one endpoint and one Provider. Earlier failed candidates remain evidence, not accepted outputs. [Integration](tasks/integration.md) records 155 scoped unit checks passed, one existing manual benchmark ignored and zero storage-boundary violations.

Use [角色与目录团队](../../.docs/roles-workspaces.md) for setup, selectors, command examples, recovery and daemon adoption. The original implementation/acceptance and first [guidance follow-up](tasks/guidance.md) are accepted; the current bounded follow-ups are listed below. Startup and native subagent instructions now share direct role/team examples; role setup and directory messaging are discoverable in root/command help, `run docs --cli` and `--help-all`. Eleven scoped rebuilt-output/quickstart checks passed. Project `.bin/agentx` contains the verified guidance artifact `158e1c505871c5f8a594de372a44ec89def8487e485bf80e97195ee6cc329a02`; `.bin/agentx-roles` retains the accepted original feature artifact. Exact follow-up evidence and lifetime limits are in its Task.

## Support and resource boundaries

Completed bounded maintenance: [file prompt source context](tasks/roles.md#user-requested-file-prompt-source-context) adds the original prompt file location to the adopted text, and [role/profile guidance](tasks/guidance.md#user-selected-roleprofile-launch-guidance) replaces the injected Provider-specific launch example. Seven combined [output/launch/history checks](evidence/prompt-origin-profile/report.json) pass. Current `.bin/agentx` is `agentx 0.7.31`, SHA-256 `ccd0edc74d9fdbd0cb6beb58a879b33d1c71a928088857159d815a1045f07965`; [selected inputs and cutover](evidence/prompt-origin-profile-inputs.json) have no source drift. Earlier accepted feature/guidance evidence remains historical, as do the prior artifact identifiers above. No live daemon or adopted instruction snapshot was changed.

Real-model inference, real Provider compatibility, new GUI-terminal execution and non-Linux platforms were not established by this acceptance. Fresh remote role/workspace launch and shared Codex app-server stdio role/workspace launch explicitly reject before those effects. The ordinary existing global shared-stdio path keeps its prior contract. A new capability probe prevents old daemons from silently discarding collaboration data; operating an existing scope requires an explicitly selected supporting daemon.

The original implementation Delivery did not replace `.bin/agentx`, installed/default programs or running services. Taro's separate user-authorized recovery/deployment work promoted the accepted bytes to project `.bin/agentx`; its cutover and TERM evidence belong to [startup usability](../domain-daemon/startup-usability.md). The completed guidance follow-up subsequently updated only the project executable, retained a rollback and preserved system installations, live services and adopted instruction snapshots. Its build/output checks remain separate from the original feature evidence.
