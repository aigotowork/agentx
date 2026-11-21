---
schema: task
status: completed
role: debugger
description: Remove the concurrent Product Execution SHA-256 format build blocker.
agents: [{"scope":"/home/long/works/longwork/agentx/.agentx","generation":"a6195a5b-1dd4-4a6d-87d4-1f18f1b664fb","participant":{"canonical_name":"gevima","incarnation_bits":4745316266040901121}}]
---

The project request-isolation package failed at `src/messaging/inbox_store.rs:262` with E0277: sha2 0.11 digest does not implement LowerHex. See `.artifacts/daemon-isolation-integration-20261120/package.log`. A second producer in `src/integrations/codex/product_execution.rs` appears to have the same format. Fix the two boundaries together so the existing lowercase `sha256:<64 hex>` proof contract and any tests stay unchanged, preferably through one small authoritative helper. Verify by source/targeted checks only; the parent owns Cargo, packaging and all runtime transitions. Coordinate with the Product Execution editor if active; do not overwrite unrelated work. Report completion when the function sites agree and have no compile blocker.

## Diagnosis and handoff

The package log confirms E0277 at the old verifier: `sha2` 0.11.0 returns a digest array without `LowerHex`. At intake, the working tree already had the complete source correction: [`ProductInput::digest`](../src/messaging/inbox.rs#L104) writes each byte as two lowercase hex digits after `sha256:`. The [native proof producer](../src/integrations/codex/product_execution.rs#L104) and [mailbox verifier](../src/messaging/inbox_store.rs#L261) both call it with the frozen `formatted_context`. No Rust source or test files were edited for this handoff.

The focused probe used `rustc --edition=2024 --crate-name product_digest_probe -L dependency=target/release/deps --extern sha2=target/release/deps/libsha2-8ab645e6ee40c0a6.rlib <temporary probe.rs> -o <temporary probe>`, with the dependency file confirming `sha2` 0.11.0. The old expression reproduced E0277. The actual helper extracted from source compiled, and its output matched Python `hashlib.sha256` and `sha256:[0-9a-f]{64}` for empty text, `abc`, `formatted café 🐈\n` (with a trailing newline), and `leading-zero-182` (digest starts with `00`). Source assertions confirmed both call sites. The parent owns the next package build and any runtime transition; no Cargo, packaging or deployment was run for this Task.
