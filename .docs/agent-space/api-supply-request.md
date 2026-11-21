---
schema: document
status: assessed
owner: agent-space
last_updated: 2026-11-15
description: Consumer requirements for gabi to assess native API supply for the visual AgentX office.
links: [./concept-alignment.md#supply-requirements, ../../.controller/agent-space-alignment/implementation.md, ../agent-management/design.md]
---

# Visual office API supply — assessment request

## Request and responsibility

The user explicitly asked zumi to organize the missing facts and ask gabi to assess them. Gabi owns the API design and should determine native projection, query boundaries, feasibility and sequencing. This is a consumer requirement, not a parallel API specification, mandatory new endpoint set or implementation dispatch. Inspect the latest native code before deciding; the baseline below may predate gabi's ongoing work.

The office frontend is implementing an independent offline semantic slice now. It will later consume the authoritative API rather than promoting its synthetic presentation types into service contracts. No requirement here blocks the current offline work. Do not infer missing facts from cwd, names or prose.

Canonical semantics, edge cases and original G1–G8 fact inventory: [concept alignment](concept-alignment.md). Native sources: [Agent management projections](../../src/management/contract.rs), [adopted collaboration](../../src/agents/workspaces.rs), [Role discovery/resolution](../../src/config/roles.rs), [identity and attachments](../../src/agents/attachments.rs), [workspace usage](../roles-workspaces.md). Preserve their ownership; exact DTO/operation names are yours to select.

## User-facing outcome

Within one selected AgentX scope, show directory offices and the actual Agents belonging to them; let a user distinguish available posts from named people, inspect one Agent's communication and model records, and eventually perform supported native actions. Role is a post/category; zero or multiple Agents can adopt it. Skill is an available/used method resource, never a person. The office metaphor creates no separate Team/Bot storage or authority model.

## Priority A — minimum truthful real-data grouping

| Consumer need | Missing fact / required meaning | What does not establish it |
| --- | --- | --- |
| Place an Agent in exactly one directory office, global area, or unknown-membership state | Adopted canonical workspace membership for current and stopped records. Distinguish confirmed global/no workspace from unavailable metadata. Native launch collaboration already stores membership; current inspected `AgentView` exposes only `directory`. | cwd, provider session location, parent agent, Role source directory. |
| Show the Agent's actual post | Adopted Role identity/provenance, sufficient to identify name and original definition/source namespace/path. Distinguish confirmed no Role from missing data. Preserve the adopted meaning when current role files change/disappear. | Display name, model, prompt text, current same-name Role definition. |
| Keep selection/drafts/actions safe | Retain existing scope/dataset generation/exact current or stopped Agent references when enriching views. Same display name with a different incarnation stays a separate target. Existing paging/capability/unknown-operation semantics must remain available. | A newly invented display-only Agent ID or a name-only map. |

These are primarily projections of existing native facts, rather than a request for new participant fields or storage. Please assess legacy data handling: native documented legacy unassigned/global behavior versus genuinely unavailable/corrupt/unsupported observations should be explicit. A plain nullable field is sufficient only if its absence meaning is unambiguous in the actual contract.

## Priority B — complete office and post browsing

| Consumer need | Required supply or acceptable first boundary |
| --- | --- |
| Discover known offices, including an empty configured office | Known canonical workspace/directory contexts and discovery provenance/coverage. Existing memberships alone can support an initial known-office view if that limit is explicit. Native marker discovery is `.agentx/roles/`; explicit membership without a marker remains valid. No unbounded scan or new workspace registry is prescribed. |
| Browse posts available in one directory | Native effective Role resolution and safe descriptive metadata: origin/local/inherited/global, description, profile reference, default declaration and resolution errors. Current catalog definitions are separate from adopted Agent snapshots. Let native resolution handle shadowing; the browser must not reimplement it. |
| Show a directory's actual message entry | Read/preview native resolved destination workspace/Role and actual endpoint recipient when known, including parent/global fallback, absence, ambiguity and error. A default Role declaration alone is not proof of an actual Agent binding. Read-only preview must not launch or send. |

The frontend can show inherited/global origins compactly and label incomplete discovery; it does not need full instruction text, credentials, runtime tickets or internal process information.

## Priority C — later native action alignment

Current inspected management `LaunchInput` has tool/profile/directory/prompt, but no Role selector or independent workspace selection. Assess how the existing native launch semantics can be exposed so that selecting a post actually adopts that Role and intended membership, while cwd remains independent. A prompt/profile launch is not an equivalent substitute.

Similarly assess directory-contact routing and explicit route/launch/send behavior, reusing native endpoint ownership and existing request identity/outcome-unknown rules. Existing Agent send/stop/resume/transcript capabilities should be reused, not rebuilt. Fresh launch, resume and fork are distinct operations. These actions are a later integration slice; they do not need to delay Priority A read supply.

## Existing supply to reuse; optional enrichment

- Existing exact Agent references, generation, paging, communication sender/recipients/intent/thread/reply data, operation receipts, transcript availability/reason and action capabilities should be reused. Please identify any current mismatch, rather than creating alternatives.
- Provider model records are separate from AgentX communication. Current inspected `ExchangeView` is coarser than the original rich renderer; the frontend can render actual available granularity. Detailed tool/reasoning blocks are optional, not a prerequisite for initial real reading.
- Adopted launch profile, observed model and genuinely current versus last-reported cwd are useful optional detail. Do not equate the profile catalog with historical adopted configuration or actual Provider model.
- Skill catalog and per-Agent usage are deferred unless already safely supplied. Catalog availability is not usage, and prompt references are not proof of execution. An unsupported catalog must not appear as an authoritative empty list. No unified Skill platform is requested.
- Native Agent mailbox unread is not human browser unread; the frontend should keep viewer-local new-content state separate unless an actual reader contract exists.

## Decisive examples to assess

1. `/projects/app/web` is a distinct workspace, with a Role inherited from `/projects/app`; two named Agents adopt that Role. One works in `/tmp`. Both remain web-team members and retain the inherited adopted post.
2. A global-origin Role launches into the web workspace; its Agent belongs to web. An Agent with confirmed global membership working in web appears only in the global area.
3. An empty `.agentx/roles/` directory has no Agents. It can be a known office with available inherited posts; no phantom person is created. A directory with only `.agentx/` is not silently treated as a native marker boundary.
4. An Agent has no Role, another has unavailable Role metadata, and a stopped same-name predecessor exists. These remain distinguishable, and drafts/actions never cross exact references.
5. The adopted Role file was edited or deleted after launch. The old Agent's provenance remains readable while the available-post catalog reflects current resolution.
6. A child directory inherits a default Role whose endpoint belongs to an ancestor. Preview reports the true route or ambiguity; it never invents a child controller or triggers a write.

## Requested assessment from gabi

Please return a concise disposition against these needs:

- Already supported in your latest design/source, with authoritative links.
- Existing native facts needing an API projection, including a recommended coherent scope.
- Native changes genuinely required, or unavailable/legacy facts with explicit limits.
- Recommended minimal first read slice (Priority A), then office/post browsing and later actions; optional detail that can remain deferred.
- Any conflict with native semantics or your current API direction, and the decision/input actually needed.

Publish your result in your existing API design/work record, or another file you own, and reply to zumi with its path. Preserve the active frontend owner's writes and user LAN preview `:18967`; this request authorizes assessment and collaboration, not an unrelated service cutover or implicit broad implementation.

## API-owner assessment received

Gabi completed the [source-verified assessment](../agent-management/office-supply-assessment.md). It accepts the consumer direction, recommends safe adopted-collaboration projection on existing live/stopped Agent views first, and identifies checked decoding as necessary to distinguish legacy absence from invalid/unavailable data. Full endpoint preview needs pure native inspection because EndpointLookup can write a binding. Known-context discovery and effective posts follow within bounded coverage; Role/workspace action composition follows separately. The assessment owns these implementation judgments and records current source limits. No new API support was implemented by this request/assessment.
