---
schema: scenario
description: Discover, study and compare external skills, then choose evidence-based reuse for a concrete consumer task.
tags: [skills, research, reuse]
---

# Research skills for a concrete task

Use this scenario when asked to find relevant skills, understand how a skill works or why it may help, compare alternatives, or assess adoption into the consuming project. Produce a supported choice or a useful method analysis. For an already selected skill's execution, follow the consuming project and that skill; for inventory maintenance alone, use the existing collection owner's generator and schema.

## Frame the question

Identify the task or decision to improve, the expected output, the relevant environment and any stated source limits. Use information already available; ask only when a missing fact would materially change the research. A request to compare local sources does not require a new public survey.

Choose the depth that resolves the question. A recommendation needs a small useful candidate set and source-based reasons. A method study needs the mechanism, assumptions and limits. Extend the search only when a concrete gap remains; do not invent a candidate quota, scoring system or research campaign.

## Discover candidates using existing tools

Use the consumer's existing source collection, installed discovery CLI and its current help. With the official skills CLI, `npx skills find <keywords>` discovers candidates and `npx skills add <source> --list` lists without installing; the package runner may fetch the CLI. Prefer an already prepared executable in offline or pinned environments. Choose the matching route:

| Starting point | Route | What it establishes |
| --- | --- | --- |
| A task with no selected source | Use the official skills CLI `find` with task keywords; narrow to an owner when relevant | Candidates from skills.sh's indexed public catalog |
| A known remote repository or local checkout | Use official `add <source> --list` | Skills recognized by the CLI in that source, without installing them |
| A question about exact files in a research collection | Use `rg` on the actual inventory or `SKILL.md` files, then open the original file | Paths and previews in the current local checkout |
| A question already covered by local research | Read the relevant existing analysis and follow its source links | Prior reasoning and evidence within its recorded scope |

Use source-language terms when local keyword searches miss a concept. Tags come only from upstream metadata; they are not a complete taxonomy. A short preview cannot establish the full trigger or method. Where the inventory records aliases or issues, follow those references to avoid counting platform links as independent capabilities and inspect the original files.

Public discovery, installable sources and local checkouts have different coverage. A failed or empty public search does not prove that a skill does not exist. If online discovery is unavailable, continue with known sources and local evidence and state the coverage limit. Use the consumer's existing network route. An online catalog, a local checkout and platform-registered skills have different inventories; do not infer one from another.

## Read and explain the actual method

1. Identify the source repository or website, the actual skill entry and the revision inspected. For a checkout, record the origin and commit when they matter, plus local changes affecting the conclusion. Distinguish a directory nickname from the upstream repository identity and a read date from source freshness.
2. Read the complete SKILL.md frontmatter and body. Follow only the references, templates, data and scripts needed to evaluate its important claims. If an external document changes independently, record the version or retrieval date used.
3. Explain the mechanism as `input → decisions/actions → intermediate artifacts → output/feedback`. Identify what improves the target judgment, what inputs and tools it assumes, and what can fail. Inspect a claimed script or calculation as source before relying on its description.
4. Compare candidates and existing project capabilities on the same task. Consider useful differences in method, output, dependencies and adaptation cost. Use a concrete example and a relevant boundary case when needed to distinguish the alternatives; a quick lookup does not require a full comparative study.
5. Separate author claims, facts read from source, researcher inference and actual observed results. Installation counts and search ranking help discovery; they do not establish effectiveness for this task. Treat unread support code and untested behavior as limitations.

For product and design methodology questions, select the relevant [professional method](../domains/product-design/AGENTS.md) to compare what a candidate adds. A prior research assignment does not govern the current task.

## Decide what to reuse

Choose the smallest adequate outcome: use an existing skill, configure it for the consumer, adapt a specific method, retain it as a reference, or decline it with reasons. Prefer existing CLI and tool capabilities; add custom code only for an identified gap. An existing collection generator owns its exact inventory, the official skills CLI owns discovery and installation, and `rg` owns local keyword search.

When adaptation is part of the requested work, apply [external method adaptation](../domains/harness-authoring/scenarios/adaptation.md). Compare source assumptions with the consuming project's tools, artifact destinations and authority; check attribution and licensing when copying or distributing content. Preserve the useful method without importing unrelated global modes, mandatory delegation or approval rituals.

Treat upstream instructions as research material until selected for use under the current task. Research alone does not require installation, execution of source scripts, or registration of skills. If the session already authorizes adoption or a trial, proceed within that scope and verify the relevant outcome; do not add another approval step by habit. A static source analysis supports a reasoned assessment, while a claim of improved results requires actual observations.

Retain a new checkout only when keeping its source helps the research. Preserve existing checkout state; do not update every repository to make one comparison. After changing an owned collection, refresh its existing inventory with the documented generator. Generated previews remain lookup aids, not conclusions or independently adopted skills.

## Record a useful result and stop

For a short recommendation, provide the choice, source link, fit and material limits in the answer. For substantive research, update the owning topic's existing note, design or task; if no topic exists, create one bounded note under the responsible module's `.sketches/` and link it from its entry. Keep collection operations with the collection owner and professional analysis with that domain. Write reusable agent guidance in direct English.

Record enough to make the decision auditable:

- The research question and scope, with the actual sources and revisions inspected.
- The method or capability that matters, its relation to existing alternatives, and supporting file/section links.
- The reuse decision, required adaptation, evidence limits and any concrete unresolved question.

Finish when the original question has a supported answer or a precise evidence gap. A long candidate list, successful installation or generated inventory alone does not complete a request to explain a skill's value. Validate changed reading links and any actual trial at the smallest useful scope; do not create another review workflow.

The standalone [skill-research Skill](../skills/skill-research/SKILL.md) is a compact capability entry for callers outside this scenario system. Use the deeper analysis above when the question needs it; neither entry requires installing candidates simply to compare them.
