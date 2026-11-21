# Explore a contract

[Documentation home](../README.md) · [Open the library example](../example/index.html)

Start with the layer selector. The directory groups the chosen layer's declarations by domain; the middle canvas shows the model or current contract; the inspector shows fields, ports, source evidence and related objects. On small screens, the directory and inspector open as drawers.

| Task | Action |
| --- | --- |
| Find a declaration | Use the directory search or press `/`. Select a result or press Enter to open the first match. The focused layer scopes the search. |
| Read an object or method | Select a type for details or use **完整契约** on its layer card. Method ports expose inputs, output and errors/outcomes. The optional overview, walkthrough and sequence are driven by the project's configuration. |
| Look up a related contract without leaving | Hover or focus a reference for a summary, or click/tap it to interact. On keyboard, Enter, Space or ArrowDown opens the interactive preview. Related fields inside it can open nested previews; back or Escape returns locally. |
| Compare contracts | Use **固定对照** in a preview. Up to three cards can be kept, folded or removed individually; they stay available when changing scenes and layers. |
| Inspect lower-level implementation | Expand implementation details or **跨层关系**. Filter the relation tray to the current subject or whole layer, and inspect the exact field/port, target and any mapping/source evidence. |
| Drill down and get back | Choose **进入画布** from a reference. Crossing a layer boundary offers return to the origin's scene, selection, camera and reading position. Manual layer switching also remembers local exploration. |
| Read provenance | Inspect the proposal/source marker, support text and source disclosure. Copy a path and line to find it in the project's source. Neither the status nor the walkthrough proves runtime behavior. |
| Change what is visible | Use dependency trace, fit, zoom or background drag where the viewport supports it. Reset clears local exploration and pinned comparisons; reloading starts a fresh session. |

Try the library walkthrough: select **查看凭条**, open the preview for `LoanSlip` and pin it. Follow its `copy` reference to inspect `CatalogCopy`; entering that type takes you to the catalog layer, and the return action brings you back to the walkthrough.
