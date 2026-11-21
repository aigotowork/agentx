# Browser investigation console

Read the [original Task](../../.controller/observability/implementation-tasks/web.md), [UI design](design.md), [authored cases](cases.md), and [adapter contract](../../.docs/observability/browser-api.md). Native declarations in `api/native.ts` and the `ObservationApi` client are adapter-owned; never invent replacement DTOs or change disclosure in views.

| Responsibility | Owner |
| --- | --- |
| Connection and deferred application/resources | `entry.ts`, `index.html` |
| Investigation state, exact identity/era guards and read orchestration | `state.ts` |
| Native evidence formatting and safe semantic DOM primitives | `presentation.ts`, `elements.ts` |
| Persistent shell and contextual subject navigation | `app.ts` |
| Request, configuration and event evidence regions | `subjects.ts`, `configuration.ts`, `events.ts` |
| Scoped light theme, layout and narrow disclosure | `app.css` |
| Disposable finite asset build and strict compiler settings | `build.mjs`, `tsconfig.json` |
| API/transport/native-derived types | Adapter-owned `api/` |

Use existing TypeScript and root Vite tooling to bundle the public `@agentx/client` SDK; no UI framework or page redesign is introduced. `build.mjs` typechecks the page then emits only hashed finite assets in the native manifest. Build the public package before standalone checks. Cargo rebuilds it once before all browser asset batches and watches SDK source, scripts/config/manifests and installed compiler inputs. No bare npm imports or source-path `/api/` assets reach the browser. Existing page state owns polling/cancellation orchestration; public SDK owns transport, exact codecs and read guards. See [API boundary](api/AGENTS.md), [package setup](../../packages/agentx-client/README.md#web-consumers) and [current migration/evidence](../../.controller/web-sdk-adoption/web.task.md). Independent integration executes checks; source completion does not establish browser acceptance.
