# AgentX project bindings

For CBM connection exhaustion, isolated project environment and index recovery, read [project CBM operations](../../.docs/codebase-memory.md). Preserve the account-wide daemon and other projects' indexes.

Follow [shared guidance](../AGENTS.md). Reusable roles, methods and Skills are maintained in the [harness source](../../../harness/AGENTS.md); build a fixed release and explicitly install it here before changing consumer bindings. Do not recreate local copies of shared methods.

This directory owns only AgentX-specific additions. Published `.agent-guides/roles/*.md` are discovered by convention and use the selected default profile. Optional [native overrides](../../.agentx/roles/) retain special project profile/default selections; ordinary wrappers are unnecessary. Follow [current role usage](../../.docs/roles-workspaces.md) and [implementation evidence](../../.controller/convention-roles/AGENTS.md). Inspect them from the repository root with `AGENTX_DIR="$PWD/.agentx" .bin/agentx roles --dir "$PWD"`. A changed binding does not refresh an already-running Agent's instruction snapshot.

The [Browser Explorer bundle](browser-explorer/README.md) supplies a complete local Role and an AgentX Profile that passes Playwright MCP settings to Codex. Use these files as the source when adopting the role in another project.

Project/runtime constraints stay in root [AGENTS.md](../../AGENTS.md) and the affected module. Original role designs and execution evidence stay in their existing AgentX topics; the [upstream/adoption Task](../../../harness/.controller/agentx-guidance-upstream/implementation.task.md) records the ownership change and local recovery material. Shared codebase-memory guidance supports its wrapper and native CLI/MCP interfaces; select the available interface without changing account configuration.
