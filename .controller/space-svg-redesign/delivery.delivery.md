---
schema: delivery
status: completed
description: Create and experience an original SVG game-like office and conversation prototype for Space.
stages:
  prototype:
    description: Compose a new art direction and complete core loop, then operate and assess it without instructional copy.
    tasks: ["./tasks/prototype.task.md", "./tasks/evaluation.task.md"]
---

# Space game-like SVG prototype

The user rejected the previous design's visual quality, game feel and explanation-led interface. The main Space experience is an illustrated, lively office: people, purposeful rooms, movement and interactions; select a character and converse in a deliberately designed surface. The requested visual cue combines hand-inked, slightly eccentric survival-game illustration with a mosaic/pixel character. Translate qualities into an original visual language, without reproducing another game's characters, UI, maps, or assets. Author drawings in SVG from a blank sheet. Design the Space workspace only; Management, Observability, current applications, and all running services are outside scope.

## What went wrong in the previous round

The [interface requirements](../../.agent-guides/scenarios/interface-design.md#trust-users-make-the-interface-explain-itself) already direct the designer to use structure, controls and feedback before explanatory prose. No rule was missing. The prior [assignment](../web-product-design/tasks/space-prototype.task.md) asked the author to inspect the existing Space, consume a system shared with Management, and satisfy a broad parity/behavior set. It called for deliberate visuals but did not put an original office art direction, scene-first opening or discovery without help text ahead of breadth. The author delivered the behavior but used a general tool-workspace hierarchy; the [prior opening](../web-product-design/preview/space/index.html#L99) used a slogan and supportive copy in the central work area rather than putting users into an actionable scene. The Controller [noticed the large welcome region](../web-product-design/delivery.delivery.md#controller-screenshot-reading) and recorded it as a future question instead of treating first-screen comprehension and visual fit as a failure. The [prior evaluation](../web-product-design/evidence/candidate/README.md) measured browser paths, not whether someone seeing the interface without instructions could make sense of it. The resulting evaluation is valid for those local behavior claims only. The Controller owns the brief and acceptance mismatch; the authoring lapse is real, but evidence does not show a missing role or a general Agent capability failure.

## Design decisions and authority

The new prototyper owns visual exploration, composition, art, motion, click targets, disclosure and conversation design. Use no old implementation or prototype references. Establish an original art direction before filling out ancillary features. Prioritize the core loop: enter and read the room → recognize who is doing what → pick a person → converse → return to their location or find another person. Use user-language labels, names, real working/demo conversation content, brief contextual consequences and action feedback. Keep design rationale, instructions, architecture, feature tours and debug/simulation controls out of the regular workspace. If a view depends on explanatory copy, redesign hierarchy, affordance or feedback. A few truthful words where a user makes a decision are welcome. Broader parity is a future implementation question, not an art constraint. A compact list is valuable as an alternate path to a known person; no second product theme or required tour.

No automatic adoption of the previous design-system contract. Two preview sizes matter: desktop scene and a narrow touch/sidebar reader. A named reduced-motion treatment and nonspatial keyboard path protect the primary loop. Prototype state and all effects are synthetic, no external dependencies or runtime APIs. Put preview status in the external review entry or a small local marker, never a teaching panel.

## Evaluation

Evaluate one coherent SVG candidate in a browser. The evaluator receives task intent alone for the first encounter, not the task instructions, control names or happy path recipe. Capture the first screen and actual actions at desktop and narrow widths; say what visual cue prompted each step or where there was a stall. Review the office at rest and in motion, selection, conversation open, return and a list path. Mark task breakage, hidden targets, visual mismatch or explanatory copy serving as a crutch as material. A working selector and a pretty static screenshot are separate, insufficient signals. Review at natural scale and write concrete findings, then allow one coherent correction/recheck for defects. Expert walkthrough is not a user study; art direction remains subject to user judgment.

## Preview

LAN review surface: http://192.168.2.178:18985/. Dedicated static server serves this Delivery's `preview/` only; no existing preview, daemon or service is changed. Reopen or refresh manually for updated static pages. Started from repo root via `python3 -m http.server 18985 --bind 192.168.2.178 --directory .controller/space-svg-redesign/preview`. The [prototyper](tasks/prototype.task.md) owns the candidate and editable SVGs; the [evaluator](tasks/evaluation.task.md) owns observed evidence. Controller kula owns acceptance and route from feedback to design revisions, and will personally inspect the first complete visual composition and screenshots.

## Completed outcome and evidence

`muse` owned the [original SVG and interactions](tasks/prototype.task.md), and `zore` owned the [independent first encounter and focused browser replay](evidence/evaluation/README.md). Both Tasks are complete. The preview at [the dedicated LAN entry](http://192.168.2.178:18985/) stays available for the user's visual review; it is synthetic and isolated.

The original art and core loop are usable: the Signal Workshop and Glasshouse use distinct props and lighting; five characters across them are individually reachable via the room and the list. The Workshop shows three concurrent roles and a visible courier-to-technician exchange. Selecting an actor opens a matching portrait and usable letter, retaining the room, draft and focus on return. No welcome slogan, tutorial, decorative ID, or explanatory panel occupies the working interface. The [author handoff](evidence/author/README.md) locates editable SVGs, the local interaction bindings and screenshots.

Independent Chrome evaluation at 1440×900 and 390×844 found the courier from scene cues without coaching, followed the motion, and verified separate actor targets, cross-room selection, draft, simulated message/reply, keyboard focus, and reduced motion. It also identified the E1 association issue on the examined version. In the final candidate a temporary dotted floor cue ties the moving courier to his still tappable nameplate. The author verified both hit paths and preserved the rest of the walkthrough; the Controller visually checked [desktop](evidence/author/desktop-handoff.png) and [narrow](evidence/author/narrow-handoff.png) captures after the fix. These pieces of evidence are separate; the independent report is for the candidate before the final E1-only change.

Accepted for user visual review as a gameplay and interaction prototype. Two rooms and five characters do not establish higher-population usability; real users, physical device keyboards, live messaging and production integration were not exercised. Future production binding should use the proposed TypeScript domain model once implemented, keeping room art, motion and UI state local. No changes to the existing application or services.
