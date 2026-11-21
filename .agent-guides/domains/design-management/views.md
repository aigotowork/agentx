---
schema: document
description: Semantic contracts for connected scenario, domain, module, runtime and interface views.
---

# Select complementary views

Choose views by the questions users need to answer. Keep a small overview and link to details. A view can be a paragraph, table or diagram in the existing topic; the following meanings apply only to the selected question, not as a required artifact set or wire schema.

| View | Required meaning when selected | Avoid confusing it with |
| --- | --- | --- |
| Product/scenario map | Actor, intent, trigger, desired outcome, main branches, scope and coverage | A feature checklist or implementation backlog |
| Feature design | Continuing purpose and scope, related needs/scenarios, professional design sources, actual support and remaining targets | A Proposal for one change or a copy of every specialist specification |
| Experience and interaction | Information needs, entry/navigation, visible states, actions, feedback and return; links to the criteria and domain facts they express | A page inventory, prototype success or a service DTO |
| Domain model | Subject meaning, identity, cardinality where useful, relationships, lifecycle and invariant owners | Database tables, processes or a class inventory |
| Module architecture | Responsibilities, public boundaries, required/provided contracts and directed composition/dependency edges | Runtime ordering inferred from imports |
| Runtime architecture | Actual hosts/processes, transports, data/resource owners, startup/shutdown/reconnect lifetime | A package tree or conceptual ownership diagram |
| CLI/interface map | Intent to canonical command/action, operands/options/defaults, output/effects and next action | Separate help pages without a global surface |
| Scenario flow/sequence | Ordered actor actions, internal collaborations, state/effects, visible results, relevant failure and recovery | A happy-path list or a sequence claimed from type references |
| Current/target/change view | Current basis, accepted target, alternatives, affected relationships and intended delta | One mixed diagram with an ambiguous global “complete” badge |

Use Mermaid flowcharts, sequence diagrams, state diagrams, tables or existing interactive tools as appropriate. Label arrows by meaning. If different edge types coexist, distinguish calls, data flow, ownership and dependency. State cardinality only when established. Explain key terms before using internal acronyms.

## Shared references and authority

A reading model can reference these concepts without adopting a universal file format:

| Reference | Meaning |
| --- | --- |
| Subject identity | Existing name, heading link or stable local ID when cross-view references need one; distinguish its domain/module/process role |
| Scenario and criterion | Link to the single owning user promise and meaningful step |
| Relation | Source, target, typed/labeled relationship, current/target scope and applicable support |
| Source | Direct link to the owning path/declaration/decision; selected version or captured basis only when the claim needs it |
| Evidence | Original check/run/artifact, what it supports and does not establish |
| Change | Motivation, affected references, proposed/accepted decision and owning work item |

Names are not proof that subjects are identical. Explain mappings such as a domain Session to multiple Executions or an interface action to several operations in the original topic. A labeled relationship or link can be sufficient; use an edge ID only when references to that particular edge require one. These references do not require an object registry.

Keep two status dimensions separate: decision (proposed, accepted, deferred, retired) and delivery evidence (defined, implemented, connected, verified, distributed/installed, or explicitly unknown/not-applicable). Local vocabulary may specialize these meanings. A shared support record can be referenced by several views instead of duplicating labels.

## Relationships between design expressions

Use the following meanings when a link alone leaves the design or change responsibility unclear. Keep the explanation with the source that owns it, using ordinary prose or a small mapping. These are semantic relationships, not extra metadata fields or a universal graph schema. [Cross-interface modeling](../../scenarios/collaborative-modeling.md#carry-a-promise-across-interfaces) derives unresolved operational mappings; this view keeps their established meaning readable.

| Relationship | Owning expression and question |
| --- | --- |
| Contribution | A feature references the need it aims to meet; delivering behavior does not prove the intended value |
| Use/composition | A scenario or design identifies capabilities it uses and the owner of the resulting coordination; can the complete result still hold if one changes? |
| Refinement | A criterion or professional design cites the original promise it makes precise; do conditions, identities and outcomes remain consistent? |
| Constraint | The original rule states its scope and consumers reference it; which known scenarios or operations must obey it? |
| Realization | The implementing boundary relates a guarantee to its actual mechanism and evidence; do existence, connection and effect each have support? |
| Presentation/derivation | A view, prototype, fixture or generated output names the source and local choices or omissions; what must be reviewed when the source changes? |
| Verification/support | A check or observation identifies its criterion, version and conditions; what does the result establish and where does it stop? |
| Change/replacement | A Proposal identifies the affected original sources, retained guarantees and intended differences; how do prior consumers or in-flight objects continue? |

A feature entry can link these original sources for navigation without maintaining a mirrored rule or status table. Derive reverse lookups from recorded relationships and report their search coverage. A current reading model explains the composed outcome; its graph completeness cannot certify semantic completeness or discover unrecorded consumers.

## CLI scenario depth

Use the canonical command declarations and existing CLI design method for the command tree and input matrix. For a selected end-to-end scenario connect:

`intent → exact invocation/input → application use case → domain interaction → state/effect → visible result → follow-up/recovery`

Show prerequisite identity/connection/configuration, meaningful defaults, stdout/stderr/exit meaning, and representative output. Label output as designed example or actual captured output. Do not invent successful command syntax or infer payload fields from prose; link exact declaration/schema when too large to inline.

Include important branches such as missing selection, refusal, disconnection, lost mutation response or original-result lookup when applicable. Distinguish “accepted”, “stored”, “delivered”, “read” and “acted on” according to the real product. Use internal details in the design view only where they explain the result; keep ordinary usage guidance focused on the caller.

## Tooling and portability

Prefer existing declaration extractors and model viewers that can use the original authoring materials. Native declarations own exact fields and signatures; documents own meaning and design relationships. Tools own rebuildable indexes, caches and reading projections. Do not hand-copy those facts into a second JSON/JS model to make ordinary design readable. If a tool cannot extract a relationship, keep its readable source and use a simpler view; a finite verification abstraction belongs to the selected experiment. A general tool receives explicit consumer input; it must not hardcode a product checkout, scan credentials, execute referenced product code or treat source labels as file access authority.

For an exported view retain its input identity, useful source links, evidence limits and generation basis. Mark references unavailable in an offline copy; portable guidance must not require the author's repository. Rendering and link integrity are structural claims. Source freshness, semantic agreement and actual behavior each need their own basis.

If an interactive view is built, preserve a readable entry and meaningful navigation without forcing every consumer to run a server. Useful controls include current/target selection, subject/scenario selection, related edges/steps and source/evidence access. Choose the smallest implementation supported by actual consumers; no generalized graph editor is implied.

