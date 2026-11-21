---
schema: proposal
description: Customize the fixed AgentX bootstrap through one mutually exclusive text, file or bounded Bash hook selector.
status: withdrawn
models: [../agent-context.md, ../../src/runtime/agent_context.rs]
---

# Custom bootstrap generation

Withdrawn on 2026-11-17. The user selected existing `AGENTX_INSTRUCTION_SOURCES` for static replacement and requested a [source reference](../prompt-system/README.md), without implementing new behavior. The three selectors, Bash hook, JSON module protocol and five-second deadline below are historical design only and are not available configuration. `--agentx-system-prompt` retains its additive behavior. The existing [context proposal](agent-context.md) and [instruction lifecycle](../agent-context.md) remain the composition/lifetime owners.

## Scenario and boundary

Operators need to replace AgentX's default collaboration guide without modifying or rebuilding its binary. Static text, a maintained file and a context-dependent generator are three sources for the same replacement. Routine child-Agent delegation remains the single default launch command; these selectors are operator configuration, not new bootstrap instructions for Agents to learn.

The discussion considered replacing the builtin `agentx` fixed-instruction contribution while retaining adopted Role instructions and explicit launch instruction additions. The user subsequently clarified this boundary and chose existing instruction sources rather than a new selector. Native Provider instructions, user task input and dynamic runtime snapshots retain their owners. Replacement prompt text cannot change actual native identity or membership. The remaining sections preserve the abandoned design for historical reference.

## Existing capability and naming clarification

Source inspection finds no general `AGENTX_SYSTEM_PROMPT` environment setting. The existing CLI `--agentx-system-prompt` supplies explicit additional instructions; fresh launch retains these as `instruction_additions` and composes them after the builtin and adopted Role prompt. Tool-specific `AGENTX_CODEX_SYSTEM_PROMPT` and `AGENTX_GEMINI_SYSTEM_PROMPT` settings are separate Provider options, not a common bootstrap interception API.

The existing `AGENTX_INSTRUCTION_SOURCES` already replaces the fixed-source list using ordered `{id, text}` or `{id, file}` sources. Omitting the builtin from that list supplies a static replacement while adopted Role/additional instructions retain their current composition. Thus static text/file customization needs no new runtime mechanism; the new names, if retained, should be convenience selectors over that same composition rather than a second implementation. External execution, structured default-module input and the five-second subprocess deadline are the actual missing capabilities. The public hook output is an ordered rendered module list; existing file sources are source declarations, not an instruction to execute files returned by a hook.

## Selector contract

| Effective environment selection | Proposed behavior |
| --- | --- |
| None | Use the existing builtin guide |
| `AGENTX_BOOTSTRAP_PROMPT` | Use the literal UTF-8 value |
| `AGENTX_BOOTSTRAP_PROMPT_FILE` | Read one UTF-8 file relative to the effective Agent working directory, or an absolute path |
| `AGENTX_BOOTSTRAP_PROMPT_HOOK` | Interpret the value as a Bash script path, resolve it against that same directory and invoke `bash -- SCRIPT_PATH` |
| More than one selector is present | Reject with a diagnostic naming the conflicting selectors; do not choose one by priority |

Count presence, not truthiness. A present empty inline value deliberately disables the builtin guide; empty file/hook paths are errors. Literal and file text are not template-expanded or executed. Recommended handling when explicit `instructions.sources` omits the builtin: reject a supplied custom-bootstrap selector as unused/conflicting configuration rather than silently ignoring it. The current caller-owned environment loading resolves effective values before selection; a shared daemon does not execute scripts or inherit per-launch configuration into its process environment. CLI/config-file equivalents, if added, should join this same resolved selector rather than create another precedence mechanism.

## Hook protocol v1

One JSON object on stdin followed by EOF. Keep resolved launch facts in `context` and the ordered rendered bootstrap modules in `data`. Use an explicit version and a bounded set of facts, rather than serializing internal records or the complete environment. Illustrative input:

```json
{
  "version": 1,
  "context": {
    "agent": {"name": "koko", "tool": "codex", "role": "coder", "title": "Implement xxx"},
    "launch": {"cwd": "/projects/app", "workspace": "/projects/app", "launched_by": "mira", "headless": true, "bindings": ["/projects/app/.controller/xx/xx.task.md"]},
    "session_id": null
  },
  "data": [
    {"id": "identity", "text": "Rendered AgentX identity instructions"},
    {"id": "messages", "text": "Rendered messaging instructions"},
    {"id": "capabilities", "text": "Rendered command guidance"}
  ]
}
```

This is a proposed public shape, not a claim that the current bootstrap snapshot exposes all these facts or modules separately. Implementation must trace every field to its owning resolved launch projection. Unknown or unavailable optional facts are `null`, not guessed; native session IDs commonly do not exist before Provider startup. Input `data` contains rendered builtin modules after existing exclusions/notes, before Role and other fixed sources are appended. Stable module IDs let scripts retain, remove, replace or reorder specific content without parsing the full prompt or depending on array positions. The examples illustrate the item shape; the complete default module inventory must be defined against the real renderer during implementation. Keep full environment values, credentials, native argv and conversation history out of this public input by default.

On successful exit, stdout contains exactly one JSON object with the same required `data` array shape:

```json
{
  "data": [
    {"id": "identity", "text": "Retained or modified identity instructions"},
    {"id": "project", "text": "Project-specific collaboration guidance"}
  ]
}
```

The returned array is the complete replacement module list, not a patch. Preserve array order and join nonempty `text` values with two newlines. Omitted input modules are removed; new unique IDs are allowed. IDs are nonempty, unique, case-sensitive strings; `text` must be a string. `data: []` deliberately supplies no builtin guide. Empty item text contributes no bytes or separators. IDs support selection and diagnostics, not hidden ordering, instruction priority or runtime authority. Do not introduce sorting, dependency declarations, recursive sections or patch operations for this use case.

This supersedes the earlier proposed scalar `default_prompt`/string `.data` format before implementation; no compatibility mode is needed. Inline and file selectors can retain their plain-text convenience and normalize internally to one `{id: "custom", text: ...}` block. They do not need another JSON format just to supply a static replacement. Internally, adapt public item IDs to the existing [ordered context block](../../src/runtime/agent_context.rs) source/text model and keep composition in its owner.

Logs belong on stderr. No output, invalid JSON, a missing/non-array `data`, invalid/duplicate item IDs, non-string text, invalid UTF-8, a nonzero exit or an oversized result fails bootstrap generation; do not silently substitute the default. Bound returned module count consistently with the existing finite-source policy. Reuse the current 64 KiB composed instruction limit, including join separators, and enforce finite transport-output bounds while reading; exact encoded stdout/stderr limits belong to the eventual native subprocess contract.

## Lifetime, deadline and failure

Generate in the originating host only when fixed instructions need their first successful value, then compose and freeze the result through the existing native instruction store. Same-Agent resume and compact reuse that saved value without rereading the file or rerunning the hook. A new Agent identity generates its own value. This is not a runtime-context refresh hook.

The five-second wall-clock deadline covers process creation, stdin transfer, stdout/stderr draining and process completion. Implementation must supervise process descendants and pipe lifetime so a hung script or child cannot extend the deadline indefinitely. Unavailable Bash is an explicit capability error. Avoid implicit retries; scripts should be side-effect-free prompt generators. The existing read/generate/freeze sequence alone does not guarantee one external invocation under concurrent startup callbacks, so concurrency admission must be designed before promising exactly-once execution.

## Decisions and implementation evidence still needed

The three mutually exclusive sources, JSON stdin, matching ordered `.data` module arrays and maximum five-second execution follow the user's suggestions. Stable `id`/`text` items, complete replacement semantics, bootstrap-only scope, script-path interpretation, explicit empty content and frozen-lifetime behavior are recommendations pending the user's selection. Source inspection confirms existing text/file fixed sources, ordered source/text blocks, originating-host generation and retained same-Agent instructions; an external process hook and public module decomposition are new behavior. No executable support, hook timing, Provider integration or model-following is claimed.

If selected for implementation, derive checks from the actual branches: all selector collisions including empty strings; literal/file normalization; relative paths; retained/removed/reordered/new modules; duplicate IDs and malformed items; empty array and empty text; invalid JSON/type/exit; blocked stdin and inherited descendant pipes under the deadline; output limits including separators; missing launch facts; Role/addition preservation; and same-Agent resume/compact reuse versus new-Agent generation. Record the fixed candidate and executed Provider scope separately from protocol design.
