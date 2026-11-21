# Shareable Space navigation

Read [Task and coordination](task.md). User @bigboss requested that Space navigation update the URL so copying/opening it elsewhere reproduces the same page/panel/resource scene. This is an implementation request, not design-only. Existing first-load query focus is historical input; preserve its exact-identity and embedding guarantees.

Source implementation and independent Linux Chrome acceptance are completed: [accepted source outcome and rollout handoff](task.md#accepted-source-outcome-and-rollout-handoff), [canonical usage/model](../../.docs/agent-space/routes.md), [verification](verification.md#completed-independent-outcome). Selected :38023 is unchanged; live native package/adoption is handed to @bigboss separately. Original worker bindings are closed; do not revive checks or assignments without an affected change.

The implementation owner designs and implements the cohesive browser navigation model in `web/agent-space`, updates its canonical usage/module guidance and this Task, and hands off finite consumer scenarios. Controller space-responsive-vaneda owns coordination, root entry and acceptance; independent verification owns its tests/evidence. Preserve all existing user changes and services, especially :38023. Source acceptance does not imply live rollout. No recursive delegation.
