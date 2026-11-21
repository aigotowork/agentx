---
schema: scenario
description: Apply a complete Rust source-layout change, then verify and repair the result as one change set
tags: [rust, migration, refactoring]
---

# Rust module migration

Use this scenario for directory, file and module naming migrations that preserve program behavior. Start from the user's authorized scope and a complete old-to-new path map. Finish the whole change set before running compilation or tests.

## Plan the complete layout

1. Read the source entry points and destination ownership. Record the target tree, responsibilities and exact relocation map, including unchanged files and resources.
2. Include directory moves, module renames, declarations, visibility, imports, re-exports, relative paths, embedded assets and build/test configuration in one plan.
3. Keep unrelated areas out of scope. A symlink or plugin outside the source migration does not justify moving it or expanding the tooling project.
4. Preserve existing user edits. Use the current workspace or a bounded source snapshot when it helps; an isolated copy is optional, not a mandatory workflow.

## Apply the whole change

- Move and rename the planned files, create their owning modules, and update all affected references before checking the build. Temporary unresolved imports are expected while the change set is incomplete.
- Use suitable semantic tooling or a consistent scripted transformation where useful. The Agent may directly repair imports, declarations, visibility, macro paths and resource references. A tool limitation is not a reason to stop an authorized migration or build a larger tool.
- Do not compile, run tests, perform analyzer validation, or preview the whole project after every individual file or module move. Do not repeatedly reload an analysis service or start a service for every file.
- Keep code behavior, external CLI/configuration spellings, storage formats and installed resource locations unchanged. Necessary module wiring and source-relative path adjustments belong to the migration.
- If interrupted, inspect the current files and continue the unfinished portion. Do not blindly replay completed moves, overwrite unrelated edits, or reset the repository.

An already adopted semantic migration tool is optional support. Inspect its actual contract: a runner that preserves module names or validates every individual move may not fit a broad rename/layout change. Use suitable source edits instead of changing the intended migration to fit a tool.

## Verify once the layout is complete

1. Check that every mapped source has its intended destination, old paths have been removed where planned, and assets and non-Rust consumers use the new paths.
2. Run the project's relevant build, formatting, type and behavior checks against the complete result. Use the project's existing recipes and declared packaging/version checks when the assignment includes an executable delivery.
3. Diagnose any failures and fix the affected code or paths directly. Rerun only checks justified by those fixes, then complete the remaining required checks.
4. Inspect the final diff for accidental behavior changes and report the migration, evidence and remaining limits accurately.

The outcome is a coherent, reviewable layout and verified behavior. Per-file green checkpoints, a particular tool, and a sequence of successful previews are not completion criteria. No extra permission, commit or publication step is implied.
