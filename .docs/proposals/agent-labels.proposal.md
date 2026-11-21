---
schema: proposal
status: accepted
description: Deliver scalar Agent labels with indexed persistence, native filtering and usable management consumers
models: [../agent-classification.md]
deliveries: [../../.controller/agent-labels/implementation.delivery.md]
---

# Agent labels implementation

The user accepted the [owning design](../agent-classification.md), emphasized concrete database/index performance, and explicitly requested a dedicated Controller to organize Agents and deliver the system. Implement that model through the existing Rust/SQLite/native service and selected CLI/management/Web consumers. The design owns exact semantics; do not duplicate it here or infer implemented support from the design SQL/browser exercises.

Scope: multiple scalar key/value labels per Agent; per-key atomic edits and revision conflict/replay recovery; indexed current-Agent filtering; creation/resume/fork identity behavior; native-derived management contracts; usable labels editing/filtering/grouping in the existing management consumer. Separate multi-value tags, implicit inheritance/default labels, historical selectors, label-addressed messaging and automation engines are excluded. Preserve native identity, runtime ownership and ongoing metadata-removal guarantees.

The [implementation Delivery](../../.controller/agent-labels/implementation.delivery.md) owns execution and acceptance. Isolated source/build/native/API/browser evidence is required; neither the accepted design nor synthetic SQL warm timings establish production integration or live service adoption.
