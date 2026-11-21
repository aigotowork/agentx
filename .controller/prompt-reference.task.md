---
schema: task
status: completed
description: Research and document the existing AgentX prompt composition, replacement boundaries and command/message guidance.
---

Owner: koko. User requests documentation only; do not implement the abandoned system-prompt replacement or bootstrap-hook proposal. Preserve unrelated code and runtime resources. Work inline: this is cohesive source research across existing prompt consumers, not a new implementation or a reason to create a team.

Outcome: a user-facing Chinese reference at `.docs/prompt-system/README.md`, with focused Provider and hint appendices as needed, reachable from root AGENTS and existing context usage. Explain actual source selection, builtin modules, Role/additions, initial task/handoff, dynamic context, native delivery and all bounded command/message hint families. Reference native sources instead of maintaining another prompt implementation.

Method: trace runtime composition and Provider consumers; inventory fixed guidance emitters and their production callers; distinguish static tips, configurable per-message hints, mailbox/event guidance, ordinary diagnostics and historical/test-only paths. Validate examples, links/anchors and source coverage with static checks. Existing lifecycle acceptance records own executable evidence; source research does not establish installed/live adoption or model-following.

Research model: four content channels (fixed instructions, current runtime snapshots, user task input, mailbox/tool feedback). Main reference owns composition/selection/lifetime; Provider appendix maps native channels; hints appendix inventories emission conditions/frequency and production callers. Native source remains authoritative; the reference summarizes rather than duplicating rendered prompts.

Findings and friction recorded:

- `--agentx-system-prompt` remains additive. Existing `AGENTX_INSTRUCTION_SOURCES` supports ordered static replacement. `notes` is builtin-only; Role/additions append outside source selection; compatibility selection changes when an explicit instruction selector is present.
- `config bootstrap render` previews the builtin, not composed/stored instructions. A rendered preview can therefore differ from a source replacement, Role/additions and an existing frozen Agent.
- The current `agentx-runtime` contributor exposes a peer snapshot, not automatic Task/progress aggregation. The concise existing usage table was narrowed accordingly.
- Registry text for `list:types` and `mention:matching` has no production emission caller found. The document marks these as defined-only, not delivered behavior.
- The Codex/Gemini system-prompt config fields are registered, but no production consumer was found reading the fields into ordinary launch text. No behavior repair is authorized by this research; the Provider appendix records the limit.
- Hook mailbox formatting includes hints/recv tips; ordinary CLI listen/adhoc output uses a separate text formatter. Do not claim identical emitted guidance across these channels.
- Claude native Task bootstrap and the shared Codex service marker are separate from generic source replacement. The reference documents these exceptions rather than promising universal replacement.
- The old external-bootstrap-hook proposal was withdrawn per the user's selected existing-sources solution. No hook/selectors or system-prompt behavior change was implemented.
- The codebase-memory index query was unavailable (daemon connectivity timeout); targeted source searches and caller reads supplied the evidence instead. No unsupported graph-completeness claim is made.

Deliverables: [main reference](../.docs/prompt-system/README.md), [Provider channels](../.docs/prompt-system/providers.md), [hints inventory](../.docs/prompt-system/hints.md), linked from root AGENTS and the existing context usage entry. The old hook proposal is marked withdrawn and points to the selected existing-source solution.

Verification: a bounded static check resolved 256 local Markdown links and 33 Markdown anchors; checked six shell blocks with `bash -n`; parsed three JSON source-selector examples and one TOML example; compared the hint appendix against all 16 registered CLI/messaging tip keys and the main/appendix references against 21 Bootstrap constants. Whitespace and code fence checks passed. Mermaid syntax was inspected but not rendered. No AgentX launch/config command, build, provider execution, or model-following check was performed. This documents source behavior and does not establish installed/live adoption.
