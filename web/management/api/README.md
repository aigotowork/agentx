# Native management declarations

[Rust declarations](../../../src/management/contract.rs) own the wire contract. The [public SDK generation workflow](../../../packages/agentx-client/README.md) exports the actual OpenAPI contract and regenerates the SDK. This directory consumes SDK types and should not contain an independent generator or wire schema.

`RoleDefinitionView.profile` is nullable catalogue metadata; `RoleView.profile` is the concrete adopted profile. Do not replace adopted facts using current catalogue defaults.
