---
schema: task
status: completed
agent: doma
description: Research Codex native agent definitions and configuration as an implementation direction for AgentX roles.
---

# Research scope

The user requests analysis and a document, not runtime implementation. Work inline because this is one cohesive role/configuration/launch-boundary study. Preserve concurrent source edits, configured Codex resources, installed binaries and running services.

Outcome: a Chinese research document at `.docs/proposals/codex-native-roles.md`, discoverable from root AGENTS, comparing native agent definitions, native profiles, MCP/plugins and AgentX roles/profiles. Identify feasible discovery and launch mechanisms, ownership, configuration composition, lifecycle constraints, alternatives, uncertainties and a bounded validation plan.

Method: use openai-docs and codebase-memory; verify official documentation against local CLI help and AgentX source. Keep official current documentation, local executable facts, source-derived current behavior and proposed support distinct. No delegation, provider inference or live configuration changes are needed.

Working findings: local `codex-cli 0.169.0` describes `--profile` as `$CODEX_HOME/<name>.config.toml`; compatibility with older inline profiles and native agent config selection must be checked rather than assumed. The manual helper and direct official page retrieval return HTTP 403; the project proxy supplies the narrow official pages. CBM finds role/profile symbols; relevant three source files match recorded metadata, with best-effort coverage only. AgentX currently adopts role prompt/profile, while native external configuration is not fully frozen by its argv snapshot.

Outcome: [research and proposed architecture](../.docs/proposals/codex-native-roles.md), linked from root AGENTS. The recommendation uses the existing Role -> AgentX Profile -> Provider launch chain: Role owns responsibility and instructions; Profile owns tool, argv and the proposed environment overrides; Codex's shared `CODEX_HOME` and native `--profile` own Codex configuration, MCP, plugin, Skill and permission declarations. Codex internal custom-agent spawning is outside this proposal. Proposed `env` fields are explicitly unimplemented.

Recorded uncertainties: official subagent documentation and configuration reference describe different custom-agent formats; external primary-session selection is not established by local CLI help. Existing bootstrap and writable-root preprocessing do not compute the complete effective native profile; argv passthrough is insufficient evidence of configuration preservation.

Verification: local Codex version/help and schema generation executed without inference; relevant graph paths matched metadata and production sources were read. Documentation relative links, code fences, TOML examples and scoped whitespace were checked. Mermaid was inspected, not rendered. No AgentX runtime/configuration changes, builds, launches, credential checks or live adoption occurred.
