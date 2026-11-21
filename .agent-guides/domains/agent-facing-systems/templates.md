---
schema: document
status: active
description: Compact scaffolds for adopting the agent-facing systems method in another project
---

# Adaptable templates

Use the sections needed for the current task. Replace uppercase placeholders, illustrative domain names and paths with actual project values before publishing or executing anything. These are authoring scaffolds, not a new schema language, executable application or required repository layout. Reuse an adequate existing project form instead of creating a duplicate.

## Adoption record and module skeleton

Place the short record in the existing design/task and link it from the normal project entry.

```markdown
# Agent-facing capability: CAPABILITY

Caller / task: WHO needs WHAT through WHICH supported runtime.
Completion: OBSERVABLE RESULT, including relevant lifetime and persistence.
Scope: SELECTED SURFACES; explicitly unsupported: EXCLUDED MODES.
Domain owner / object / observations: OWNER; IDENTITY; CURRENT FACTS.
Native source: SCHEMA PATH; SERVICE/ERROR PATH; OPERATION/METADATA PATH.
Projection owners: CLI / HTTP / SDK / MCP sources actually used.
Discovery: PUBLIC ENTRY; RECIPE; CANONICAL SKILL + supported registration paths.
Effects / recovery: PRECONDITIONS; CHANGES; REPLAY POLICY; UNKNOWN-OUTCOME QUERY.
Provider choice: FIXED VERSIONS; REQUIRED GUARANTEE; SOURCE; compatibility reason.
Evidence: CHECK COMMAND / CONSUMER CASE / RECEIPT; remaining limits.
Next action: OWNER and CONCRETE STEP, or complete for the declared scope.
```

Illustrative responsibility skeleton for a TypeScript service:

```text
src/<domain>/contracts/   native schemas, operations, service/errors
src/<domain>/            domain behavior and owned state
src/<adapter>/           CLI, HTTP or MCP policy; only selected adapters
src/generated/           maintained derived outputs, if generation is useful
scripts/                 repeatable generation/check entry, if needed
docs/                    selected caller recipes and runtime/setup guidance
skills/<capability>/      canonical SKILL.md and packaged references
llms.txt                 short public documentation index, if adopted
```

The names express responsibilities, not mandatory folders. Keep private module helpers within their owner and use in-process native contracts where sufficient.

## Native oRPC + Zod scaffold

This example exposes one read of an illustrative domain `Record`. Replace it with real vocabulary and semantics. It establishes declarations only; domain implementation, provider binding and adapter generation remain project-owned.

```ts
import { oc } from '@orpc/contract';
import { z } from 'zod';

export const recordSchema = z.object({ id: z.string().min(1), title: z.string() });
export const operationSchemas = {
  read: {
    input: z.object({ id: z.string().min(1) }),
    output: recordSchema,
  },
};
export const domainErrors = {
  NOT_FOUND: { status: 404, message: 'Record not found' },
};
export type RecordService = {
  [K in keyof typeof operationSchemas]: (
    input: z.output<(typeof operationSchemas)[K]['input']>
  ) => Promise<z.output<(typeof operationSchemas)[K]['output']>>;
};
type OperationMeta = {
  resource: 'record';
  effects: { readOnly: boolean; idempotent: boolean; destructive: boolean; externalAccess: boolean };
  cli: { command: string; description: string };
  mcp: { name: string; description: string };
};
const base = oc.$meta<OperationMeta>().errors(domainErrors);
export const contract = {
  read: base
    .meta({
      resource: 'record',
      effects: { readOnly: true, idempotent: true, destructive: false, externalAccess: false },
      cli: { command: 'show', description: 'Read one record' },
      mcp: { name: 'record_read', description: 'Read one record' },
    })
    .route({ method: 'POST', path: '/read', operationId: 'record_read' })
    .input(operationSchemas.read.input)
    .output(operationSchemas.read.output),
};
```

Adapt API usage to the project's pinned versions. Keep additional fields only when consumed. Add mutation/lifecycle/error meaning beside real operations, not by guessing from this read-only example. Derive the native client/provider from `contract`, and use supported schema conversion for OpenAPI/MCP. The [Browser Environments example](browser-environments.md) links a maintained generator and authored adapters.

## Projection policy record

Use one row per meaningful mapping, omitting unused surfaces.

| Native operation / schema | Surface name and shape | Loss / policy | Enforced by | Evidence |
| --- | --- | --- | --- | --- |
| OPERATION + INPUT/OUTPUT SOURCE | CLI COMMAND / HTTP ROUTE / SDK METHOD / MCP TOOL | ABSENCE, PARSING, ENVELOPE, ERROR OR LIFETIME DIFFERENCE | OWNER PATH | REAL CONSUMER CASE |

For effects, state `target + input + relevant state -> result + changes`, then define replay and interruption behavior. Where the domain has no operation receipt or reconciliation capability, state that limitation rather than creating a generic job framework.

## llms entry

```markdown
# PRODUCT

> CAPABILITY and intended callers in one sentence.

IMPORTANT OPERATING BOUNDARY and ordinary service/install prerequisite.
Read only the guide matching the selected task.

## Tasks

- [Quick start](docs/agent-quickstart.md): Select the target and finish the first task.
- [CALLER](docs/agent-CALLER.md): Exact supported caller recipe.
- [Recovery](docs/agent-workflows.md): Reconcile interrupted work and clean up owned resources.

## Optional

- [Skill](skills/CAPABILITY/SKILL.md): Operating procedure for supported Skill hosts.
- [API contract](openapi.json): Exact HTTP operations and schemas, if published.
- [Discovery/setup](docs/agent-discovery.md): Supported Skill and MCP installation.
```

Delete unused links and verify actual published URLs. Keep any requested filename alias pointing to this canonical content. Do not claim that clients automatically load it.

## Skill entry and packaging

Preserve native Skill frontmatter. Replace the description with concrete activation conditions.

```markdown
---
name: CAPABILITY
description: Perform TASK with PRODUCT. Use when the user needs CONCRETE SCENARIOS.
compatibility: Requires ACTUAL RUNTIME, PRODUCT INSTALLATION and SERVICE prerequisites.
---

# CAPABILITY

Use PRODUCT for OWNED RESPONSIBILITY; use OTHER TOOL for ITS RESPONSIBILITY.

1. Read [quick start](references/agent-quickstart.md); select the user-intended target.
2. Invoke EXACT SUPPORTED OPERATION and retain RESULT IDENTITY needed for the next step.
3. Read only the selected [caller guide](references/agent-CALLER.md).
4. Interpret the result and release only CALLER-OWNED RESOURCES.

For interrupted operations and failures, read [recovery](references/agent-workflows.md).
State the capability's specific lifetime/effect boundary here when it affects ordinary use.
```

Package all required references under the Skill directory, or declare an unavoidable external prerequisite. Register it in the selected host's supported scope. If repository references use symlinks, materialize them for standalone distribution and inspect the copy outside the checkout. [Discovery](discovery.md#package-a-real-agent-skill) supplies the decision criteria and host sources.

## Caller recipe

```markdown
# CALLER: TASK

Use this recipe when CONCRETE INTENT. PRODUCT owns LIFECYCLE; CALLER owns ATTACHMENT.

Prerequisites: SUPPORTED VERSIONS, installed executable, running endpoint, identity/access.

| Step | Exact invocation / input | Expected result / effect |
| --- | --- | --- |
| Select target | ACTUAL COMMAND | IDENTITY and preconditions |
| Perform operation | ACTUAL COMMAND with explained placeholders | MACHINE RESULT and completion meaning |
| Observe / continue | ACTUAL COMMAND | PROMISED OUTCOME through relevant boundary |
| Release attachment | ACTUAL COMMAND | Which work/data remains and which is stopped |

Configuration: DEFAULTS; CLI/ENV/config sources and precedence; path/endpoint scope.
Failure: STABLE ERROR or unknown outcome -> QUERY/RECONCILE/RETRY under declared policy.
Evidence: tested VERSION/CASE/RECEIPT; remaining unsupported or unexecuted modes.
Reference: link exact contract/setup rather than copying all field definitions.
```

Replace every `ACTUAL COMMAND` with literal supported syntax and explain placeholders and quoting. Include only real operations; a conceptual sequence is insufficient as a caller recipe.

## Evidence receipt

Use the existing evidence format if present. This compact JSON is illustrative, not a required schema or new ledger.

```json
{
  "scope": "SELECTED_BOUNDARY",
  "versions": { "provider": "PINNED_VERSION", "caller": "PINNED_VERSION" },
  "target": "ISOLATED_FIXTURE_IDENTITY",
  "cases": [{ "case": "MEANINGFUL_CASE", "outcome": "pass", "artifact": "RECEIPT_PATH" }],
  "limits": ["UNEXECUTED_OR_UNSUPPORTED_CASE"]
}
```

Record invalid-input non-dispatch, a domain failure or unknown-outcome path when relevant, as well as success and cleanup. Keep check commands and narrative limitations in the same existing delivery context.
