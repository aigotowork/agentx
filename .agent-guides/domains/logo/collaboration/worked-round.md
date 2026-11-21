---
schema: document
description: Illustrative logo round showing distinct concepts, mixed preferences, fixed activity inputs and recoverable continuation
---

# A round that can be resumed

This constructed example demonstrates the [workflow](../workflow.md), [feedback method](conversation.md) and [record semantics](records.md). Names, files, operation IDs and outcomes below are illustrative; this package contains no corresponding artwork or running tool. Real use writes only the needed context into the consumer's existing design/task record.

## Brief and meaningful alternatives

The request is: “Create a warm, inventive identifier for Loomfield, a maker's collaboration tool. Show several genuinely different directions. We like crafted surfaces, but keep the name readable.” The working brief records the exact name, known header and workshop-badge uses, desired warmth/invention and open stylistic choices. Header dimensions and badge process remain unknown; they are not invented production constraints.

The Agent proposes drawable territories with visible differences:

| ID | Proposition | Organizing form | Next question |
| --- | --- | --- | --- |
| D-A | Independent makers form a useful shared structure | Woven bands with a deliberate over-under relationship | Does the relationship read as connected craft or an indistinct knot? |
| D-B | A resourceful companion helps build things | Alert animal gesture with a rounded contour and supporting wordmark | Does the character fit the audience without appearing childish? |
| D-C | Work carries a maker's signature | Custom letter relationship developed into a restrained engraved stamp | Does the wordmark feel personal or institutional? |

These are different concepts; brushed and polished finishes on D-B are variants. References separately explain weave crossings, animal anatomy, lettering rhythm and metal behavior. Each retains origin, role, extracted property and permitted use/unknown status. Generated surface studies stay labeled as such.

## Preserve mixed feedback

After comparable rough sources are presented as board P-1, the user says: “B: keep the alert animal, but the interior is too busy. C: I like the engraving only, not the border. Can B feel less childish?” The record retains that statement without turning it into acceptance of B or a blanket preference for engraving in every use.

```yaml
round: R-2
brief_revision: 1
question: Can the selected animal direction retain alertness with a more mature expression?
baseline: B-master@1
feedback:
  - id: F-1
    author: user
    target: B-master@1
    statement: Keep the alert animal, but the interior is too busy; make it less childish.
    submitted: true
    retention: [subject-only, expression-only]
    requested_change: [simplify-detail, less-childish]
    vocabulary_version: 1
  - id: F-2
    author: user
    target: P-1@1/C-engraved-view
    statement: I like the engraving only, not the border.
    submitted: true
    retained_facet: material/engraved-metal
    retention: material-only
    change: Exclude C's border from the proposed hybrid.
interpretation:
  id: I-1
  author: agent
  basis: [F-1, F-2]
  hypothesis: Interior density and juvenile proportions may cause the mismatch; material may be a separate preference.
plan:
  id: R-2-plan@1
  retain: [animal-subject, alert-gesture, readable-name]
  change: [simplify-interior, compare-mature-proportions]
  control: [pose, palette]
  explore: Engraving on the retained source after the structural comparison.
  check: Compare source versions before judging material; user evaluates the mature character hypothesis.
```

The user does not need this YAML in chat. A short visible explanation can name what will survive, the controlled change and the question. Selection for further development authorizes the scoped work already requested; it does not create a deployment decision.

## Keep dispatched work and later opinion distinct

Suppose an available rendering tool starts activity A-1 with R-2-plan@1, B-master@2 and submitted feedback F-1/F-2. Its input record is fixed. While it runs, the user adds F-3: “Keep the large main emblem rich; I meant simpler only for the navigation use.”

Record F-3 against the relevant source and use. Revise the plan for future work, retain the primary cue and derive a compact adaptation only for the navigation context. Do not pretend A-1 consumed F-3 or silently mutate its inputs. Its late result returns to R-2-plan@1 even if the user is now viewing C. If A-1 produced a usable source view but failed its metal rendering, retain the view and record the missing material output.

A completed activity does not resolve whether the animal feels mature. Record included feedback separately from the feedback actually addressed. Unknown save or execution outcome remains unknown until checked through the real capability; retry uses its actual duplicate/recovery contract. An ordinary manual revision does not need a fictional background job.

## Resume and close at the promised stage

A handoff retains the current question, B source versions, fixed A-1 inputs/results, F-3's scope, the plan revision, pinned comparison and supported draft/save state. It does not need every hover or zoom event. Another Agent can continue the relevant comparison without restarting brand intake or discarding a user's narrowed preference.

If image reading is disallowed, the Agent can check SVG structure and rendering status and report the user's actual comments, while leaving its own visual craft review unperformed. It cannot claim to have seen less childish anatomy or convincing metal. If no credible material capability is available, the source/refinement can still be delivered with the metal study recorded as incomplete.

This exploration closes with the promised alternatives or scoped refinement, exact delivered source/board versions, retained/open decisions and evidence limits. Production adoption later follows the [asset owner's integration path](../evaluation.md); neither user preference nor tool completion implies publication.
