---
schema: task
role: debugger
status: completed
description: Resolve the filtered multi-Agent kill picker regression found during project release.
agents: [{"scope":"/home/long/works/longwork/agentx/.agentx","generation":"a6195a5b-1dd4-4a6d-87d4-1f18f1b664fb","participant":{"canonical_name":"domuzo","incarnation_bits":4745316233546143926}}]
---

The project release candidate was built successfully with `just package` and passed live read-only daemon/list compatibility. Do not publish, overwrite `.bin/agentx`, restart services, or run Cargo; the release controller owns that lane.

Investigate `tests/cli_target_selection.py` at its filtered multiselect step (around line 98). Candidate and failure log are under `.artifacts/project-release-20261120-134940/` (`agentx`, `cli-target-selection.log`). In the PTY trace, the query for the first Agent selects one item; after Ctrl+U and entry of the second Agent name, selecting appears to toggle the first Agent off and Enter never opens review. Determine whether the fixture, search state/cursor, or PTY interacts incorrectly. Rerun the focused fixture against this exact candidate as needed, then make the smallest justified source or test correction, preserving other edits. Report cause, changed paths, and the exact verification to use with the next package. Coordinate through the parent; do not touch TUI transcript smoke (owned by parent).

## Diagnosis and verification

The [failed PTY trace](../.artifacts/project-release-20261120-134940/cli-target-selection.log) shows `kuna` narrowing the inventory and selecting its first row. Ctrl+U clears the query and restores all three rows. The second query, `nage`, matches the shared `agentx-ma[nage]ment-…` working directory on every row. [The selector](../src/tui/select.rs#L149) searches the name, label and description, and resets the cursor to the first match after each edit. Consequently the second space toggles `kuna` off and Enter has no marked target to review. The fixture's assumption that a generated name uniquely narrows the list caused the failure; the trace shows the intended key sequence arriving and the selector following its current search contract.

The [fixture](../tests/cli_target_selection.py#L89) now uses distinct launch titles for the two search inputs and waits for `No matches` when searching for the same-label Agent from the excluded workspace. It still confirms the two-target review and checks that the two selected Agents stop while the third Agent and the excluded Agent stay live. Product code remains unchanged.

The corrected fixture passed all 10 scenarios against the original package (`4c21590aeaf3f5527627ca78137681051d6951731a33c06a6c5a91afca035c35`). The [run result](/tmp/agentx-target-acceptance-qdo4j0rt/run/selection-result.json) and [manifest](/tmp/agentx-target-acceptance-qdo4j0rt/run/manifest.json) record the exact artifact, unchanged binary and clean fixture teardown. No build or deployment was performed. For the next package, run:

```sh
PYTHONDONTWRITEBYTECODE=1 python3 tests/cli_target_selection.py /absolute/path/to/candidate/agentx
```
