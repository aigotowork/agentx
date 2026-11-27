//! Shipped references for the CLI, configuration and workflow authors.

use super::{
    config::{config_help, terminal_help_text},
    help,
};

pub(crate) fn run(args: &[&str]) -> i32 {
    let unknown: Vec<_> = args.iter().copied().filter(|arg| !matches!(*arg, "--cli" | "--config" | "--scripts")).collect();
    if !unknown.is_empty() {
        eprintln!("Unknown docs option: {}", unknown.join(", "));
        eprintln!("Run 'agentx docs --help' for available sections.");
        return 1;
    }

    print_docs(args.contains(&"--cli"), args.contains(&"--config"), args.contains(&"--scripts"))
}

const SCRIPT_GUIDE: &str = r#"# Creating Custom Scripts

## Location

  User scripts:    ~/.agentx/scripts/
  File types:      *.sh (bash), *.py (Python 3)

*.sh scripts (including the bundled confess/debate/fatcow workflows) run via
`bash`. On Windows, install Git Bash (already a common dependency for
Windows dev setups, including npm-installed AI CLIs) so `bash` is on PATH.
Python scripts use `PYTHON` when set; otherwise Windows tries `python`, then
the Python launcher (`py -3`).

User scripts shadow bundled scripts with the same name.
Scripts are discovered automatically — drop a file and run `agentx run <name>`.
Add a description comment on line 2 (after shebang) — it shows in `agentx run` listings.

## Shell Script Template

  #!/usr/bin/env bash
  # Brief description shown in agentx run list.
  set -euo pipefail

  name_flag=""
  target=""
  while [[ $# -gt 0 ]]; do
    case "$1" in
      -h|--help) echo "Usage: agentx run myscript [OPTIONS]"; exit 0 ;;
      --iam|--target)
        [[ $# -ge 2 && -n "$2" ]] || { echo "$1 requires a value" >&2; exit 1; }
        if [[ "$1" == --iam ]]; then name_flag="$2"; else target="$2"; fi
        shift 2 ;;
      *) echo "Unknown argument: $1" >&2; exit 1 ;;
    esac
  done

  name_arg=()
  [[ -n "$name_flag" ]] && name_arg=(--iam "$name_flag")

  [[ -n "$target" ]] || { echo "--target is required" >&2; exit 1; }

  # Your logic here
  agentx send "@${target}" ${name_arg[@]+"${name_arg[@]}"} --intent request -- "Do the task"

## Identity Handling

agentx passes --iam to scripts automatically. Always parse and forward it:

  name_arg=()
  [[ -n "$name_flag" ]] && name_arg=(--iam "$name_flag")
  agentx send @target ${name_arg[@]+"${name_arg[@]}"} -- "message"
  agentx list self --json ${name_arg[@]+"${name_arg[@]}"}

The conditional array expansion also works with macOS Bash 3.2 under `set -u`.

## Launching & Cleaning Up Agents

With `--json`, a launch prints one JSON object whose "names" array lists the
spawned agents (also `launched`, `failed`, `batch_id`, `errors`). Parse it to
track them; agent names are plain words, so splitting on quotes is safe:

  LAUNCHED_NAMES=()
  track_launch() {
    local output="$1"
    local names
    names=$(grep -o '"names":\[[^]]*\]' <<<"$output" | grep -o '"[^"]*"' | tail -n +2 | tr -d '"') || true
    for n in $names; do LAUNCHED_NAMES+=("$n"); done
  }
  cleanup() {
    for name in ${LAUNCHED_NAMES[@]+"${LAUNCHED_NAMES[@]}"}; do
      agentx kill "$name" --go ${name_arg[@]+"${name_arg[@]}"} || true
    done
  }
  trap cleanup EXIT
  trap 'exit 130' INT
  trap 'exit 143' TERM

  launch_out=$(agentx launch --tool claude --go --json --headless \
    --agentx-prompt "Wait for a task via agentx; end your turn to receive messages." \
    ${name_arg[@]+"${name_arg[@]}"} 2>&1) || { echo "$launch_out" >&2; exit 1; }
  track_launch "$launch_out"

An `EXIT` trap also runs after a successful script. If you intentionally want
spawned agents to remain running, clear the traps with `trap - EXIT INT TERM`
only after successful setup. Otherwise, wait for the workflow to finish before
exiting, so cleanup does not kill workers before they can do their task.

## Waiting for Workflow Events

For event-driven workflows, `agentx events --wait` can block until the next
milestone instead of polling:

  worker="${LAUNCHED_NAMES[0]}"
  thread="myscript-$(date +%s)-$$"
  agentx send "@${worker}" --thread "$thread" --intent request \
    ${name_arg[@]+"${name_arg[@]}"} -- "Do the task, then send me DONE"
  agentx events --wait 120 \
    --sql "type='message' AND msg_from='${worker}' AND msg_thread='${thread}' AND msg_text LIKE '%DONE%'" \
    ${name_arg[@]+"${name_arg[@]}"}

Filter by the worker sender as well as the thread and completion marker; the
request itself contains `DONE` and must not count as a worker response.

## Workflow Building Blocks

- Give concurrent workflow runs distinct `--thread` values when their messages
  should stay separate.
- `--intent request|inform|ack` tells recipients whether a reply is expected.
- Scripted launches commonly use `--go` so they do not pause at the preview.
- agentx injects `--iam` when it knows the caller; parse and forward it when your
  script issues agentx commands on that caller's behalf.
- Launched agent names are generated dynamically. Capture them from the
  `--json` launch output when you need to address or clean them up.
- `agentx events --wait` and `agentx events sub` are useful alternatives to sleeps
  when the next step depends on an agentx event.
- `agentx events --follow` streams matching events as JSON Lines for a long-lived
  watcher; save the last `id` and pass `--after-id` to resume after a restart.

## Generic Workflow Pattern Ideas

- Worker + reviewer loop: a worker sends `ROUND N DONE`; a reviewer replies with
  `FIX: ...` or `APPROVED`. The worker can revise until the reviewer accepts it.
- Cascade pipeline: one stage finishes, the next reads its result or transcript
  (`agentx transcript @name --full`) and continues from there.
- Ensemble + judge: launch several agents on the same question in parallel, then
  have a judge read their thread messages and synthesize a verdict.
- Cross-tool pair: mix tools for different roles, for example one agent implements
  while another tool reviews its transcript or checks the result.
- Reactive workflow: subscribe to an event such as an agent becoming idle or a
  file changing, then trigger the next action with `agentx events sub`.

## Reference Examples

View source of any bundled or user script:

  agentx run <name> --source

See `agentx docs --cli` for full CLI command reference.
"#;

fn print_docs(show_cli: bool, show_config: bool, show_scripts: bool) -> i32 {
    let show_all = !show_cli && !show_config && !show_scripts;

    if show_all {
        println!("# agentx Documentation\n");
        println!("Sections:");
        println!("  1. CLI Reference");
        println!("  2. Config Settings");
        println!("  3. Script Creation Guide\n");
        println!("---\n");
    }

    if show_all || show_cli {
        println!("{}\n", help::get_help_text());
        for name in help::COMMAND_NAMES {
            println!("\n## {name}\n");
            println!("{}", help::get_command_help(name));
        }
        if show_all {
            println!("\n---\n");
        }
    }

    if show_all || show_config {
        println!("# Config Settings\n");
        println!("File: {}", crate::paths::agentx_dir().join("config.toml").display());
        println!("Precedence: defaults < config.toml < env vars\n");
        println!("Commands:");
        println!("  agentx config                 Show all values");
        println!("  agentx config <key> <val>     Set value");
        println!("  agentx config <key> --info    Detailed help for a setting");
        println!("  agentx config --edit          Open in $EDITOR\n");
        for field in crate::config::user_fields() {
            if field.name == "terminal" {
                println!("{}\n", terminal_help_text(false));
            } else if let Some(help_text) = config_help(field.env) {
                println!("{help_text}\n");
            } else {
                println!("{} - {} ({})\n", field.env, field.desc, field.kind.as_str());
            }
        }
        println!("Per-instance config: agentx config -i <name> <key> [value]");
        println!("  Keys: timeout, hints, subagent_timeout");
        if show_all {
            println!("\n---\n");
        }
    }

    if show_all || show_scripts {
        print!("{SCRIPT_GUIDE}");

        if !crate::workflows::SCRIPTS.is_empty() {
            println!("Bundled workflow names:");
            for (name, _) in crate::workflows::SCRIPTS {
                println!("  agentx run {name} --source");
            }
            println!();
        }
    }

    0
}
