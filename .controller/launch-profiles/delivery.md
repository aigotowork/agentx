---
schema: delivery
description: Implement reusable launch profiles through existing configuration, launch and relay boundaries.
status: completed
---

# Launch profiles implementation

The user authorized implementation of the [proposal](../../.docs/proposals/launch-profiles.md). One inline implementer owns this bounded extension to existing configuration and launch paths; no delegated workflow is needed.

## Structure and acceptance

Config owns profile definitions and locked file edits. Runtime owns selection, argument composition and frozen launch configuration. CLI owns grammar and presentation. Participants own durable launch provenance, and Relay resolves selectors on the target. Existing native Provider adapters retain argument semantics.

Implement native structures and module boundaries first, then file operations/resolution, then CLI/local/remote/history integration. Validate profile CRUD/default references, explicit and ENV selection, argv overrides/replace, passive preview, preserved resume and remote target resolution. Use disposable scopes; preserve installed programs and live project services. Build through the existing package recipe, retaining any previous project executable.

## Current state

Implementation, scoped regression and release packaging are complete. The proposal owns design decisions; [usage](../../.docs/launch-profiles.md) owns the user-facing workflow. Config CRUD/defaults, one-pass resolution, CLI previews, target-side Relay selection and native historical provenance are integrated. Schema v24 adds one nullable/default-empty profile column with in-place upgrade and stamped-schema repair. This completion is the documented local implementation outcome, not a claim of real cross-device or all-Provider acceptance.

## Verification and limits

- Launch-filtered native tests: 373 passed, one existing ignored case (`/tmp/agentx-profile-tests.log`).
- Scoped config, native history and schema checks: 11 passed; a new remote-parser test exposed missing serde defaults for omitted selector booleans. The selector now uses explicit default/unknown-field validation; the corrected remote test passed in the subsequent integration run.
- Corrected remote target-selection, additive schema repair and provenance checks: 3 passed. Existing/new stdio fixture tests in the same run: 16 passed (`/tmp/agentx-profile-integration.log`).
- Follow-up stdio profile resolution after project direnv changes scope: one passed (`/tmp/agentx-profile-env.log`). The early stdio boundary identifies project preparation without resolving profiles against the pre-environment scope.
- Final profile/config, CLI help/router, environment and native resume regression selection: 88 passed (`/tmp/agentx-profile-regression.log`).
- Additional affected Relay control checks: 60 passed; the existing exact advertised-capability fixture required the two newly implemented capabilities. Its updated expectation passed the targeted recheck (`/tmp/agentx-profile-relay.log`, `/tmp/agentx-profile-capabilities.log`). This changed only a test expectation; production Relay code was unchanged during that recheck.
- [Reusable public CLI checks](../../tests/launch_profiles.py): 11 scenario groups passed against the debug executable; report at `/tmp/agentx-profile-debug-evidence.json`. These use isolated HOME/config/data and failing Provider/direnv sentinels to verify passive preview.
- Affected patch whitespace and design/usage links passed. No actual model invocation, real cross-device Relay, Windows/macOS runtime or browser profile editor has been exercised. Remote evidence establishes the target preparation/parser logic, not network delivery.

## Release artifact and closure

The existing `just package` recipe succeeded, including the executable version check (`agentx 0.7.31`). The wrapper verified restoration of the previous `.bin/agentx` by SHA256 and retained the new executable as [`.bin/agentx-profiles`](../../.bin/agentx-profiles). Its SHA256 is `d591a12847b6a7c32d86ad6c40dd1f4a803460707cd9e7b90121dae7ae2e486d`. The last subsequent Rust source edit changed only the test-only expected Relay capability list; production behavior was unchanged.

The same 11 public CLI scenario groups passed against that release artifact; [retained evidence](cli-evidence.json) records its exact hash and checks. Installed binaries, current project executable/services and the separate LAN demo were preserved; no daemon restart or default configuration migration was performed. Ordinary adoption into a live existing scope remains a separate runtime operation. There is no remaining implementation work for this bounded scope; real multi-device Relay and non-Linux/provider execution remain explicit evidence limits.
