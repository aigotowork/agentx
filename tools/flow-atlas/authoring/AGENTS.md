# Source authoring

Read [README](README.md), [public contracts](contracts.ts) and [the bound Task](../../../.controller/domain-daemon/tasks/flow-atlas-ts-authoring.md). This directory owns a bounded authoring library, not the full Definition/Scenario compiler, a scheduler or a live runtime client.

`handles.ts` owns identity and native method storage; `compiler.ts` owns static lint and portable inspection; `conformance.ts` owns explicit instrumentation and observed path checks. `adapters/zod.ts` is the only core integration that imports Zod. Native domain schemas and handlers live in `examples/native-contracts.ts`; the design example imports them without copying fields. Core modules never import examples or inspect arbitrary application source.

Read [lint rules and maintenance](lint.md) for exact diagnostic groups, configurable policy, CI usage, source ownership and current case coverage. Update that index with a material rule change; source remains authoritative. Do not imply whole-repository AST/import analysis from declared-handle checks.

Inspect/export/lint may call a schema conversion adapter but must never invoke native methods, classifiers or schema parse hooks. Runtime wrappers invoke only on explicit consumer request. Metadata/docs/extensions are inert, JSON portable data. Keep failures, uncertainty and missing responses distinct from successful returns. Follow the Task's execution ownership: controller runs all Node/TypeScript/tests/build/browser checks for this delivery.
