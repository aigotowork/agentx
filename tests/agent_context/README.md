# Independent context acceptance

Authority: [Delivery](../../.controller/agent-context/delivery.md) and [proposal](../../.docs/proposals/agent-context.md). Result and remaining gaps: [acceptance](../../.controller/agent-context/acceptance.md). These files belong to the independent verifier. The user authorized the isolated composer crate with an external Cargo target and source-only plugin tests before release completion. Main-project Cargo, candidate CLI and native tests still require explicit final handoff of `/tmp/agentx-context-candidate`; file existence alone is not a handoff.

## Cases and evidence boundary

| ID | Starting condition and action | Required outcome | Evidence |
| --- | --- | --- | --- |
| C01 | Absent selector, empty string, whitespace, explicit `[]` | Empty selector selects builtin; `[]` invokes no generator | Independent composition tests; candidate launch |
| C02 | File list overridden by env empty/JSON and config-key JSON | Env empty restores builtin; JSON list replaces, never appends | Candidate launch/config |
| C03 | Text, empty text, file, builtin in order | Literal content, stable order, no shell/template evaluation | Independent composition tests |
| C04 | Edit relative file between startup/resume/repeated compact | Current cwd file read each time; current reason/participant/tool passed | Independent composition tests; hook fixture |
| C05 | Missing file, invalid UTF-8, unknown builtin, duplicate IDs, zero/two selectors | Diagnostic, no partial output or fallback | Independent composition tests; candidate launch |
| C06 | 65536-byte file/text and aggregate with separators | Exact byte boundary accepted; oversized rejected, multibyte correctly counted | Independent composition tests |
| C07 | Builtin retained/replaced/disabled with notes and native developer instructions; home config, default/argv native profiles, explicit normal/attached overrides | Notes exactly once only with builtin; home/profile instructions retained; explicit developer argv wins over home/profile without duplicates | Candidate argv capture; native request evidence separately |
| H01 | Active bound Codex/Claude, SessionStart compact, edited file, already announced | Fresh context; same active state, exact session, no extra user turn or mailbox acknowledgment | Isolated candidate hooks + before/after participant observations |
| H02 | Separately omit session ID, pass empty ID, pass wrong sibling ID, or pass retired ID; repeat with shared roots | No context output, foreign injection, rebinding, position/status mutation or delivery confirmation | [Explicit payload vectors](compact_sessions.json); isolated candidate hook/shared fixture |
| P01 | Pi/OMP session_compact followed by context twice | Fetch fresh projection per compact; hidden context without sendUserMessage/status/ack | Node plugin harness, not native model evidence |
| P02 | Pi/OMP RPC failure, empty context, late reply after session switch | No cached stale recovery; explicit error where applicable; no sibling leak | Node plugin harness |
| P03 | OpenCode native compaction callback and v2 context; successful nonempty refresh then RPC failure | Fresh session-qualified RPC; unrelated session receives no context; failure clears old cache for next ordinary request; later success recovers | Node plugin harness; native compatibility separate |
| N01 | Actual Codex manual compact in isolated home/cwd | Native compact event and next model request include changed sentinel and retained developer sentinel | Actual provider only; no claim from synthetic event |
| N02 | Actual automatic/mid-turn compact; shared thread A/B | Active turn continues; no synthetic user turn; correct thread only | Actual provider only; distinct from N01 |

## Runner structure

`Cargo.toml` is a separate test-only workspace importing the production composer via `#[path]`. It does not change the product manifest or compile all product binaries. Use a disposable external Cargo target directory. It tests composition contracts, not native provider delivery. The Node source harness requires `--source-authorized` and replaces RPC spawning and listeners with in-process fakes.

`launch_cases.py` uses the fixed candidate and the existing finite Codex recipient from `tests/environment_loading/codex_stub.py`. All configuration, provider homes, capture output and daemon scopes are temporary. It captures argv and rejects false launch successes; its observations cannot establish native compact consumption. It uses the existing exact-peer fixture cleanup, never process-name cleanup.

Native and hook execution procedures must record exact binary digest, scope, participant/session identity, before/after state, event JSON, model-visible evidence and cleanup. Keep failures and unsupported paths visible. Do not copy credentials or attach to user sessions. No full-provider acceptance is implied by authored cases.
