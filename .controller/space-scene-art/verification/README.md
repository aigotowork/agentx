# Studio review

Use the actual static Space build, served on an isolated origin. The delivered path explicitly selects Studio art over the trusted public SDK host. The same runner can exercise the no-network authoring sample by omitting its `sdk` argument and building without a trusted host.

```sh
VITE_AGENT_SPACE_MANAGEMENT_URL=http://127.0.0.1:18991 \
VITE_AGENT_SPACE_EXPECTED_SCOPE=/fixture/agentx-space-studio/.agentx \
VITE_AGENT_SPACE_PREVIEW_CAPTION=工作室样例 \
node web/agent-space/scripts/run.mjs build /tmp/space-studio-candidate
python3 .controller/space-scene-art/verification/preview.py --bind 127.0.0.1 --port 18991 --directory /tmp/space-studio-candidate
node .controller/space-scene-art/verification/studio.browser.mjs http://127.0.0.1:18991/ /tmp/studio-evidence sdk
```

The two directory paths must be new. Use `AGENTX_PLAYWRIGHT_PACKAGE` if Playwright is installed outside project dependencies; use `AGENTX_BROWSER_EXECUTABLE` for a different Chrome binary. This server only serves built static files and known GET resources and rejects all mutations; it runs no daemon or Agent. The report and screenshots come from the actual app: view an actor and their status/affordance, open the matching conversation, edit and switch to list, reopen the same draft, verify a separate actor, use the touch/keyboard directory in 390px reduced motion, and inspect an empty room. The PNG from the diagnostic facade confirms actual Canvas presentation; ordinary screenshots include interactive HTML overlays. In SDK mode the report includes actual Workspace and exact Agent HTTP GETs. Evidence is for browser interaction and expert visual review with synthetic data, not real-user research or live adoption.
