---
schema: task
status: completed
agent: raze
description: Design simple Agent labels for filtering, consumer grouping and plugin-owned business classification
links: [../../.docs/agent-classification.md, ../agent-metadata/cleanup.task.md]
---

# Outcome and authority

The user requests a coherent labels design before implementation: multiple labels per Agent, selection/filtering, UI grouping and plugin/business interpretation. They explicitly permit omitting a separate multi-value tags feature if it adds complexity. They accepted the general design on 2026-11-17 and emphasized database/index design and performance. The current follow-up must replace the provisional JSON-scan storage suggestion with concrete physical tables, indexed query paths and bounded SQLite evidence. This assignment authorizes design documents, disposable SQL experiments and a local explanatory preview; it does not authorize runtime implementation or live adoption.

# Working method and findings

Use the domain-contracts role's object/caller modeling and the shared design-discussion method. Work inline: this is one cohesive domain design, with no benefit from delegation. Inspect native identity, resume, storage and consumer boundaries; author the whole design, then check its consistency and the illustrative caller interaction once.

The user-referenced `.inbox/agent-labels.feedback.md` is absent from the checkout and has no result in a path-specific Git history query. The cleanup Task says the earlier discussion note was replaced by the current future-classification document. Treat the current user request as requirements; do not reconstruct missing feedback as quoted evidence. The running installed CLI still exposes the earlier metadata while the source cleanup is in progress, so installed help is not evidence for the candidate's contracts.

The complete recommendation is in the [owning model](../../.docs/agent-classification.md): a finite string-to-string map; one value per key, many keys per Agent; no special default label, implicit label or separate tags field. Labels are mutable classification, while participant identity, adopted workspace/Role and runtime status keep their existing authority. Plugin namespaces express interpretation conventions, not permissions. The design specifies exact-owner resume retention, empty fork/new-name instances, missing-key inequality and atomic per-key patching with optional revision checks. These are proposed semantics, not implemented behavior.

# Preview and verification

The standalone [preview](../../.sketches/agent-labels/index.html) is at `http://192.168.2.178:18967/.sketches/agent-labels/`. Start from the project root with `python3 .sketches/agent-labels/serve.py`; the current discussion server is PID `3017257` (tool session `81601`). Stop only that process with Ctrl-C or, after confirming its command, `kill 3017257`. The server whitelists the discussion page and owning design; it does not expose the repository tree. It remains available for this discussion, uses synthetic records and resets on manual refresh, without HMR or native calls.

One Chrome run opened the actual LAN URL with HTTP 200 and exercised AND selection, inequality including absent keys, grouping, business-state edits preserving native runtime status, empty-value rejection and a 390px viewport without horizontal overflow. No page errors occurred. After replacing the temporary general static server with the whitelisted preview server, a focused check established the unchanged page/document URL (200), no-store responses and unrelated private/source paths rejected (404). At 1366×900 all six representative Agent rows fit in the results region without horizontal overflow. Seventeen owned/discovery document links resolve, and the scalar/filter/lifecycle example checks pass. This is illustrative browser/document evidence, not validation of native filters, transactions or lifecycle implementation.

# Completed result and limits

The project entry and domain model now route to the [single owning design](../../.docs/agent-classification.md). The initial design covers the three requested uses, the optional-tags decision, key/value/absence rules, atomic patching and conflict recovery, scope/selector/page semantics, creation/resume/fork retention, native ownership/storage and a finite implementation handoff. No Rust, production Web consumer, production database, installed binary, cleanup assignment or existing user service changed. The user accepted the general design, and the database/performance follow-up described below is complete. Runtime implementation remains outside the current design assignment.

# Database/index follow-up

Applied the installed [data-query Skill](../../.agent-guides/skills/data-query/SKILL.md) to model label-layer work, inspect native scope/identity boundaries and compare actual SQLite plans with synthetic cardinalities. The earlier JSON-scan suggestion is superseded in the owning model by [two normalized tables and fixed indexes](../../.sketches/agent-labels/storage.sql): owner/key primary storage, key/value/owner covering lookup and creation-to-exact-alias reverse lookup. No dynamic per-key indexes or separate cache service are needed. Page labels load in one bounded batch; precise negative-filter totals remain explicitly on demand.

Run `python3 .sketches/agent-labels/check_storage.py --output .controller/agent-labels/storage-evidence.json` from the project root to reproduce. The script creates/removes its own temporary disk database and imports the actual existing creation schema without opening managed state. One run seeded 100,000 owners/975,000 label rows and 10 workspaces, then retained SQL/parameters/plans/row counts/warm timings in [the evidence](storage-evidence.json). All selected query paths use their expected indexes. The finite checks cover intersection, missing-value inequality, uniqueness, empty-value/revision rollback, stale revision refusal and foreign keys. The full negative count over 10,000 scoped owners took a 40.488 ms warm median versus 0.350 ms for a page, supporting the decision to omit implicit full counts. Timings are not production SLA.

The native listing currently mixes current rows and stopped events and computes additional views per entry. The owning design now records implementation obligations to filter before expensive hydration, keep label filtering explicitly current-only, bind cursors to filters, batch exact native scope/identity mapping and account for multiple resume aliases. The synthetic scope table is not proof these integrations already exist. The experiment deliberately fixes one sparse label-first join path; it does not prescribe that order for dense/negative production parameters. These limits remain visible in the model instead of being hidden by favorable benchmark timings.

The existing preview now discloses the physical model and performance boundary locally under “数据库与索引”; a focused Chrome check verified the actual LAN page and updated design document, the new disclosure and no horizontal overflow at 390px. Twenty-three owned local links resolve, the retained DDL hash matches the executed fixture, superseded JSON-scan recommendations are removed, and scoped `git diff --check` passes. No passed SQL experiment was rerun after documentation-only edits.
