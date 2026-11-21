# Contract Canvas

Start at [README.md](README.md) for the documentation map. All general authoring, usage and maintenance guidance must work from a standalone copy of this directory.

| Change | Read and update |
| --- | --- |
| Create a project, build or host output | [Quickstart](docs/quickstart.md) and [builder](build.mjs) |
| Model shape, typed dependencies or evidence | [Model guide](docs/model.md), [schema](schema.d.ts) and [validator](validate.mjs) |
| View configuration or reading paths | [View guide](docs/view.md), [schema](schema.d.ts) and [validator](validate.mjs) |
| Viewport, diagrams, navigation or disclosure | [Viewer usage](docs/viewer.md) and [viewer implementation](viewer/) |
| Public build APIs, ownership or compatibility | [Maintenance guide](docs/maintenance.md) |
| Example data or scenario | [Library model](example/model.json), [view](example/view.json) and [design source](example/DESIGN.md) |

The tool owns validation and static presentation. Consumers own declaration facts, view choices, and project-specific source policy. Keep consumer IDs, source paths and workflows out of reusable code. Escape authored strings at script/HTML/SVG boundaries and keep source locations inert. Keep the viewer openable from local files without fetch or imports.

Keep one authoritative reference for each concept and use links from other pages. Rebuild generated entries after changing their inputs, template or embedded helpers. Verify changes through the affected consumer, including the copied example when the portability contract changes.
