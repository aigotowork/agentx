---
schema: scenario
description: Compose scenarios and resources with explicit routing, scope and ownership
tags: [harness-authoring, routing]
---

# Compose guidance and reading routes

Use when connecting scenarios, changing an entry point or organizing guidance and supporting resources. Apply [entry-point conventions](../../../documentation.md#agent-entry-points-and-context); keep one editable source for each explanation.

## State what a relationship means

| Relationship | Make clear |
| --- | --- |
| Prerequisite | What result or context is needed before which action |
| Composition | Which methods apply together and what each owns |
| Alternative | What condition selects an option, with priority only where needed |
| Follow-up | What result makes the next scenario relevant |
| Specialization | Which scope refines or replaces which default and why |
| Reference | What the material helps explain; whether it is necessary for the current action |

A useful action route names the trigger, target section or capability and result needed from reading it. Add only the information necessary to choose and act. A link's existence does not make it a required read or an execution permission.

Apply all matching independent routes. Use first-match selection only for explicit alternatives. Keep order where a real dependency exists, and explain what to do with missing information rather than assuming a branch was selected. Avoid circular read requirements and reuse still-valid context.

## Organize by ownership and use

Choose the narrowest meaningful owner before a category. General, cross-domain or actionable scenarios without a settled domain can remain in root `scenarios/`; unproven ideas stay in the owner's `.sketches/`. Group a domain when it improves responsibility or discovery, repairing incoming references together.

Any meaningful scope may use roles, scenarios, skills, refs, scripts, examples, assets or subdomains as needed. A small subject can remain one file. Repeated directory names do not imply inheritance, merging or overrides. Keep short detail in local sections; move selectively needed detail behind explicit links. Preserve native Skill `references/` and script conventions where appropriate.

Use consumer-facing `scripts/` for capability resources and local `.tools/` for author maintenance where those names match project conventions. Resource placement does not itself include files in a release or register a Skill with a platform; follow the project's actual distribution rules.

Project additions belong in `.agent-guides/local/` with explicit entry routes; module-specific guidance can remain with that module. Scope any replacement of shared defaults explicitly. Installed shared files remain product-owned, and matching filenames confer no override authority.

Use the owning [Package catalog](../../../documentation.md#module-artifacts) for concrete resources and [placement guidance](../../../documentation.md#artifact-placement-rules) for reusable location rules. Do not copy their inventories or templates into every entry.

## Check the consuming path

From the real entry and a representative task, follow only applicable links. Confirm the Agent can select the method, obtain essential context and determine where results belong. Check a combined-condition case when composition changed and an excluded case when a trigger broadened. Use [validation](validation.md) for the affected concerns; resolving all filenames alone does not establish a usable route.

## Account for the host's actual loading behavior

Keep the automatically encountered entry short: responsibility, essential constraints and action-triggered routes. Moving detail into conditional files can reduce loaded context; adding a conditional heading inside an automatically loaded long file does not remove its bytes. Nested entries may be accumulated, truncated or refreshed differently by different Agent hosts and versions. Check the selected host's current documented behavior and, where available, an isolated prompt/input diagnostic before claiming discovery. Do not turn one platform's default budget or precedence into a universal guidance rule.

A read-only API consumer needs effect, identity and recovery semantics but usually not internal build or release procedures. A maintainer needs those procedures when changing the corresponding responsibility. Walk the real root and module/README routes for normal, combined and excluded tasks; do not rely on the author silently supplying missing context. Preserve user authority and explicit local scope rather than assuming matching filenames override shared rules.
