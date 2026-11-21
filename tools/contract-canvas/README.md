# Contract Canvas

Contract Canvas is an offline workbench for reading a project's domain object contracts. Explore declaration layers, properties, method inputs and results, and typed relationships. Preview and pin a reference, enter a lower layer and return, or follow a documented scenario. The viewer does not execute any contract or infer declarations from source.

**Try it:** Open [the library example](example/index.html) in a modern browser. In **借阅预览**, inspect `LoanDesk.previewLoan`, preview its `CatalogCopy` input from the catalog layer, and compare it with the returned `LoanSlip`. The same files also work on a static server.

| I want to… | Start here |
| --- | --- |
| Use the tool in another project | [Quickstart: create, check and publish a project](docs/quickstart.md) |
| Write objects, fields, ports, dependencies or scenarios | [Model authoring and format](docs/model.md) · [complete model example](example/model.json) |
| Set the initial layer, labels or optional diagrams | [View configuration](docs/view.md) · [example view](example/view.json) |
| Explore a model using the workbench | [Viewer controls](docs/viewer.md) |
| Maintain or integrate the tool | [Architecture, APIs and maintenance](docs/maintenance.md) · [agent entry](AGENTS.md) |

Each consumer owns two JSON files: **`model.json`** states what the project declares and how those declarations relate; **`view.json`** chooses how to present those facts. The tool owns the [TypeScript shape](schema.d.ts), [validation](validate.mjs), [static builder](build.mjs) and [shared viewer](viewer/). Copy this directory on its own to reuse it. No other repository files or third-party packages are required. Authoring/building uses Node.js 18 or later; reading the generated files uses only the browser.
