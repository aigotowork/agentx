---
name: tsxctx
description: "Read component/package contracts, jsx, symbols for js/ts/tsx/mjs... files, focused source without loading whole files. Precious and saving tokens."
metadata:
  short-description: Compact JS/TS context reader
---

# tsxctx

Run from the project root, or use `--cwd <path>`. Choose by question; inspect `tsxctx <command> --help` for additional options.

Accepts `.js`, `.jsx`, `.mjs`, `.cjs`, `.ts`, `.tsx`, `.mts`, `.cts`, and TypeScript declaration files. The nearest `tsconfig.json` or `jsconfig.json` supplies import-resolution settings; `--tsconfig` accepts either. File support does not imply an exhaustive language index: CommonJS `require()` / `module.exports` / `exports.*` forms are not yet collected as import/export entries, though their named local functions remain readable. 

## Choose a read

| Need | Command |
| --- | --- |
| Component props and referenced type locations | `tsxctx usage <file> <component>` |
| One component's layout | `tsxctx read <file> <component> --kind component --level outline` |
| Whole-file JSX layout | `tsxctx layout <file>` |
| One JSX node and its business props | `tsxctx read <file> <node> --kind jsx --level cleaned` |
| File structure, state, and handlers | `tsxctx outline <file>` |
| Locate a target before reading | `tsxctx symbols <file>` |
| Full function, component, or type | `tsxctx read <file> <target> --kind <kind> --level full` |
| Known source lines (one-based, inclusive) | `tsxctx read <file> 52:81` |

Name the component in multi-component files. Start with the known target; expand to a whole-file view only when surrounding context is needed. Do not fetch the same implementation through multiple tools.

## Read results correctly

- File-purpose guidance appears directly below `FILE`; declaration contracts appear beside their declarations or under component `GUIDANCE`. Inline and multiline AI tags are equivalent. Use `read --kind type` or `read --kind interface` for exported type contracts; `usage` selects components.
- Default `layout` drops all non-event attributes, including business props. Use `--styles keep` for complete attributes; focused JSX reads can use `--level raw` for original JSX.
- Outlines can omit effects such as bare `useEffect` calls. Read full source for behavior, timing, and cleanup; include outer variables when reading a nested function. Before editing, inspect affected source and callers; refresh locations after edits.
- Ambiguous target: specify the component or `--kind`. Missing target: use `symbols`. Tool unavailable or parsing fails: use `rg` and focused source reads.
