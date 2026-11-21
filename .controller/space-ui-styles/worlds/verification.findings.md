# Space worlds browser findings

The independent Chromium run passed all eight browser cases against the Controller's r2 builds and the shared Studio fixture. The run used intercepted responses to serve those existing build files and the fixture's real GET payload function at the assigned URL origins. It exercised the real app, SDK requests, renderer and Design Canvas scenes in a fresh browser context for each case. It did not reach either LAN listener from this sandbox.

| Surface | Observed result | Evidence |
| --- | --- | --- |
| Space, 1440×900 | The same focused `/studio/catalog` office painted six actor anchors in each of Lunar, Terrarium and Newsprint. A pointer click opened `@mora`, current incarnation `101`, and rendered the fixture reply containing R-14. Each reader decoded its scout portrait and accepted the local draft. | [Desktop screenshots](../../../.artifacts/space-ui-styles/worlds-zuhuro-04/desktop-comparison.png) |
| In-place desktop switch | After a deliberate camera pan, the accessible style selector ran Lunar → Terrarium → Newsprint → Classic → Lunar while the reader stayed open. The exact route and selected object, draft, custom camera and Canvas node/registry ID stayed the same. The presented frame advanced 13 → 15 → 17 → 19 → 22. Each world and Classic displayed a distinct portrait; Lunar restored its initial asset. The fixture saw two exact GET requests at initial open and zero more during all style switches. | [Frame and HTTP report](../../../.artifacts/space-ui-styles/worlds-zuhuro-04/report.json) |
| Space, 390×844 | In each world, tap 名册 and open the `@mora` row. The decoded directory and reader portraits matched, history and controls fit, and the 374 × 66 px composer stayed inside the viewport. Switching to Classic and another world preserved the exact identity and draft without repeating exact reads. Tapping 返回办公室 restored focus to the actor. Reopening the same row from Chat with Enter recovered the draft and performed the expected fresh exact GETs. | [Mobile screenshots](../../../.artifacts/space-ui-styles/worlds-zuhuro-04/mobile-comparison.png) |
| Design Canvas, 1600×1000 | All three named whole-office cards mounted the working Canvas and painted their respective worlds and six actors. Lunar opened its local `@mora` reader, showed R-14 and retained a typed local draft. The board made zero API calls or mutation attempts. | [Design Canvas screenshots](../../../.artifacts/space-ui-styles/worlds-zuhuro-04/design-comparison.png) |

The browser report recorded zero page errors, failed API/art responses, unexpected origins or write attempts. Background Management polling used GET; the no-repeat assertion applies to exact `@mora` reads during style changes. Runtime continuity here means the same mounted Canvas with increasing committed frame IDs under reduced-motion emulation. The runner did not measure transition smoothness, hardware touch or native effects.

The three worlds are recognizable before the reader opens: Lunar's cold steel and orange equipment, Terrarium's green canopy and warm greenery, and Newsprint's outlined characters and inked room. The narrow reader keeps identity, tabs, style selection, visible history and draft input in view. Two shared composition issues are visible: at 390 px the status filter text is cut short next to 联系目录; on desktop the large Lunar and Newsprint readers cover part of the scene and leave the stopped-history label partly visible behind the panel. Neither blocked the tested read, draft or return flow. Desktop office framing leaves substantial empty stage around the cast; a closer initial crop would make portraits easier to distinguish, especially in Lunar.

Run [worlds.browser.mjs](worlds.browser.mjs) in an environment that can access the listeners to close the remaining direct HTTP evidence gap:

```sh
node .controller/space-ui-styles/worlds/worlds.browser.mjs \
  http://192.168.2.178:18996/ http://192.168.2.178:18997/ \
  .artifacts/space-ui-styles/worlds-direct-NEW
```

The runner honors `AGENTX_PLAYWRIGHT_PACKAGE` and `AGENTX_BROWSER_EXECUTABLE`. For the replay, append `/tmp/agentx-space-worlds-r2 /tmp/agentx-space-worlds-canvas-r2` to the command and use a fresh output directory. The evidence run used Chromium headless shell 156.0.8078.4. The [direct attempt](../../../.artifacts/space-ui-styles/worlds-zuhuro-02/report.json) returned `ERR_EMPTY_RESPONSE` from this sandbox; the Playwright MCP was also denied by the session's approval policy. Application and asset files, listeners and live Agent resources were untouched.
