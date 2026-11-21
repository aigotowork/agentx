# Contract design inspector

Read the parent tool entry and [README](README.md). This is a read-only projection of `authoring/contracts.ts::InspectorArtifact`, not a business runtime, trace player or a second contract store. `build.py` owns offline file I/O/embedding; `inspector.js` owns DOM/SVG projection and local selection. It must not import TS application entry points or execute handlers. The existing authored-trace viewer retains its own player and layout semantics.

Use textContent for all author text. Only explicit HTTP(S) documentation links are active. Relative source/document paths are displayed as text because a portable HTML or single-artifact server has no repository root resolver. Never turn them into broken links or enable executable/data/absolute filesystem schemes. Do not infer state changes or real execution from a declared possible process edge. Targeted cases are `tests/flow_atlas_inspector.py` and `tests/flow_atlas_inspector_browser.mjs`.

Viewport pan/zoom and container geometry are owned by [spatial](../spatial/AGENTS.md); the inspector mounts each freshly rendered SVG into that camera using view/process scope. Selection repaint must retain the camera. Inspector diagrams are declarations; camera movement does not modify them.
