---
schema: document
description: Build dedicated playable or usable works that connect implemented rules to reader experience and transferable explanation
---

# Executable explanations

Use for a dedicated playground, small application or game when the reader needs to use a rule, encounter its consequences and try a different approach. Such a work can be the core publication and justify substantial development. Determine the needed experience before selecting React, an independent project or an embed.

## Connect the concept to the experience

Establish concept and conditions → concrete implementation rule → reader action → observable consequence → explanation of cause → changed case or retry. A work can begin with use and reveal theory on demand; do not require reading a full essay before entry.

For each important concept, name the rule that implements it and the operation that makes it visible. Give readers enough feedback to connect the result to that rule. Useful mechanisms include comparison switches, state overlays, action traces, local source, keyframe replay and concise causal feedback. Select only views that help the lesson; an explanatory game need not become a dashboard.

| Topic | Candidate work and meaningful operation | Necessary limit |
| --- | --- | --- |
| Responsive layout | Editable HTML/CSS and container size, with reset and aligned output | One browser/size is not all browser support |
| Input buffering | Playable movement/collision/retry; compare buffered input under equivalent timing | Observed rules do not establish all players' preferences |
| Physics feedback | Original angle/force challenge, trajectory option and repeated same-level comparison | Model, level and objective bound the result |
| Cache identity | Synthetic requests varying locale, identity and representation; expose key and returned object | Local rules do not measure an edge network or prove real authorization |
| Migration compatibility | Change consumer/service versions through mixed states and recovery | Only the declared version/failure matrix is covered |

These are design candidates. If a claim depends on genuine browser behavior, execute that behavior rather than drawing a fixed result. If the work is a teaching simulation, label it. Simply playing successfully does not establish that the reader understood the rule.

## Complete the task loop that makes the lesson real

In the existing brief, state:

- Which concept becomes usable and which choices expose it.
- The required goal, actions, feedback, failure and retry loop.
- Which implementation details change that experience and which peripheral features can be omitted.
- How the reader can connect an outcome to a rule and compare a changed condition.
- The source and runnable version tied to the explanation, and access if embedding fails.

An input-buffering game may need careful control response and a full retry loop; accounts and leaderboards add no necessary explanation. A responsive-layout work may need genuine editing and resizing; it need not become a complete online IDE. Scope “complete” by whether the intended experience works. A fixed success animation is inadequate when operation is the lesson.

## Choose the hosting boundary

| Candidate | Fits when | Define and verify |
| --- | --- | --- |
| Inline component | Parameters, state or labels closely connect to surrounding content | Inputs, local state, loading/unmount, style boundaries and host impact |
| Independent project with iframe | A game/editor needs its own build, styles or runtime | Address and fixed version, embedded/full-page routes, size, focus, touch, scroll and failure |
| Self-hosted code playground | Editing and repeatable output need long-term ownership | Source, dependencies, reset, allowed execution and work version |
| External example/embed | An existing work accurately serves the task | Author/source, use conditions, external dependencies and failure route; preserve reproducibility beyond a mutable link |
| Full page or download | Width, input, performance or deeper exploration need a separate surface | Accessible launch/run instructions and the same version relationship |

These are options, not claims of existing host support. An iframe does not automatically solve focus, mobile input, loading or cross-origin communication. Define finite messages and allowed origins if communication is needed. Grant only the capabilities the work needs; do not give a content editor arbitrary script execution.

A work may be developed independently before selecting its page connection. A generic experiment platform is unnecessary. Verify actual embed/media/source policies through [renderer checks](../publication/renderer-and-release.md). Keep the article and runnable version associated so later updates do not silently change the demonstrated rules.

## Judge the complete experience

Apply the [teaching contract and acceptance](interactive-experiments.md). Check that actions affect relevant results, feedback explains causality, alternatives keep equivalent conditions, retry exposes new cases and the hosting choice allows the intended readers to use it. A short surrounding article can still contain rigorous reasoning through inspectable rules and feedback.

Recordings, loops or keyframes can introduce the work or preserve its central observations when unavailable. They need their own [material composition](materials-and-composition.md), and cannot claim the complete exploration of a playable version. State what only the running work supplies.

Successful construction establishes that the checked instance runs. General correctness, production scale, learning improvement and audience preference need their own evidence. Production effort and component count are not quality measures.
