---
schema: scenario
description: Route VSCode extension design, implementation and validation to public contracts and selectively adapted skill methods.
tags: [vscode, design, implementation, skills]
---

# Development scenarios and selected skills

Use all matching rows; read only the selected sections. The application's Task and declared supported environments bound the work. Current official documentation explains the platform; the extension's selected `engines.vscode`, type declarations and actual host determine which APIs are usable. A remote skill link is a method reference, not an installed tool or an instruction to run its setup.

For feature work, maintain the [usage scenario and each meaningful step's expectation](../../scenarios/feature-scenarios.md) in the feature's own context. This basis lets a later reader judge what the captured editor, Webview and Host facts mean. Use the installed adapter's public capture/export contract for supported mechanisms; consumer actions/state/effect checks keep their own owner.

| Situation | Route | Required result |
| --- | --- | --- |
| Choose native navigation, commands, menus, Comments or views | [Native interaction](#native-interaction) | A visible action with the correct target and context |
| Add or change a Webview or custom editor frontend | [Webview boundaries](#webview-boundaries) | Deliberate message, resource, state and rendering boundaries |
| Change local/remote/web support, workspace files or executable configuration | [Host and workspace support](#host-and-workspace-support) | Explicit supported runtime and resource authority |
| Add diagnostics, completion, navigation, rename or a language server | [Language features](#language-features) | A justified provider/LSP choice and current-document results |
| Integrate a debugger or change its protocol/session | [Debugger integration](#debugger-integration) | An adapter with real target/session evidence |
| Add user-visible strings, keyboard interactions, themes or accessibility behavior | [Localization and accessibility](#localization-and-accessibility) | Usable labels, focus and supported locale/theme behavior |
| Reproduce a UI bug, test activation, diagnose leaks or slow repeated work | [Debugging and validation](#debugging-and-validation) | Evidence from the affected runtime and a bounded regression check |
| Explain a scene's layout/state/rendering, retain observations or investigate historical feedback | [Checkpoint acceptance](acceptance.md#plan-the-three-layers-at-checkpoints), then [retention and feedback](acceptance.md#retain-checkpoints-and-follow-up-feedback) | Correlated observations with explicit gaps and a bounded revalidation path |
| Build a VSIX or prepare an authorized release | [Packaging and publication](#packaging-and-publication) | Reviewable package contents and preserved release identity |

## Native interaction

Select the surface using the [official UX map](https://code.visualstudio.com/api/ux-guidelines/overview), then apply [native implementation boundaries](AGENTS.md#native-implementation-boundaries). For commands, inspect [registration, arguments, contribution, visibility and enablement](https://code.visualstudio.com/api/extension-guides/command) together. Give user-facing contributions meaningful titles; choose category, icon and menu group for their actual location. Explicitly control Command Palette visibility for actions meaningful only in another surface.

The selected [command skill](https://github.com/github/awesome-copilot/blob/997e95a6e42869c350f8ca6ec4c066287697c9f0/skills/vscode-ext-commands/SKILL.md) is a short review aid for those attributes. Its `_…#sideBar` ID pattern is an upstream convention, not a VSCode requirement; preserve the application's command namespace. A menu `when` condition does not replace handler validation. Verify the affected visible path through [actual-host acceptance](acceptance.md).

## Webview boundaries

Read the [Webview guide](https://code.visualstudio.com/api/extension-guides/webview) for the affected lifecycle, resource, message or security section. Use a Webview when native APIs cannot adequately express the feature. For a custom editor, also select the [custom editor contract](https://code.visualstudio.com/api/extension-guides/custom-editors); the document and its save/undo lifecycle need an owner beyond the frontend.

Keep the extension host responsible for resource access and business operations. Define messages with explicit types, validated payloads and request/resource identity. Reject messages for a disposed or superseded view. Keep `acquireVsCodeApi()` private to the frontend and acquire it once per session. A frontend message is a request; it does not establish a completed save.

Use `asWebviewUri`, bounded `localResourceRoots` and a Content Security Policy that enables only required content. Treat workspace text, paths and settings as untrusted HTML input. Preserve drafts across hide/reveal through explicit state; prefer `getState`/`setState` when adequate and justify `retainContextWhenHidden` by an actual need. Neither mechanism replaces durable business storage.

Apply the existing [web development routes](../webdevelopment/AGENTS.md) to frontend design and implementation. Test pure frontend behavior in a browser when useful, then exercise actual Code messaging, resource loading, focus and hide/reveal for integration claims. [Tool selection](tools.md#select-the-interaction-surface) distinguishes Webview content from workbench DOM and OS menus.

At a meaningful Webview checkpoint, retain the panel/frame's actual structure, the needed frontend and Host state, and its rendered surface through [the checkpoint plan](acceptance.md#plan-the-three-layers-at-checkpoints). Correlate message request/resource identity and view generation with the independent Host effect. A standalone frontend fixture or `getState` value is not a complete Code runtime snapshot.

## Host and workspace support

Read [extension-host placement](https://code.visualstudio.com/api/advanced-topics/extension-host) when changing `main`, `browser`, `extensionKind` or a process boundary. Local desktop Node, remote workspace Node and browser worker hosts have different resources. A Webview is a frontend surface; a web extension runs its extension code in a browser worker. One does not imply the other.

For newly supported [web extensions](https://code.visualstudio.com/api/extension-guides/web-extensions) or [virtual workspaces](https://code.visualstudio.com/api/extension-guides/virtual-workspaces), use URI-aware public APIs such as `workspace.fs`; an arbitrary URI is not a local `fsPath`. For remote operation, check the [remote extension guidance](https://code.visualstudio.com/api/advanced-topics/remote-extensions) before interpreting paths, `localhost` or process location. A desktop-only extension may explicitly reject unsupported contexts; do not expand its platform promise as a side effect of a local fix.

Apply [Workspace Trust](https://code.visualstudio.com/api/extension-guides/workspace-trust) when workspace content/configuration can choose code or executables. Align manifest support, runtime admission and restricted configuration with the product's policy. Test only promised environments, including the affected unsupported/refusal path; a trust-disabled fixture cannot prove Restricted Mode behavior. Preserve dirty `TextDocument` content and explicit multi-root selection where the operation depends on them.

## Language features

Start with the [public provider/LSP mapping](https://code.visualstudio.com/api/language-extensions/programmatic-language-features). A focused `vscode.languages` provider can be sufficient. Select a [language client/server](https://code.visualstudio.com/api/language-extensions/language-server-extension-guide) when separate analysis execution or reuse across editors justifies its additional protocol and process lifetime.

Record supported document selectors, URI schemes, cancellation and document-version handling. For LSP, use the selected [protocol version](https://microsoft.github.io/language-server-protocol/) and client library declarations for initialization, negotiated capabilities, synchronization and shutdown. Test an edit during analysis and the real visible diagnostic/completion result against the intended document. A server response for an old version must not overwrite current state. Compilation or a standalone parser test does not prove client/server integration.

## Debugger integration

Use the [debugger extension guide](https://code.visualstudio.com/api/extension-guides/debugger-extension) and the selected [DAP contract](https://microsoft.github.io/debug-adapter-protocol/). Distinguish contributions and launch configuration resolution from adapter transport and target process ownership. Track launch versus attach, initialization/configuration, breakpoints, stopped/continued events, and disconnect/termination according to the advertised capabilities.

Choose a disposable target for real adapter acceptance. Establish a breakpoint hit, source/stack/variable correspondence and correct detach/termination for the changed behavior. Mock Debug illustrates the protocol and UI; it cannot establish that the actual debugger controls the intended process. Debugging the extension's own code follows [debugging and validation](#debugging-and-validation) and does not require implementing DAP.

## Localization and accessibility

The [localization skill](https://github.com/github/awesome-copilot/blob/997e95a6e42869c350f8ca6ec4c066287697c9f0/skills/vscode-ext-localization/SKILL.md) usefully separates manifest contributions, walkthrough Markdown and runtime strings. Apply the [official localization tooling](https://github.com/microsoft/vscode-l10n#readme) for exact mechanics: `%key%` plus `package.nls.json`/locale files for manifest text, and `vscode.l10n.t` plus the declared bundle location for supported runtime APIs. Check the existing engine baseline before adopting that API. Maintain the product's supported locales and fallback; do not translate command IDs or introduce a second localization system merely to follow the skill.

For native controls, use public accessibility properties supported by the selected API. For Webviews, provide semantic controls, meaningful accessible names, keyboard operation and focus restoration, and follow [theme variables](https://code.visualstudio.com/api/extension-guides/webview#theming-webview-content) and [screen-reader/reduced-motion preferences](https://code.visualstudio.com/api/extension-guides/webview#accessibility). Check the changed locale/theme/focus state in the actual surface. A DOM label check does not prove screen-reader output; record whether actual assistive technology was exercised.

The core VSCode accessibility skill is a research reference only. Its `IAccessibleViewImplementation`, registry and signal services belong to VSCode internals. Do not import them or require a new workbench accessibility service from an ordinary extension.

## Debugging and validation

Select the failing runtime first: extension host, Webview/workbench renderer, or external server/adapter. Use its logs, debugger and source maps. Follow [official extension testing](https://code.visualstudio.com/api/working-with-extensions/testing-extension) for real host tests; use an existing prepared runner before adding dependencies. `@vscode/test-cli`/`@vscode/test-electron` support desktop integration and `@vscode/test-web` supports web-host testing. Their setup may download Code or require packages, so a documentation example is not an available local command.

For user-visible behavior, use [acceptance](acceptance.md) and [the current application harness](tools.md#reuse-the-current-application-harness). Retain the actual input, checkpoint layers, assertion and independent resulting state. Account for virtualized lists before calling an item absent. A required step that cannot run stays unverified with its concrete missing tool or observation. For a pure API/rule change, keep the smallest affected contract check; do not add UI checkpoints solely to populate a report.

For suspected leaks, inspect resource ownership across repeated open/change/close operations. Bind subscriptions to the shortest responsible lifetime, dispose replaced registrations, and observe whether listeners or owned processes accumulate. Use `ExtensionContext.subscriptions`, public `Disposable` contracts and the project's existing lifetime helpers. The core memory-audit method supplies useful ownership questions, but its internal classes/test helpers and chat-specific thresholds do not apply to extensions. Use profiling when observations warrant it; do not create a repository-wide leak campaign for a bounded change.

## Packaging and publication

Follow the application's build/release owner and the [official packaging guide](https://code.visualstudio.com/api/working-with-extensions/publishing-extension) only when producing a package or release. Inspect included activation output, runtime dependencies, assets/localization, exclusions, identity and `engines.vscode`. Verify the resulting artifact in an owned disposable host when package composition changed. A development-source run does not prove the VSIX contains everything needed.

Separate package preparation from Marketplace publication or installation into a user's environment. The Task's existing authority decides which actions are allowed. Keep credentials with the release owner, and preserve installed state and immutable releases. Do not copy tutorial install/publish commands into a development acceptance run.

## Optional tool skills

These researched sources are not registered or executed by this guide. Select one only for a concrete gap after checking current tools, dependencies and Task authority. Read its native entry and apply the local adaptations below before execution.

| Candidate | Useful condition and method | Local adaptation |
| --- | --- | --- |
| [Playwright Interactive](https://github.com/openai/skills/blob/49f948faa9258a0c61caceaf225e179651397431/skills/.curated/playwright-interactive/SKILL.md) | Persistent browser/Electron inspection when `js_repl` and Playwright are available; connect claims to functional and visual checks | Keep the actual Code runner and owned target. Distinguish renderer reload from host/process restart. Do not import setup/settings changes, a generic Electron launch, or fixed exploratory quotas. Verify CSS/device-pixel mapping before coordinate input. |
| [Webapp Testing](https://github.com/anthropics/skills/blob/33375500bcea98d610eb30ce10ac4e59b89c390d/skills/webapp-testing/SKILL.md) | Standalone Webview frontend with an available Python Playwright environment; inspect rendered controls before selecting actions | Wait for application state rather than requiring `networkidle`. Browser checks do not establish Code integration. Keep process ownership with the application runner; the bundled server helper does not prove descendant cleanup. |
| [Screenshot](https://github.com/openai/skills/blob/49f948faa9258a0c61caceaf225e179651397431/skills/.curated/screenshot/SKILL.md) | OS surface missing from tool-specific capture | Prefer the existing PID-bound window capture. App-name matching can capture several windows, and its macOS helper can request permission automatically. Use only an exact owned target and existing authority; capture does not provide native input. |

The external references retain their pinned source revisions. Read a selected native entry and its resources before using it. Static method references do not establish that its tools are installed or that they ran in the current environment.
