# Environment loading acceptance fixtures

These fixtures exercise the accepted [environment-loading Proposal](../../.docs/proposals/environment-loading.md) through a disposable project, scope, and local child processes. They are outcome inputs, not a second source of product semantics. Native CLI spelling and diagnostic field names come from the released native contract before the runner is executed.

## Fixture topology

`prepare.py` creates one run root with two independent projects/scopes, an outside directory, ordered dotenv files, scope-owned TOML declarations, an envrc marker, a separate legacy scope environment file, and an invalid source. It emits a manifest containing paths and non-secret expected labels. No generated file contains a credential or a value copied from the caller's environment. Each ordinary scope also exposes `run envprobe` as a disposable custom Python workflow, reusing the finite recipient so basic originating-value checks need no daemon or Provider preflight.

`codex_stub.py` is a finite native-recipient substitute copied to the fixture's own `bin/codex`; it supports only the existing hooks-list preflight, native version probe, a finite child exit, and optional exact stdio echo. Its capture is limited to the fixture's explicit variable allowlist, cwd, argv and TTY status. Foreground, stdio, background and contaminated-parent shell reconstruction must go through the released product launcher; a standalone shell script would not establish those product boundaries.

`envrc` is data for a dedicated project at the existing authorized direnv seam. Its marker makes accidental shell execution observable without affecting the other projects. An explicitly selected `.envrc` must not execute as dotenv; the authorized stdio path may execute it only through the existing direnv contract and only when its project scope requests that path. The legacy scope is separate so its last overlay cannot conceal the ordered-file checks.

## Scenario contract

| ID | Starting state and action | Required observation | Independent effect/invariant |
| --- | --- | --- | --- |
| ENV-01 | Set `AGENTX_ENV_FILES` to base, project, local | Ordered files load and later files win | The recorded provenance lists all three paths in order |
| ENV-02 | Set repeated CLI files and a lower-priority ENV/config list | Explicit CLI replaces lower-level declarations | No file from the replaced list is read |
| ENV-03 | Use `[environment].files` relative to each project | Relative paths resolve against the selected local project | Two projects resolve to their own files and do not cross-contaminate |
| ENV-04 | Inherit `CALLER_WINS`, then enable and reverse override | Default preserves caller values; override lets files win; explicit false restores caller wins | Empty values remain present values, not deletions |
| ENV-05 | Select an external file that supplies `AGENTX_DIR` | External file can bootstrap the selected scope before acquisition | The created scope is the selected disposable scope only |
| ENV-06 | Select a scoped config file that tries to redirect `AGENTX_DIR` | Scoped declaration cannot redirect its own scope | No alternate managed state or child is created |
| ENV-07 | Put selector declarations inside a loaded file | Selectors are frozen once and are not recursively loaded | The nested source marker is untouched |
| ENV-08 | Select missing, unreadable, and invalid files containing marker values | Failure identifies source/path/line without values | No managed state, daemon owner, Provider, or child exists afterward |
| ENV-09 | Run help and version with poisoned or absent declarations | Both remain passive and successful | No scope, child, direnv, or Provider side effect occurs |
| ENV-10 | Launch the finite child foreground, stdio, background, and nested | Effective environment reaches each local child | Cwd, argv, exit status, and explicit values survive composition |
| ENV-11 | Use an authorized disposable `.envrc` and legacy scope `env` | Existing stdio direnv and legacy precedence remain as documented | `.envrc` is not imported twice; ordinary no-new-setting behavior remains |
| ENV-12 | Start two originating scopes/contexts concurrently | Each retains its own loaded environment | No daemon-global environment mutation or cross-scope value appears |
| INS-01 | Request local configuration inspection after loading | Sources and variable provenance are visible with values withheld by default | Secret-like values never appear in stdout/stderr/evidence |

The first execution should cover these cases with one fixed source/artifact release. Unsupported Provider/OS/window combinations remain explicit limits; this fixture proves the Linux local boundaries it actually observes.

## Native contract receiving decisions

The [native environment contract](../../.docs/architecture/environment.md) selects repeated `--env-file PATH` or `--env-file=PATH`; reversible `--env-override` / `--no-env-override` with the last CLI control winning; `AGENTX_ENV_FILES` as a comma list; and `config environment [--json]` as a values-withheld local inspection. An explicit empty source selector disables lower-priority sources. Dotenvy parses dotenv data and substitutions; a literal shell expression must be quoted and must never execute.

Selectors and the override boolean are frozen independently at the pre-dotenv control scope: explicit `--scope`, otherwise inherited `AGENTX_DIR` or home scope, after authorized stdio direnv. CLI > ENV > that control-scope TOML applies independently to list and boolean. This resolves bootstrap circularity: a control-scope TOML override can govern external files, an inherited `AGENTX_DIR` wins by default, and an external file can choose a different final scope only when the inherited key is absent or override is enabled. The resulting scope's selectors are never reevaluated; the diagnostic distinguishes selector scope from final scope. A TOML-selected dotenv declaration of `AGENTX_DIR` is a failure before acquisition, not a silently accepted redirect. Explicit command scope wins finally.

Authorized stdio direnv is caller input before dotenv. Its values win with default override and lose when override is enabled. Legacy `<scope>/env` remains a separate last, non-empty, child-only overlay and ignores all `AGENTX_*` declarations. Its fixture uses ordinary `ENV_FIXTURE_LEGACY` to prove the final overlay, `ENV_FIXTURE_EMPTY` to prove empty legacy values are ignored, and an ignored `AGENTX_ENV_CASE` declaration to retain the selected dotenv assertion. These seam outcomes must be checked separately so the legacy overlay cannot mask originating-value assertions.

## Execution and retention

The acceptance runner must freeze the released source manifest before compiling, record the current artifact and protected runtime metadata before any package step, and use one sole executable owner. Each attempt gets a new run root. On failure, retain the attempt and diagnose the fixture or product cause before retrying. Cleanup is bounded and records process identity; it must not discover or operate the protected project resources.

`freeze_harness.py` retains the exact consumer scripts and syntax basis for each continuation without changing immutable product inputs. `outcome.py --phase metadata` isolates the passive13-shape correction check; `--phase workbench` starts one disposable embedded host using a selected project's relative file, verifies its bootstrapped scope/project descriptor after removing that source, then stops its exact host/daemon and checks the owned port is closed. This is HTTP startup evidence only, with no browser/UI replay.

`final_audit.py` binds the final artifact to frozen inputs, checks every retained command/child/preflight/parent/daemon incarnation, observes only acceptance-owned product images and named protected process metadata, and removes the three owned preparation/check directories after all resource invariants pass. It does not promote the artifact or operate user services.

`--phase baseline` uses no environment-file/override CLI flags, no selector/override environment variables and an empty TOML file. It proves ordinary inherited values and an inherited empty value through the real stdio recipient. This is distinct from launch fixtures that select an instrumentation file for capture/PATH setup.
