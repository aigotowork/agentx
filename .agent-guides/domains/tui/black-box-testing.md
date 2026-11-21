---
schema: scenario
description: Exercise an explicitly selected external terminal command through a finite driver and retain readable failure evidence.
tags: [tui, testing, terminal]
---

# Test an external terminal command

Use when keyboard interaction, visible text or terminal-rendered cells answer the question without source/framework hooks. Select the installed terminal-scenario or termwright component through its public README/help. No source import, browser or visible terminal window is required for a headless route. Use [framework observation](tools.md) for internal application state or a real window when native pixels/input matter.

## Discover the actual tool

Resolve the executable from the selected release/project binding and inspect version, help and capabilities before writing a plan. For termwright, supported discovery commands include `--version`, `info capabilities --json`, `info steps --json`, `info protocols --json` and `fonts`; installed help owns exact syntax. A source checkout with the same version label can still differ from the binary.

The studied termwright 0.2.0 macOS arm64 binary changed child PTY size while retaining the parser's initial dimensions. Use fixed-size sessions for that profile; use a verified PTY/parser route for dynamic resize or record separate startup-at-size cases. Do not execute a platform-specific binary on an unsupported host. Font, protocol and capture gaps remain part of the claim.

## Bind a finite scenario

Supply an explicit command argv, cwd, relevant environment, initial size, fresh output/runtime roots and deadline. A command array already contains its arguments; do not also put them in an ignored args field. Choose harmless consumer fixtures and an owned shutdown mechanism. Installed directories are read-only inputs, never destinations for runs.

Use the selected runner's own example/schema; raw termwright steps and terminal-scenario steps are not interchangeable. Give meaningful steps stable keys and expectations. Several type/press/hotkey/wait operations can form one checkpoint. Separate a recording channel's step/finish/cancel control from target input; drain bounded NDJSON output and keep intent, accepted action and actual receipt separate. An exported draft with unresolved expectations requires editing before execution.

When using raw termwright steps, the established shapes include:

```json
{
  "steps": [
    {"waitForText": {"text": "READY", "timeoutMs": 1000}},
    {"type": {"text": "hello\n"}},
    {"expectText": {"text": "GOT=hello", "timeoutMs": 1000}},
    {"notExpectText": {"text": "ERROR"}},
    {"screenshot": {"name": "received"}}
  ],
  "artifacts": {"mode": "always"}
}
```

This is a fragment for a synthetic echo target, not an executed run or complete launch configuration. The target must produce the receipt; successful input writing does not prove consumption. Retain the original full plan and actual tool identity with each fresh attempt.

## Wait and retain the failure

Prefer old state gone plus expected new state present, or an actual application revision. An already-visible footer establishes no transition. Use eventual disappearance waits where needed; an immediate negative assertion answers only the sampled moment. Text-idle checks can miss colors, cursor or pending work and are not business completion.

Use capture modes according to actual semantics: raw `always` can retain text/cells after steps, while an explicit screenshot produces PNG. Do not assume a failure automatically gets a PNG. A preceding screenshot is not the later failure image. Select a runner that attempts all checkpoint layers after failure, or use an owned daemon to capture after the failed condition and before closure; record any failed capture separately.

For raw termwright, onFailure captures failure text/cells while successful explicit screenshot steps still write their PNG; neither onFailure nor always promises an automatic failure PNG. A trace records step timing and tool observations, so retain the original plan for parameters and expectations. Use the formal scenario runner when per-step success/failure capture and retained reading are required.

Keep actual actions, waits, outcomes, text/cells/PNG, tool/version, dimensions/font, capture order and cleanup. A locally rasterized grid is not an OS screenshot. Sequential samples lack a common barrier unless the adapter establishes one. Combining marks, fallback glyphs, cursor drawing and style coverage can differ. For UI design or user-intent review, explain any image read against those limits; ordinary diagnosis uses structured evidence under the [shared image limits](../../principles.md#quality-tests-and-images). A viewport-only API does not supply raw PTY history.

## Own daemon lifetime where needed

For lower-level exploration, use the installed daemon protocol with a unique private socket, bounded readiness and one serialized client request at a time. A client that blocks the only connection cannot be interrupted by assuming another client will be admitted. Bound operations from the caller as well as through protocol timeouts. Type does not necessarily wrap bracketed paste; mouse bytes do not prove application support.

Stop input, request close and wait on the original process handle. The studied termwright close can reply with `error.code=closing`; its EOF-based status is not necessarily the child's exit status. Preserve protocol and actual process observations separately. Close owned foreground descendants through the consumer's mechanism; sampling direct children does not prove cleanup of detached workers.

A deadline may require terminating an owned handle, but retain forced/unknown cleanup instead of reporting normal success. Never kill a later process using a PID from an old manifest. Remove only private runtime resources, preserving the run. For retained reading use the [observable feedback method](../../scenarios/observable-feedback.md); choosing history never launches a command, sends input or restores a terminal.
