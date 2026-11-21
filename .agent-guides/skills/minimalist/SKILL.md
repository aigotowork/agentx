---
name: "minimalist"
description: "Use for coding tasks that need efficient implementation, reuse, or reduction of unnecessary dependencies and abstractions. Choose the simplest complete solution after understanding the real flow and repository contracts. Not a prose-writing or guidance-authoring mode."
---

# Minimalist

Solve the actual coding task with the least unnecessary complexity. Preserve explicit requirements, correct ownership and necessary safeguards. Follow the project's current instructions and contracts.

## Choose an adequate solution

After reading the task, relevant contracts and affected flow, consider in order:

1. The required behavior already exists: reuse it or change nothing.
2. An existing responsible module, helper or pattern can own the change: extend it there.
3. The standard library, native platform or an installed dependency fits the required semantics: use it.
4. A small implementation fits the current architecture: add it where the responsibility belongs.
5. Existing structure prevents a correct, coherent change: perform the refactoring required by this task.

Choose the first option that satisfies the real outcome and constraints. Do not distort the design to avoid a justified abstraction or dependency. A one-line solution is useful only when its semantics and readability remain clear.

## Keep the change complete

- Trace relevant callers before changing shared behavior; fix the responsible mechanism rather than only the reported symptom.
- Avoid speculative features, extension points, configuration and scaffolding. Introduce an abstraction when current responsibilities, complexity or consumers justify it; reuse count alone does not decide.
- Compare total complexity and maintenance cost. Fewer files or a shorter diff must not split ownership, duplicate rules or omit a required path.
- Include input validation, error handling, authorization and other safeguards needed by the task. Explain non-obvious contracts or decisions where maintainers need them. Explicit requests for each comment or safeguard are unnecessary.
- Use existing scope and authorization to proceed. Ask only when a material unknown changes the outcome or permitted action; do not repeatedly reconfirm an accepted requirement.
- Record material deliberate limits and their revisit conditions in the existing code/design/task context. Do not invent a parallel debt ledger.

## Verify and finish

Run the smallest appropriate checks for the changed behavior and the project's required contracts. Non-trivial logic needs meaningful evidence; trivial edits do not need a new test framework or tests that mirror the implementation. Report the outcome, relevant evidence and material remaining limits. Stop when the requested result is complete.

This Skill governs coding choices. For designing or writing governance content, use the project's authoring entry instead of applying a coding ladder or code-first output format to prose. It requires no companion Skills or persistent conversation mode.
