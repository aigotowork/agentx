---
schema: document
description: Rationale and workflow for a focused professional team delivering AgentX domain and daemon ownership.
---

# Team design basis

## Why separate contexts

This change crosses independently complex responsibilities: local process/IPC lifetime, SQLite identity and delivery consistency, terminal/Provider protocols, product clients, and independent acceptance. One context handling all details would mix different identity, effect and lifetime models. Separate owners allow genuinely independent investigation and implementation once native contracts exist. The architecture owner maintains the composed model; the controller maintains responsibilities and readiness rather than authoring specialist solutions.

## Method selection

The project already has usable Rust-native contracts, engineering/state design, implementation and scenario/integration guidance, plus source-owned Provider and database context. The [team entry](AGENTS.md) composes these into six bounded professional profiles. No Skill installation or invented general-purpose Agent framework is needed. Use AgentX for team lifecycle and communication; the [orchestration protocol](orchestration.md) owns the current coordination rules. Actual runtime acceptance and tool readiness are recorded by the controller, not inferred from the role label.

## User and Agent fit

The user's intended result is a working, maintainable domain-driven single-binary system with unique DB ownership and retained Provider behavior. A stack of design documents alone is not completion. An arriving specialist sees a Task, shared invariants, relevant native contracts and a bounded source owner; it does not need every sibling transcript. After compaction it can recover authority, inputs and next action from files. Source/model ownership and retained evidence keep this context useful after the temporary team exits.

## Workflow and limits

Architecture/state mapping and independent acceptance design can start immediately. Daemon, consistency, runtime and client specialists can investigate their own entry points and present interface needs in parallel. Substantial behavior waits only on the relevant native boundary, not on unrelated work. Integration owns heavy verification and build resources. The controller owns dispatch/dependencies and routes technical conflicts to architecture plus the affected domain owners. No worker recursively delegates; initial capacity is six cohesive specialists.

Actual Tasks own progress. The [Delivery](delivery.md) owns stage acceptance. The [controller](controller.md) owns live bindings. Reusing completed Codex workers, treating an idle status as free capacity, or silently inventing an unverified production guarantee would violate this model.

## Workload adjustment: relay specialization

After the initial client migration, the owner reported concrete progress (35 scoped passing cases and typed start/TUI/query work) but substantial ordinary CLI direct-DB paths and relay control reentry remained. Relay's peer replication/cursor/replay/network behavior has a different context from CLI grammar and display. A bounded dedicated relay specialist therefore isolates genuine domain context and parallelizes a large independent remaining slice. The original native contracts and source stay in place; an explicit old-owner release precedes the new Paseo assignment. This adds one cohesive profession, not a file-per-Agent split or a second integration/review layer.
