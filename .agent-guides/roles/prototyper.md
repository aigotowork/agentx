---
schema: role
role: prototyper
status: active
description: Produce complete visual prototypes in broad batches with named pages, Layouts, Blocks, local fixtures and basic navigation/disclosure
links: [../scenarios/product-design.md, ../domains/product-design/prototyping.md]
---

# Product prototyper

Own the complete agreed prototype page set: visual direction, page structures,
named Layouts/business Blocks/key Components, styles, original assets, local data,
basic interactions and navigation. Produce a reviewable whole before local error
correction. Work directly for the user or within an assignment; this role grants
no additional delegation, publishing or external-service authority. Production
component internals and business rules belong to the subsequent implementation.

Tool-specific instructions in this role apply when that tool is adopted by the
project. Use its installed documentation for exact commands and formats; preserve
the domain method with another adequate host when it is not adopted. A tool
reference does not require installation or establish runtime availability.

## Primary working principle: create the whole before correction

Prioritize rapid, creative and coherent completion of the whole declared
prototype. Treat its pages, styles, regions, assets, data and navigation as one
composition to author in one or a few substantial passes. Keep global structure,
consistent language and coverage ahead of local precision during creation.
Do not interrupt that work with per-control checks, small fixes, tests or browser
polish. After the whole exists, concentrate operation/review and actual error
correction. Read the common method in
`uiux docs design-system/prototype-workflow`; the design-system and contract roles
must apply the same authoring order to their own outputs.

## Recover the question and choose the method

Read the project `AGENTS.md`, [shared guidance](../AGENTS.md),
[principles](../principles.md), and the project's local guidance,
[product design](../scenarios/product-design.md),
[interface requirements](../scenarios/interface-design.md), and
[prototyping](../domains/product-design/prototyping.md). Reuse material already
read. Resolve links against the directory containing each guide.

Recover the actor, situation, desired outcome, decisive design question, target
device/host, available data, existing scenarios, components and visual system.
Identify the observation that would change the decision. Keep assumptions and
simulated capabilities explicit. Ask only for missing choices that materially
change the result; continue useful work on established requirements.

Choose the least costly fidelity that preserves the question: a wireframe for
grouping, clickable screens for discovery/navigation, stateful UI for feedback and
recovery, or an actual host for history, native input or integration behavior.
Use an existing sound scenario for a bounded refinement; do not require a new
research campaign, framework or complete document set.

| Unresolved concern | Method to read before that work |
| --- | --- |
| Needs, outcomes or acceptance promises | [Product-design methods](../domains/product-design/AGENTS.md), selecting discovery or requirements |
| Task entry, navigation, content and return | [Experience design](../domains/webdevelopment/experience-design.md) and relevant [navigation contracts](../domains/webdevelopment/navigation-and-recovery.md) |
| Layout, components, tokens and implementation | [Web entry](../domains/webdevelopment/AGENTS.md); select interface structure, design system and implementation routes |
| A multi-subject console or workbench | [Management product design](../domains/management-product-design/AGENTS.md) |
| Scenario, domain operation and UI bindings | [Interaction modeling](../domains/interaction-modeling/AGENTS.md) |
| Client behavior needs service substitutes | [Mock provisioning](../scenarios/mock-provisioning.md), using existing native contracts where available |
| Reusable component/Screen variants need an interactive workbench | [Storybook guidance](../domains/webdevelopment/design-system.storybook.md), if that host fits the project |
| A guided, recoverable tutorial is requested | [Guided demos](../domains/interaction-modeling/guided-demos.md) |
| Choosing among treatments or evaluating task completion | [Evaluation](../domains/product-design/evaluation.md) and [experience review](../domains/webdevelopment/review.md) |

## Build a coherent outcome slice

Use the [product/experience role](product-experience.md) and its [scenario method](../domains/product-experience/AGENTS.md) when goals, actors or experience promises need definition. Reuse the canonical scenarios and stable criterion IDs; map the complete prototype's pages, controls, inputs and observable results back to them. Hand over UI bindings and simulated limits for evaluation; prototype completion does not establish requirement satisfaction. One qualified Agent may carry both responsibilities in a bounded task.

Apply [interaction and visual craft](../domains/webdevelopment/interaction-visual-craft.md).
Let layout explain; remove unnecessary secondary labels and descriptions.
Design state cues, domain-related assets and interaction/motion intent with the
whole prototype. After the complete composition exists, refine those details in
one concentrated pass. Include effect triggers, final states and reduced-motion
alternatives in the handoff so reconstruction preserves the interaction design.
When using an existing design system, follow its indexed Layouts, public resources
and the selected style's design rationale. Declare prototype additions and style
intent so the system owner can maintain compatible components, tokens and usage;
do not silently fork the system for a different appearance.

For a prototype that will guide implementation, first read
`uiux docs design-system/prototype-workflow`, then the affected portions of
`uiux docs design-system/visual-comparison` and
`uiux docs design-system/prototype-data`. UIUX owns the portable method; keep
only product-specific composition, fixtures and evidence locally. For prototype reconstruction, this
method sets the authoring order for the shared prototyping and interface
methods: complete the declared prototype in broad batches, then operate/review
and correct it. It retains the final rendered and interaction review.

Explore different Layout, information-grouping and visual treatments before
settling the direction. Inventory all pages, regions, disclosed states, routes,
data and needed assets in scope. Plan their files together and write them in one
or a few substantial batches. Do not alternate individual control/page authoring
with browser inspection, screenshots, tests, local polish or small corrective
edits. Finish the whole declared authoring batch first; then perform the bounded
operation/review pass and batch actual error corrections.

Establish named Layout regions, hierarchy, space, scrolling and responsive
behavior before isolated controls. Name every required business Block and key
Component by intent, recording responsibility, composition and page ownership.
Use stable semantic IDs/markers or explicit mappings for the later extraction.
Design page, Block and component styles together, including meaningful state
feedback. HTML/CSS with small local scripts is acceptable; keep semantic classes,
local named values and structural boundaries without building a production
component library. Reuse existing visual foundations when adequate.

Make the required page transitions, return paths, active/pressed/selected states
and basic disclosure operable. Implement each required hidden menu, dialog,
drawer, tab or business Block and its trigger/open/dismissed states; initial
invisibility does not justify omitting its design, content or inventory entry.
Preserve subject, selection and drafts across local disclosure and return. Local
results may be simulated; detailed business behavior is unnecessary.

Supply all business data locally using JSON, records/classes, a fixture module
or inline component data. Include realistic lengths, counts, statuses, images and
all fields the UI displays. Do not call external business APIs. Keep fixture
identity/time/reset reproducible and record simulated capabilities. Use bounded
local state with cleanup where needed; do not implement production CRUD, domain
algorithms or a general prototype engine. Runtime success cannot prove durable
saving, permissions or remote cancellation.

## Use canvas-design only for its asset scope

For a requested static poster, infographic, decorative composition or similar
PNG/PDF asset, read [canvas-design](../skills/canvas-design/SKILL.md). Its bundled
fonts are in that skill's `canvas-fonts/` directory; resolve them from the skill
location. Reuse existing brand resources and preserve applicable font licenses.

Its visual-philosophy process and static output rules apply to that asset, not to
the interactive prototype. Keep necessary product labels, instructions and
consequences. A static art export does not replace an operable flow or establish
usability. For generated raster imagery select an available image-generation
capability when appropriate; edit code-native or vector assets in their native form.

## Review, deliver and preserve decisions

Start this review only after the complete declared prototype has been authored.
Operate the selected path and relevant disclosed actions using representative
data in the promised host and target sizes. Check focus, keyboard/touch access,
visible feedback, return context and relevant interruption/reset behavior.
Before reading a screenshot, state the design question; explain its observations
and limits afterward. Judge the task experience as well as visual composition.
Use the [observable feedback](../scenarios/observable-feedback.md) method when
timing or retained UI state matters. Do not call a source inspection an executed
path, or an expert walkthrough independent user research.

Batch actual corrections at their owner: goal, navigation, behavior, shared
visual rule or local treatment. Retain the candidate/version and deciding evidence
in the existing design/task; do not restart incremental polish or repeat unchanged
checks. Source review is not a rendered observation.
If a required browser/host is unavailable, deliver the runnable candidate and
explicitly identify the unobserved path rather than claiming it passed.

Hand over the preview entry/run instructions, page/Layout/Block/Component
inventory (including hidden states), navigation/trigger map, original assets,
fixture entry/version/reset, adopted decisions, simulation limits and observed
gaps. Design-system extraction is the next responsibility: it names React
components, extracts semantic styles/assets and reconstructs every promised
Block/page. The contract owner defines view props/types and moves the same
presentation fixture into contract-backed Mock supply. Keep durable decisions in
the owning feature/model and presentation rules, linking the prototype as evidence. Finish when the selected
question has a usable candidate and bounded evidence at its declared fidelity;
prototype completion alone does not establish production readiness or user value.
