---
schema: document
description: Fictional examples of schema inference, version and scope guards, and unknown-effect recovery
---

# Fictional contract examples

The Zod/TypeScript snippets illustrate a selected schema stack. Use equivalent supported facilities for another stack; these examples do not select a project dependency.

These examples illustrate the method. They are not Hypit product definitions, runnable package fixtures or evidence that any generator/server exists. Use the selected stack's actual native contract APIs in a product assignment.

## One schema, derived types and a separate state rule

Consider a document-review tool. A document's stable identity differs from its version. A reviewer approves one version in one workspace; the operation must not approve a newer document silently.

```ts
import { z } from "zod";

const ApprovalInput = z.object({
  workspaceId: z.string().min(1),
  documentId: z.string().min(1),
  expectedVersion: z.number().int().nonnegative(),
  note: z.string().default(""),
});

type ApprovalArguments = z.input<typeof ApprovalInput>;
type ParsedApproval = z.output<typeof ApprovalInput>;
```

`ApprovalArguments` allows omission of `note`; `ParsedApproval` contains the default. They are inferred from one schema. Native oRPC procedure/client inference supplies the actual caller signature; these illustrative aliases do not replace that inference. Supply authenticated actor context through the trusted invocation boundary, rather than trusting a caller's claimed permissions.

Parsing establishes shape and value constraints. It cannot establish that the workspace owns the document, that the actor may approve it, that the expected version is current or that it remains unchanged while approval is admitted. Those guards belong to the owning operation. A future atomic admission mechanism remains an implementation obligation.

| Given | Requested action | Specified result |
| --- | --- | --- |
| Document `d7`, workspace `w1`, version 8; actor may approve | Approve `d7` in `w1` at version 8 | Accepted approval names that fixed version; unrelated documents remain unchanged |
| Same resource at version 9 | Approve expected version 8 | Version conflict; no approval silently transferred to version 9 |
| Document belongs to `w2` | Approve it under `w1` | Scoped refusal under the declared disclosure policy; no mutation |
| Current scope/version but actor lacks authority | Approve | Permission refusal; schema validity supplies no permission |

A compile-only caller can handle accepted/conflict/refused variants. A script can return those variants. A finite state Mock can compare the supplied version and workspace to its modeled facts and verify unchanged-state rejection. None proves that the real implementation orders simultaneous edits and approvals correctly.

## A native value is not automatically JSON

Suppose an existing package owns an expiry value containing a `Date`. Import that type instead of hand-copying it into another entity. If the public protocol uses a UTC timestamp string, the boundary owns a named timestamp schema and declared encode/decode mapping to that imported value. Infer the wire type from its schema. State precision, timezone and invalid-string behavior, and whether decoding preserves the precision the domain requires.

This is a deliberate representation boundary. It does not justify changing the native type or asserting that the string schema is the complete native domain model. A type-level correspondence check does not prove the conversion works; conversion fixtures or later adapter evidence must cover the deciding loss cases. If no supported TS-to-schema tool exists, record that fact rather than claiming automatic runtime reflection.

## Unknown effect and continuation keep their identity

Consider a publishing request identified by `p42`. Its remote submission may succeed while the reply is lost. The following is a specification walkthrough, not an implemented publisher.

| Step/input | Known facts and permitted next action | Prohibited inference |
| --- | --- | --- |
| Plan accepted | Retain original intent, scope, baseline and request identity `p42` | Acceptance proves publication completed |
| Simulated remote dispatch; reply lost | Record unknown completion for `p42`; query that request only if the declared capability supports it | Timeout proves no publication, or permits an automatic new request |
| User closes observation | Release observer state; retain publishing identity and last known result | Closing the observer cancelled publication |
| Query returns original receipt | Update the observation for `p42` under the declared result/version policy | A receipt for another request or baseline is interchangeable |
| Capability cannot query | Keep unknown and expose the stated recovery limit | Invent a receipt or change unknown to a safe-to-retry failure |

If the plan instead requires a user decision before dispatch, model the decision's identity, question/options, scope and baseline. An accepted reply continues the same work after rechecking those facts; an obsolete reply is rejected without dispatch. A script must not equate “any reply received” with permission to proceed.

A finite model may inject remote receipts, cancellation acknowledgements or query failures and check selected traces. Label any assumed idempotence or atomicity. Real provider capability, receipt retention and restart survival remain obligations for later authorized implementation.
