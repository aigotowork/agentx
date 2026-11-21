---
schema: document
status: active
description: Directory/module hierarchy, compact code and separation of visible structure from detail
links: [ ./AGENTS.md, ./scenarios/engineering-design.md, ./domains/webdevelopment/implementation.md, ./domains/webdevelopment/interface-structure.md ]
---

# Structure and Compact Expression

Apply these preferences when writing or refactoring code, markup, and structured data in any language or format. TypeScript, HTML, JSON, and YAML/YML are examples, not limits on scope. Keep each level easy to scan: expose its responsibilities, sequence, and relationships, and place subordinate implementation details behind meaningful names. Optimize useful information density and local understanding together.

## Structure and detail

| Level | Make visible | Place in a cohesive subordinate part |
| --- | --- | --- |
| Project or domain | Responsibilities, public capabilities, and dependency direction | Internal implementation behind the owning module's public boundary |
| File or orchestration function | Main operations, ordering, decisions, and data flow | Parsing, calculations, formatting, or effects that obscure the main flow |
| UI composition | Layout, component relationships, and important states | Complex expressions, data preparation, and event behavior in named functions or components; see [interface structure](./domains/webdevelopment/interface-structure.md) for Page, Screen, Frame, Block and Component boundaries |

- Keep a structural passage at a consistent level of abstraction. Use names that explain the operation so readers can understand the main flow without opening every implementation. Keep important branching, error propagation, and side-effect ordering visible.
- Extract a detail when it has a cohesive purpose and a meaningful name that makes the caller easier to understand. A single caller can justify a private helper; reuse is not the only reason to separate a responsibility. Prefer a local helper first, then a separate file or module when its responsibility or context warrants that boundary.
- Preserve locality for details needed to understand the current operation. Avoid trivial pass-through helpers, one file per tiny function, speculative abstractions, or extraction that requires excessive parameters and navigation. Choose boundaries by responsibility rather than line-count targets or the mere possibility of extraction.
- Follow the [domain boundaries](./scenarios/engineering-design.md#domain-oriented-structure) when changing project or module organization. Apply extraction to the affected scope; a local edit does not require restructuring the project.

## Directory hierarchy as a module map

A project's directories, files, names and hierarchy are an architectural model that people and agents can read. Names convey purpose, containment conveys ownership and scope, and placement suggests where related work belongs. Design these signals deliberately so the structure makes responsibilities, public boundaries and intended collaboration understandable with little additional prose. Keep this model aligned with actual contracts and dependencies; language visibility, explicit exports and import/dependency rules enforce code boundaries.

Design responsibilities and collaboration first, then public contracts and implementation roles, and make the tree express that model. Iterate when the implementation exposes a mistaken boundary. Do not invent a folder template and force unrelated concepts into it, or assume that moving files alone has corrected their dependencies.

When repeated instructions are needed to explain where work belongs or how parts relate, first improve names, grouping or boundaries. Use prose for rationale, non-obvious constraints and exceptions that the structure cannot express, rather than narrating what a clear tree already shows.

| Situation | Organization decision |
| --- | --- |
| A small cohesive capability is readable as a few files | Keep it shallow; functions, types and an intentional public entry may be sufficient |
| One directory mixes several recognizable sub-capabilities or lifecycles | Group files into named capability directories; put related contracts, rules and implementations near their owner |
| Long repeated prefixes distinguish stable file families | Investigate those families as candidate submodules, then confirm responsibility and dependencies before moving; prefix similarity alone is insufficient |
| Multiple implementations vary behind a consumer-owned contract | Keep the port with its consumer and implementation-specific files in their adapter/Provider homes; composition selects the implementation |
| A nested folder is only private organization inside one module | Keep it private; it does not automatically need an exported index, package, interface or independent architecture unit |
| A child is an independently consumed capability | Give it an intentional public contract/entry and documented permitted consumers; hide internal details behind that boundary |

At a directory's root, expose the public entry and the main organization at a consistent level of abstraction. Avoid flattening all contracts, transport machinery, Provider variants and use cases into one list that requires memorizing filenames. Also avoid chains of one-child folders, generic `managers`/`helpers`/`types` dumping grounds, one-file submodules and rigid `interfaces`/`implementations` trees that separate details needed together. Use enough hierarchy to make relationships visible while keeping a common change local.

For a non-trivial reorganization, record a compact target tree and responsibilities before editing. Distinguish genuine module/public boundaries from private grouping directories, identify the contract owner and dependency direction, then materialize the skeleton and move implementations. Use an exact relocation map when many paths change. Preserve imports/exports, literal dynamic imports, worker/asset paths relative to source files, build/test roots and current documentation links. Do not leave a forwarding file for every moved private implementation just to preserve the old flat tree.

Assess the result with a concrete navigation/change scenario: from the tree and public entry, can a fresh reader locate the contract, its implementation and the composition point, and predict where a new implementation or behavior change belongs? Root-level file counts and depth can locate friction, but neither is a success target. Facade indexes without corresponding coherent implementation homes do not fix a crowded internal tree.

### Ownership and colocation

Place a subject's implementation and related contracts, specifications, styles/tokens, fixtures, tests and assets within its owning directory. Group by the subject that changes, not solely by file kind. A separate `interfaces/`, `specs/` or `styles/` directory needs a real responsibility or retrieval benefit; a shared suffix alone is insufficient.

For UI subjects, a local [design-system resource collection](./domains/webdevelopment/design-system.md#shared-and-local-homes) is a coherent presentation responsibility within that owner. It can group visual documents, tokens, artwork and animation resources without moving the subject's behavior contracts or collecting every component's private files at the project root.

Keep a capability at its narrowest useful scope. When compatible real consumers appear outside that scope, move it to their nearest meaningful shared owner and expose a deliberate public contract. Do not promote every reusable unit to the project root or couple consumers through a sibling's private files.

Give meaningful directories a concise responsibility description and an identifiable public entry, using existing local context or the parent's catalogue. Keep one authoritative description. Directory placement communicates ownership and intended use; exports and dependency rules enforce the boundary.

Place role-specific areas such as controllers, documentation or components at the scope whose work they serve. Explain meaningful nested ownership through local context; a repeated directory name does not make its contents project-global. Choose the areas needed by the actual scenario rather than creating the same directory set at every level.

### Naming and visibility cues

Use directory and filename names to make ownership and visibility legible when the project convention supports it. A module may group a public entry, its contract or ports, domain/application rules, and adapters or private implementation near the owning domain. For example, names such as `*.contract.ts`, `*.port.ts`, `*.adapter.ts`, `*.schema.ts`, `*.mapper.ts`, `adapters/`, or `internal/` can distinguish roles that would otherwise be easy to confuse. These are cues, not a universal template: use the project's established naming and export rules, and do not create a suffix or folder merely to satisfy a pattern.

For files describing one subject, prefer a shared subject stem and a purpose suffix when separate files are useful if is complex: `AddressEditor.tsx`, `AddressEditor.interface.ts`, `AddressEditor.spec.md`, `AddressEditor.styles.css`, `AddressEditor.tokens.ts`. Keep them together; create only needed files, and retain a small contract beside its implementation when clearer.

Expose deliberate public entries through the language or package mechanism (`index`, `public`, package exports, or the project's equivalent), and keep implementation-only files reachable only from their owner where the tooling permits. A `types` or `interfaces` folder does not establish ownership by itself, and an `index` that re-exports every adapter does not create encapsulation. Prefer names that express domain responsibility over technology alone, while retaining an implementation marker when several concrete roles coexist.

When a module has multiple implementations, keep the consumer-owned contract close to the capability that needs it and group concrete providers by their mechanism or deployment boundary. When a module is small, co-locate the contract and implementation rather than splitting them into rigid `interfaces/` and `implementations/` trees. The useful question is whether a fresh reader can locate the public contract, private details, and composition point from the tree without memorizing project-specific exceptions.

## Compact formatting

Across languages and formats, prefer one line or a few compact lines for short, simple constructs when their meaning and boundaries remain clear. This includes branches, declarations, calls, elements, records, lists, and configuration entries wherever the syntax permits. Use line breaks and blank lines to expose logical units, nesting, or changes of responsibility. Avoid spreading a trivial construct across many lines or adding empty lines between every statement or entry.

| Construct | Preferred expression when short and clear | Expand when |
| --- | --- | --- |
| Simple guard or single-action branch | `if (!ready) { return; }` | Conditions or branch bodies become complex, or multiple branches need comparison |
| Small literal, declaration, call, or pure function | `{ x: 0, y: 0 }`, `move(x, y)`, `(value: string) => value.trim()` | Items have independent meaning, nested structure, comments, or long expressions |
| Simple HTML/JSX element | `<button type="button">Save</button>` | Attributes, children, or conditions obscure the component hierarchy |
| Small data record, list, or configuration entry | JSON `{"x": 0, "y": 0}` or YAML/YML `modes: [read, write]` | Nested structure, comments, long values, or independently meaningful entries benefit from separate lines |
| Short sequence of related operations | Consecutive lines without unnecessary blank lines | A new logical stage begins or separation helps explain the flow |

Wrap long constructs at semantic boundaries such as arguments, fields, or child elements; never force a one-line form that requires excessive horizontal scanning. Keep necessary whitespace and indentation, meaningful identifiers, and required usage comments. Avoid packing unrelated statements, nested ternaries, or hidden side effects into a single line merely to reduce its size. Preserve language and data semantics, including significant indentation, multiline string values, and HTML text spacing.

Apply compactness within the project's enforced formatter and linter rules, including line width and brace requirements. Do not disable checks, add format-suppression comments, change formatter configuration, or reformat unrelated files just to force this preference. Use the clearest compact form the project permits.
