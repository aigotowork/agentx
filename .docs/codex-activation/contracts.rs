//! Contract navigation only; authoritative declarations are production source.
//! Do not reintroduce editable Rust type/trait copies in this design directory.
//!
//! Generic identity and attachment fences:
//!   ../../src/agents/attachments.rs
//! Exclusive prepared batches, delivery evidence and predecessor recovery:
//!   ../../src/messaging/inbox.rs
//! Required activation port and fenced progress events:
//!   ../../src/messaging/activation.rs
//! Codex root qualification and home/thread mapping:
//!   ../../src/integrations/codex/roots.rs
//! Shared native backend acquisition and framed connection ownership:
//!   ../../src/integrations/codex/service.rs
//! Per-connection bidirectional request ownership:
//!   ../../src/integrations/codex/protocol.rs
//!
//! File ownership, concrete constructors and effect ordering:
//!   ../../.controller/codex-shared-runtime/contracts-handoff.md
//! Process/state mapping: protocol.md. Native evidence: protocol-feasibility.md.
//! Production declarations alone do not establish implementation acceptance;
//! see acceptance.md and the current Delivery tasks for actual support/evidence.
