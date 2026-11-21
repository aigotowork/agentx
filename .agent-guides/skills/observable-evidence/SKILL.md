---
name: observable-evidence
description: Verify a web, editor or terminal interaction by retaining semantic structure, relevant state and rendered output at the same meaningful checkpoints. Use for UI behavior, temporal bugs and scenario evidence.
---

# Observe an interaction

Identify the user goal, initial state, action and expected visible result. Reuse the consumer's scenario, runner and artifact owner. Choose the actual host: browser DOM, editor native UI/extension host, or terminal PTY; a simulated reducer does not prove the host interaction.

1. Establish the exact build, host, viewport or terminal dimensions, resource scope and observation facilities. Choose checkpoints around meaningful state transitions, especially waiting, error, cancellation and completion.
2. At each checkpoint retain three connected observations where available: semantic structure and available actions; relevant state and resource identity; rendered pixels or terminal cells. Record missing layers and synchronization uncertainty instead of filling them from expectation.
3. Drive the real input once and observe the result. Wait for a bounded condition rather than an arbitrary delay. For terminal interactions preserve argv, TTY status, dimensions, key events, exit and stream identity. For editor/browser interactions preserve navigation and connection identity.
4. Compare the result with the intended goal and the independent effect where relevant. A click, socket flush or successful transport return alone does not prove the receiving system performed the operation. Lost replies remain unknown; do not replay mutations to improve evidence.
5. Fix within scope, then re-run the affected path with the same meaningful basis. Keep original observations distinct from new evidence and reconstructed views. Link evidence and limitations from the owning Task.

Use available public observer/Termwright contracts for collection; do not assume a particular runner or MCP server is installed. Unavailable pixel capture still permits bounded semantic/state checks if reported honestly. Avoid collecting unrelated user content or credentials. Stop after the requested behavior has sufficient evidence, not after a predetermined screenshot count.
