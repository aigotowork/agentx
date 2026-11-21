# Passive observation CLI

`trace.rs` reads rotated JSONL independently of daemon availability; an explicit trace ID optionally adds one bounded passive lookup by existing invocation identity. See [telemetry coverage](../../../../.docs/observability/telemetry-coverage.md).

Read [CLI context](../../AGENTS.md), the [original diagnostics Task](../../../../.controller/observability/implementation-tasks/diagnostics.md) and [native observation context](../../../observability/AGENTS.md). The Task owns actual work/release state; integration owns all formatter/compiler/test/package/probe execution.

`../observe.rs` is the family entry. `args.rs` owns clap and explicit local files; `client.rs` owns native passive calls and local attempt lifetime; `output.rs` owns consumer framing/presentation; `tests.rs` contains authored cases. Native refs/results remain imported unchanged. Do not add Session/acquisition/storage, identity/dev-root/update behavior, automatic reports or business retries. `config-current` alone explicitly reevaluates the selected scoped settings through the actual resolver.

Preserve originating requested paths, native scope/incarnation/generation/clock boundaries, known failures with absent time, and exact whole event windows. Follow does not rebase gaps or changed eras. JSON numbers retain native u64 precision; output pipes/signals stop only observation. [Consumer boundary](../../../../.docs/observability/consumer-boundary.md#planned-cli-surface-and-output) and [usage](../../../../.docs/observability/cli.md) are the caller-facing contract.

[Authored process scenarios](cases.md) complement the owner-local tests and existing independent artifact families. They explicitly require real passive CLI calls during held business work and actual process interruption/closed-pipe isolation; neither test authorship nor semantic output fixtures establish those guarantees. No test or process is run by importing/reading this guide.
