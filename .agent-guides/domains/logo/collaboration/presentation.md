---
schema: scenario
description: Present logos fairly across concepts, craft, materials and applications and convert user feedback into scoped decisions
tags: [logo, presentation, feedback]
---

# Presentation and feedback

Use when the user needs to compare, understand or steer design. A presentation is a decision aid with a specific question. Do not generate every possible scene by default. Build from the actual candidate sources and [presentation records](records.md), so feedback can return to the correct version.

## Choose the board for the question

| Current question | Useful view | What to control |
| --- | --- | --- |
| Which idea or personality fits? | Contact sheet of distinct territories with concise propositions | Comparable prominence and honest labels; materials may differ when intrinsic to the concepts |
| Are these really different structures? | Neutral or silhouette comparison, with source forms visible | Scale by visible mass as needed; same viewing conditions |
| Which refinement improves the mark? | Before/after or a small controlled set | Keep unrelated properties constant when diagnosing a change |
| How was the form made? | Source geometry/lettering study beside the result | Show actual construction or label a reconstruction; no decorative rigor |
| Does the identity survive context? | Selected real applications, light/dark or relevant surfaces | Same asset version; no secret redrawing for each mockup |
| Does a material expression fit? | Source plus one or more art-directed material versions | Label source changes and medium limitations |
| What will small use look like? | Actual-size ladder and enlarged detail as separate views | Display size, export pixels and device scale must be distinct |
| What is the identity system? | Primary mark, actual wordmark, compact mark and expressive assets where present | Show only variants that exist; annotate proposed ones |

A useful panel may combine source, lockup, dark background, size ladder and one application. That is one possible scene recipe, not a mandatory template for all work. An ornate emblem may need a large detail view; a lettering problem may need letter-pair comparisons; a character may need pose or expression studies.

## Make comparison fair

Use stable candidate IDs, actual source versions, restrained labels and clear source/variant roles. Avoid a polished mockup for one candidate and an unfinished outline for its competitor unless the difference in maturity is itself disclosed. Optical equalization can be better than identical bounding boxes; record that policy when it affects the comparison.

Control backgrounds and context when assessing form. Preserve deliberate color, lighting and storytelling when assessing expression. Neutrality is not a requirement to flatten material-led designs into black shapes. If an application treatment changes the silhouette or lettering, identify the new asset version.

Show enough detail to see the craft without overwhelming the choice. Separate a broad gallery from a focused shortlist. Do not replace meaningful differences with paragraphs of concept rhetoric. Short labels should explain an actual design decision, not claim unobserved trust, memorability or sophistication.

## Size and appearance need truthful labels

A `16 px` label must mean the intended CSS/display size, or explicitly identify it as bitmap pixels. A 32-pixel export shown at 16 CSS pixels on a 2x display is a different condition from a 32 CSS pixel icon. Contact sheets scaled in chat may not preserve actual-size viewing; provide a correct-scale artifact or describe that limit.

A grayscale preview is not necessarily a single-ink production asset. Inverting the whole image is not necessarily a reversed mark. A transparent export must have real alpha. Small variants can adjust detail, gaps and weight deliberately; preserve the recognition cue and label the adaptation.

Use relevant backgrounds and material conditions. A glowing mark may need a dark scene and a non-glow companion. A pale engraved mark may be intended for a particular surface. The actual application determines whether an adaptation is needed.

## Keep interaction easy

Offer useful comparisons and a small number of focused decisions. Let the user respond with ordinary language, a candidate ID, a region or a reference. They should not need to know “G2 continuity” to ask for a smoother animal cheek.

A future workbench should support browse → shortlist → compare → annotate → decide without losing selection or scroll context. Provide keyboard access, visible focus, textual names, zoom and reversible choices. Avoid using color alone for liked/rejected states. These are application requirements for the future UI, not claims that the current static boards already implement them.

For interactive product design, use [collaboration and workbench design](workbench.md). A board is one view in a shared round: annotations keep their original target/version, comparison baselines survive view changes, and a late result returns to its initiating plan. A user should be able to correct an interpretation or continue a comparison without reconstructing it from the chat transcript.

Pairwise comparisons can isolate a preference. Keep likes, dislikes and uncertain reactions independent; “neither” can be useful. Preserve a user's ability to like a material while rejecting the subject. Do not require a numerical score for every candidate or turn preferences into pseudo-objective rankings.

Selectable changes and retention scopes are owned by the [feedback vocabulary](../resources/feedback-vocabulary.yaml); style descriptors remain in the [style vocabulary](../resources/style-vocabulary.yaml). Use [conversation and guidance](conversation.md) for phase-specific prompts and interpretation. A description such as “dense detail” does not itself mean the user wants more or less of it.

## Translate feedback into the next test

| Feedback | Plausible interpretations | Useful next action |
| --- | --- | --- |
| Too rough | Contour joins, hierarchy, typography, drawing or material coherence | Diagnose the visible weak relationship; present a focused correction |
| Too similar | Same subject, skeleton, topology, composition or expression | Reopen the relevant concept dimensions rather than recolor |
| More premium | Could mean restraint, bespoke type, tactile craft, precision or confidence | Preserve the user's wording and compare concrete interpretations |
| Too cold | Could concern geometry, expression, palette, type or material | Change the likely lever; do not automatically add a mascot |
| I like this reference | The liked facet may be color, contour, material, content or composition | Retain the exact trait when known; avoid extrapolating the whole image |
| This animal feels wrong | Species, anatomy, pose, expression or association | Check subject structure and intended role before applying effects |

These are hypotheses, not universal translations. If the feedback could materially change the concept, use one focused clarification or discriminating comparison while continuing independent work. Do not repeatedly ask for confirmation of already accepted constraints.

## Make professional critique actionable

For a substantive critique, use the actual brief, stage and subject/version. Identify a specific observation, its consequence for the intended reading/use, what should survive, and a proposed change or discriminating comparison. Explain why this issue deserves attention now: it blocks a required use, weakens a retained cue, or is an optional exploration. Do not turn personal taste into an objective defect or use an invented score to rank the work.

For example, “the inner hatching competes with the animal's gaze at the intended header size; retain the face and contour, compare lower hatching density with the pose unchanged” is actionable. “Make it more professional” is not a diagnosis. Preserve the actual user reaction separately from this interpretation, and check the resulting change in the next round.

This adapts the specificity, rationale, alternatives and stage sensitivity of design-critique, attributed in the [source ledger](../research/sources.md#method-and-skill-references). Its screen-specific navigation/touch/contrast checklist applies to a workbench where relevant, not automatically to logo geometry. Under a no-image-reading instruction, base the critique on available source facts or attributed user descriptions and state that visual review was not performed.

## Inspection and evidence

Respect the session's image inspection policy. If the user asks not to read generated images, do not call image-reading tools or claim to have inspected the rendered result. You can create boards, inspect file structure and rendering status, and continue from the user's actual feedback. If image inspection is allowed and useful, declare the question and read only material that can affect that decision, following the project's visual-review guidance.

Separate static brand references from interactive UI acceptance. A rendered board does not require pretending that the product UI passed accessibility tests. A running workbench does require its actual accessibility and interaction checks before UI acceptance. A logo's image alternative and a linked logo's accessible name belong to its application.

End with the selected or retained qualities, changed hypotheses, correct target versions and the next action. A preference is not automatically final acceptance, and final acceptance of an asset is not authorization to publish it.
