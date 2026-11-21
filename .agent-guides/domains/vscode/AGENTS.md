---
schema: document
description: Guidance for implementing, debugging and accepting VSCode extensions in their actual host.
---

# VSCode extension work

Use this domain for VSCode extension contributions, native interaction, extension-host integration, disposable Code acceptance and visual review in the consuming project. The owning application Task defines source writes, runtime/resources and acceptance scope. This guide does not grant installation, system settings changes or ownership of another Agent's Code window.

Start with the affected application's `AGENTS.md`, actual extension manifest/entry, selected public declarations and current Task.

Maintain the feature's goal, meaningful steps and expectations through [feature scenarios](../../scenarios/feature-scenarios.md). These connect the user's intended result to evidence that another reader can question after the Code window closes. Keep scenario meaning with the feature; use the domain's prepared mechanisms for capture and retention.

Native editor, Webview and Host observations explain different surfaces. A readable save message cannot establish the target document or persistence. For a UI scene, apply the shared [observable-feedback method](../../scenarios/observable-feedback.md) through this domain's [acceptance checkpoints](acceptance.md#plan-the-three-layers-at-checkpoints). Use the existing application runner for new captures and an installed saved-capture exporter for a self-contained bundle. Its declared public input and dependency contract owns executable use. A pure rule change still uses its focused contract check.

| Situation | Read/apply | Outcome |
| --- | --- | --- |
| Choose a UI surface, add a Webview/language/debugger feature, change host support, localize or package an extension | [Development scenarios and selected skills](scenarios.md); follow only matching routes | Domain-specific design, public contracts and an appropriate validation boundary |
| Implement or debug a command, menu, Comments, TreeView, draft or host session | [Native implementation boundaries](#native-implementation-boundaries), then only the affected public VSCode API declarations | Correct registration, target identity, state and lifetime |
| Plan or run a user-facing acceptance scene; review screenshots | [Acceptance and visual review](acceptance.md) | Correlated semantic layout, necessary state and rendered evidence at meaningful checkpoints, plus actual actions/effects |
| Bind a feature's steps to new Code captures or export an explicit saved run | [Capture and export](tools.md#export-a-retained-run) | Reused runner/driver and export mechanism; feature-owned actions, state meaning and expectations |
| Select an interaction/capture method, diagnose automation failure or consolidate repeated probes | [Tools and reusable automation](tools.md) | A bounded tool choice and repeatable scenario with explicit limits |
| Read retained scene evidence or revalidate feedback | [Retained checkpoints and feedback](acceptance.md#retain-checkpoints-and-follow-up-feedback) | Readable history with source links; explicit reconstruction/reexecution and new result identity |

## Native implementation boundaries

Treat the workbench renderer, extension host and any owned external process as separate runtimes. Record their actual versions when a behavior depends on them. A successful standalone Node build does not establish that the extension host can load the same package. Keep provider/process composition at its declared public boundary.

Before changing a native surface, inspect the actual VSCode types and manifest contributions. Check the command ID, argument shape, `when` visibility, command `enablement`, context-key producer and disposal together. A registered command is not necessarily visible or enabled in the surface that invokes it.

- Bind resource actions to the clicked URI. Test an Explorer folder or file B while source A stays open; the active editor is not the target of every command.
- Capture dirty selection and source identity before awaiting a picker, setup or process acquisition. Preserve that intent while focus moves to Comments, a sidebar, settings or a draft.
- Use one session owner for shared data/lifetime. Native views borrow its actions and state; they do not independently open providers or interpret business references.
- Fence asynchronous results by their original scope/generation and resource identity. A late result must not replace a new selection or write to its root.
- Preserve editable body on cancellation or known failure. Distinguish confirmed save from unknown effect; loss of a reply does not authorize replay.
- Make long editing and return explicit. Keep ordinary file Save semantics. Test focus, visible source and retained draft together.
- Dispose registrations and views, stop admission and drain accepted work. Close must not depend on an unanswered native picker.

Use native controls when they fit the interaction. A Webview adds separate rendering, focus, security and lifetime work; choose it for a concrete need. Apply the existing [interaction/state guidance](../webdevelopment/interaction-and-state.md) when designing these transitions, rather than creating another state model in test code.

Design the relevant scenario/state cases and write their tests first; ordinary implementation does not run them. Apply the [test and image limits](../../principles.md#quality-tests-and-images) to any execution or visual review. Use a real Code scene for behavior that types or injected commands cannot establish; [the acceptance guide](acceptance.md) defines that boundary. Do not rerun an unaffected historical campaign after a local fix.

For a standalone Skill entry use [observable-evidence](../../skills/observable-evidence/SKILL.md). Harness-kit component `observable-evidence` supplies adapter usage through its manifest documentation resource under `docs/tools/observable-evidence`; the consumer still supplies a real Code capture host. Export availability alone does not establish that host.
