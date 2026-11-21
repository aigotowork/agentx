---
schema: proposal
description: Directory-scoped reusable roles and workspace teams with lazy default recipients.
status: accepted
deliveries: ["../../.controller/roles-workspaces/delivery.md"]
models: ["../../.sketches/role-workspace-team.md"]
---

# Roles and workspace teams

The accepted convention-role follow-up adds published Markdown definitions and thin TOML overrides. Use [current role usage](../roles-workspaces.md) and [the follow-up maintenance entry](../../.controller/convention-roles/AGENTS.md) for current source and evidence; this Proposal retains the original delivery rationale.

The user accepted the design discussion and authorized planning, design and implementation on 2026-11-15. The [design sketch](../../.sketches/role-workspace-team.md) owns the selected semantics; the [Delivery](../../.controller/roles-workspaces/delivery.md) tracks executable support separately.

Implement role discovery and inspection, role/profile launch and frozen instructions, stable workspace membership and scoped listing, and lazy directory-default messaging. Reuse the current daemon, profile resolver, participant lifecycle and durable mailbox. Role origin is not membership; cwd is not membership; directory fallback routes to the declaring workspace's endpoint. Existing unqualified sends retain their current explicit-recipient/thread/broadcast semantics. Explicit `--dir` or `--workspace` without recipients selects directory-default messaging. One atomic native participant reservation binds a default endpoint before process creation; failed or uncertain startup never authorizes duplicate live recipients.

Initial delivery must establish local Linux executable evidence with disposable scopes and a separately named binary. Remote role support must either resolve entirely on the target or reject explicitly before any effect; never silently forward unresolved new selectors to an old receiver. Unsupported remote combinations are a documented limitation, not successful support. Live services and installed programs are preserved.

Keep local role files and existing scoped profiles separate. Namespace search is nearest workspace/ancestor then global, whole-definition shadowing. No-role launches still join their selected workspace. Unbound legacy records are global; local lists include global members, with explicit `--all` for all workspaces. Native snapshots preserve adopted role and membership across history; fixed instructions retain provider-specific native channels.
