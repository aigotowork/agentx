---
schema: scenario
description: Design command grammar, configuration, results, effects and recovery as one coherent CLI contract.
tags: [cli, design, documentation]
---

# Command-Line Tool Design

Use this guide when designing, implementing or materially changing a CLI, its commands, arguments, options, configuration, help or usage documentation. Start with the users, operating scenarios and required outcomes; design the interface as a whole before filling in individual commands.

## Routing

| Situation | Apply |
| --- | --- |
| Connecting an evolving CLI to product scenarios, domain collaboration and current/target implementation | [Design management](../domains/design-management/AGENTS.md), alongside the applicable command rules below; a help wording correction alone does not require a new model |
| Establishing or changing the command surface | [Global interface model](#global-interface-model), then [Command grammar and usability](#command-grammar-and-usability) |
| Defining inputs, defaults or configuration sources | [Defaults and configuration](#defaults-and-configuration) |
| Defining command results, output streams, exit status, effects or interruption | [Results, effects and recovery](#results-effects-and-recovery) |
| Choosing existing objects through interactive commands, prompts or selectors | [TUI selection design](../domains/tui/workspace-design.md#prefer-recognition-and-selection-to-transcription): searchable choices and useful defaults, with explicit inputs and noninteractive behavior for automation |
| Selecting or implementing parsing and dispatch | [Framework selection](#framework-selection), using the accepted interface model |
| Writing help or tool instructions | [Concise help](#concise-help) and [Usage guide](#usage-guide) |
| Completing an implementation or interface change | [Verification](#verification) for the affected behavior |

## Global interface model

Maintain one authoritative interface model in the owning design or command declarations. It must provide a global view of all command paths, global options, command-specific options and positional arguments, including defaults and configuration mappings. Separate help pages or scattered parser registrations do not provide this view on their own.

Use a command tree and an input matrix as complementary views of that model. Review the complete surface before implementation; for an existing CLI, update the affected entries while checking consistency with the whole. Derive views from canonical declarations where practical rather than maintaining competing specifications.

| View | Required information |
| --- | --- |
| Scenario-to-command map | User intent, canonical command path, required inputs and observable result; expose missing capabilities and duplicate ways to perform the same task |
| Command tree | Root behavior, every subcommand and its purpose, positional arguments, local options and inherited global options |
| Input matrix | Owning scope, argument or option spelling, type and unit, allowed values, default or default derivation, ENV name, config key or explicit non-support, constraints and source precedence |
| Command behavior | Selected domain operation, admission conditions, result meaning, output/exit behavior, effects and supported continuation; use [results and recovery](#results-effects-and-recovery) |

For example, the following is an illustrative interface snapshot, not a required command hierarchy:

```text
tool
  global: --config FILE, --format text|json, --help
  inspect [PATH] [--depth N]       Summarize a directory
  export [PATH] [--output FILE]   Export its inventory
```

| Scope | CLI input | Type / allowed values | Default | ENV | Config key |
| --- | --- | --- | --- | --- | --- |
| Global | `--config FILE` | File path | `./tool.yaml` if present; otherwise no file | `TOOL_CONFIG` | Not read from the file it selects |
| Global | `--format FORMAT` | `text`, `json` | `text` | `TOOL_FORMAT` | `format` |
| `inspect`, `export` | `[PATH]` | Directory path | Current working directory | `TOOL_PATH` | `path` |
| `inspect` | `--depth N` | Integer, levels, `N >= 0` | `1` | `TOOL_DEPTH` | `inspect.depth` |
| `export` | `--output FILE` | File path or `-` for stdout | `-` | `TOOL_OUTPUT` | `export.output` |
| Every command | `--help` | Invocation control | `false` | Not configurable | Not configurable |

Record cross-input constraints beside the matrix. State where global options are accepted, whether child commands inherit them and what an invocation without a subcommand does. Prefer displaying short help when no obvious safe primary action exists. A single-purpose tool may have no subcommands; it still needs a complete interface view.

Keep exact syntax in the existing command declarations or authoritative reference. Explain behavior beside those declarations or in the owning CLI design, linking domain rules and native operation types. Help, usage and generated views consume that contract. A new command does not require a separate model file or a second command registry. Mark planned commands and unsupported modes separately from the current interface.

## Command grammar and usability

Optimize functional coverage and ease of use together. The scenario map establishes which capabilities are needed; it does not require a flag for every internal setting or a subcommand for every function.

- Choose one understandable grammar, such as `tool <action> <target>` or `tool <resource> <action>`, and use it consistently. Keep nesting shallow; introduce a group only when it expresses a meaningful distinction.
- Use subcommands for distinct user intentions, positional arguments for obvious primary operands and options for variations of the same operation. If flags select unrelated operations or create many invalid combinations, reconsider the command boundary.
- Prefer familiar domain words, explicit long option names and value placeholders that reveal meaning and units, such as `--timeout SECONDS`. Reuse a name only when its meaning and type remain consistent. Reserve short aliases for frequent, unambiguous actions.
- Keep routine commands short through useful defaults. Add an option only when a real scenario needs that choice; consolidate synonyms and avoid speculative configurability.
- Make scope, conflicts and required combinations evident. Avoid overloaded arguments, order-dependent surprises and mode switches whose meaning depends on unrelated flags.

Aim for users to predict the common command and its arguments without opening help. Walk through representative scenarios using only names and syntax; repeated explanation is evidence to simplify the interface. This goal complements, and never replaces, help and usage documentation.

## Defaults and configuration

Every argument, option and configuration value must declare an explicit default, including its type, units and any deterministic derivation. Prefer useful defaults that make ordinary invocations work. When an essential operand or credential has no valid inferred value, declare an explicit unset default such as `null`. If the command offers a documented interactive choice and a TTY is available, resolve an omitted operand through that choice before execution; otherwise fail with a concise missing-input message. Never invent a target or credential, or silently select the first candidate. Distinguish an unset value from valid empty strings, zero and `false`.

Every configurable behavior must be settable through an explicit CLI option and a namespaced ENV variable. Add a config-file key for settings that need persistence; an individual setting may intentionally omit config-file support, but record that choice in the matrix. Positional operands remain explicit CLI inputs and need documented defaults and ENV mappings when configurable. Command selection and immediate controls such as `--help` are invocation syntax, not persisted settings.

Use one documented precedence order across the tool, from lowest to highest:

```text
built-in default < config file < ENV < explicitly supplied CLI option
```

Explicit CLI input always wins; an explicit positional operand belongs to the same highest layer. If an operand also has an option form, define a single unambiguous rule, preferably rejecting simultaneous use. If a setting does not read config files, its chain is `default < ENV < CLI`. An established project may order config and ENV differently only when that contract is explicit and consistent; CLI must remain the final override.

- Resolve values by presence, not truthiness. A supplied `false`, `0` or permitted empty value must override a lower layer. Keep parser defaults distinguishable from explicitly supplied options so they cannot accidentally overwrite ENV or config.
- Use stable, tool-prefixed ENV names and predictable config keys. Define parsing for booleans, lists, enums, paths and units; use the same validation across sources. State whether lists replace or merge, defaulting to replacement unless a scenario requires merging.
- Allow the CLI to reverse lower-layer booleans, for example with `--cache` / `--no-cache` or an explicit boolean value. Do not expose only an enabling flag when config or ENV can enable that behavior.
- Define config-file discovery, selection and path resolution. A selector such as `--config` must itself have a default and ENV override; resolve it before loading the selected file. An absent optional default file may mean empty config, while an explicitly selected missing or invalid file must report an error.
- Validate the resolved inputs and their relationships before executing the command. Report the offending input and an actionable correction; do not silently replace an invalid explicit value with its default.

## Results, effects and recovery

For each command with substantive behavior, connect the user's intent to the domain operation it uses. Specify the following where they affect callers; shared rules can be stated once for the tool, with command-specific differences beside the command.

| Caller question | Declare |
| --- | --- |
| What is selected and when is it allowed? | Target identity/scope and revision, resolution time for selectors such as `latest`, permissions and state preconditions; keep admission with its actual domain owner |
| What does success mean? | Whether the result means accepted, started, observed or completed; what exit `0` establishes for this command and what it leaves unresolved |
| What can consume the output? | stdout data, stderr diagnostics/progress, file destinations and overwrite/partial-file behavior; human versus machine formats, stable fields, encoding and record framing for streams |
| What happens without a terminal? | stdin operand/data rules, TTY versus piped behavior, prompt/color/progress policy and behavior on EOF or a closed output pipe; an automated invocation must not unexpectedly wait for a prompt |
| What changes? | State or external effects, facts left unchanged, owned versus borrowed resources, and whether waiting or process lifetime controls the underlying work |
| How are other outcomes reported? | Usage/configuration errors, domain rejection, technical failure, partial completion and unknown outcome where applicable; map them to documented exit codes and output without erasing needed distinctions |
| How can the caller continue? | Relevant timeout/signal/disconnection behavior, retained operation identity or receipts, cleanup and supported query/retry/cancellation; distinguish stopping observation from cancelling work |

Keep diagnostics out of machine-readable stdout. Choose exit codes that fit the tool's ecosystem and existing contract; this method prescribes no universal numeric taxonomy. A nonzero exit after dispatch does not prove that nothing happened, and exit `0` for an accepted background job does not prove completion. Return enough identity and outcome information for the documented next action. Apply the domain's [effect and recovery rules](data-state-modeling.md#effect-boundaries-and-unknown-outcomes); a CLI adapter cannot invent safe replay or stronger cancellation.

For an illustrative background report tool, `tool run --wait` may start work and observe it, while `tool result RUN_ID` only reads that original run. Define whether interrupting the wait leaves work running, how the run identity remains available, and how callers distinguish a completed run from an unavailable observation. This is a design example, not a required command hierarchy or a claim that any product supports recovery.

Compare existing scripts' observable promises when changing defaults, selectors, result formats, codes or effects, using [changed-promise analysis](collaborative-modeling.md#compare-changed-promises). Unchanged option spelling does not establish compatibility: resolving a relative path against a different directory can change the target.

## Framework selection

Follow [technology selection design](./engineering-design.md#technology-selection-design) for reuse decisions. Use a language-provided argument parser or standard package for a simple CLI. Prefer a mature, commonly used framework in the language's ecosystem when subcommand hierarchies, shared options, validation, configuration or consistent help make a larger framework useful.

Select against the global interface model, not familiarity alone. Check support for the required grammar, explicit-value tracking, help, errors and configuration integration. Keep domain operations separate from parsing and dispatch. Reuse the framework's mechanisms and add only the missing integration; do not hand-roll a parser, command registry or help engine when an established solution fits.

## Concise help

Provide working `--help` at the root and every command level. Help must work without credentials, a valid config file, network access or command side effects.

Root help explains the tool's purpose and common scenarios in a short line, then shows usage, commands, global options and a few common examples. Command help shows its purpose, invocation syntax, arguments, local and inherited options, meaningful defaults, value choices and a few examples. Keep descriptions short and scannable; route detailed configuration and workflows to the usage guide.

Use a compact shape such as:

```text
tool inspect — Summarize a directory before exporting its inventory.
Usage: tool inspect [PATH] [--depth N] [global options]
PATH          Directory to inspect; default: current directory.
--depth N     Levels to inspect, N >= 0; default: 1.
Examples:
  tool inspect
  tool inspect ./data --depth 2
```

This excerpt illustrates concise wording, not a complete help implementation; actual output must also make applicable global options discoverable. Keep design rationale and long tutorials out of help.

## Usage guide

Deliver a dedicated, discoverable tool usage document in addition to help. Write it like a Skill for a caller: route from the user's situation to the exact command and inputs. A Markdown guide is sufficient; this requirement does not require packaging a new Skill. Link it from the tool's README or normal entry point and from help where practical.

Use this reusable template, replacing every placeholder with the tool's real interface:

```markdown
# <Tool> usage

<One sentence explaining when to use the tool.>

## Choose a command

| Situation / desired result | Command template | Inputs to replace | Result / effects |
| --- | --- | --- | --- |
| <A concrete user scenario> | `<tool> <command> <TARGET> --<option> <VALUE>` | <Meaning, type and accepted values of each placeholder> | <Output destination and relevant side effects> |

## Common workflows

<Only prerequisites that affect use, such as installation or authentication.>
<A minimal working example for each common scenario; show steps only when order matters.>

## Configuration

<Default behavior, config-file location, precedence and a compact CLI / ENV / config mapping.>
<Show an ordinary invocation, a persistent or ENV setting, and a one-off CLI override.>

## Results and recovery

<How to interpret success and output; common actionable errors and exit behavior.>
```

Templates must be complete enough for a human or agent to choose and invoke a command without guessing subcommands, required arguments, option values or ordering. For the illustrative interface, useful rows include “inspect the current directory → `tool inspect`”, “inspect two levels → `tool inspect <DIRECTORY> --depth 2`” and “export JSON to a file → `tool --format json export <DIRECTORY> --output <FILE>`”, with placeholders and output behavior explained.

Exclude architecture, framework choices, internal modules and design rationale from this usage document; those belong in the owning design. Keep help and usage examples aligned with the canonical interface, linking detailed reference material rather than maintaining a second specification.

## Verification

Use the smallest checks that establish the affected contract:

- Trace each supported scenario to a canonical command and confirm the global model covers all commands, inputs, defaults and source mappings.
- Exercise representative precedence collisions: default only, config, ENV over config and CLI over both. Cover explicit `false` or zero, boolean reversal, omitted options and invalid values where applicable.
- Check root and affected command help without configuration or credentials; confirm it explains syntax and examples without performing work.
- Run affected usage templates with suitable sample inputs and check their outputs and side effects. A caller should be able to select a template from the scenario table alone.
- Check the affected result branches, stdout/stderr and exit status together, including machine/piped use when supported. For stateful commands, exercise the decisive interruption or failure and its documented continuation; output alone does not prove the underlying effect.

For documentation-only changes, verify routes, links, example consistency and requirements coverage; do not invent an implementation or a heavyweight review process.
