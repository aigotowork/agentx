# Office public acceptance

Source of expectations: [office design](../../.docs/agent-management/offices.md). Exact serialization: [native DTOs](../../src/management/contract.rs). Execution/findings: [evidence](../../.controller/office-management/evidence.md). Existing [management acceptance](../agent_management/README.md) remains a separate unchanged regression suite.

Run `python3 tests/office_management/acceptance.py --candidate /absolute/immutable-candidate --evidence /absolute/new-results --browser`. No Cargo, installation or managed DB connection occurs. Local Qoder substitute runs candidate-generated hooks, a finite transcript and actual native replies. It establishes AgentX flow, not paid Provider interoperability. `--browser-only` is for browser-only changes on an otherwise unchanged accepted HTTP candidate and creates another disposable scope. `--actions-only` creates fresh endpoint/action/coverage fixtures for O05–O09 while retaining earlier read/provenance passes on an unchanged candidate; reports retain unexecuted case names, and combined evidence must link prior reports. An omitted browser run is explicitly incomplete evidence.

| Case | Independent setup and action | Required public observation |
| --- | --- | --- |
| O01 | Marked root, empty/nested office, `.agentx` only, outside symlink, ignored/deep trees | Finite known-office catalog; exact sources and honest coverage; no external/deep scan |
| O02 | Global, parent, shadowed child posts; invalid TOML/profile/default; overflow | Effective native post selection, canonical context, safe metadata; explicit whole-query failures |
| O03 | Global Role + explicit marker-free membership + unrelated cwd | Adopted membership, original global provenance, actual Provider cwd; independent axes |
| O04 | Edit/delete adopted Role, stop, read exact history, resume | Original provenance survives definition changes and successor; exact history remains pinned |
| O05 | Preview nested directory, unique/ambiguous compatible members, bound/drift | Exact inherited default and canonical directory; no launch/send/adoption from repeated GET |
| O06 | Directory send adopts one compatible member, reply, dropped response/replay, changed body | One exact recipient/effect, same receipt after config removal, no repeated Provider start |
| O07 | Absent parent/global endpoint → send creates → send reuses | Parent/global membership independent of requested cwd; actual native reply and one recipient |
| O08 | Admission errors, stale generation, relative selectors, role conflicts | Explicit rejection without launch/send; no file reads needed for syntactic rejection |
| O09 | >100 marked offices / >512 directories | Bounded catalog and explicit incomplete coverage |
| OB01 | Office navigation, empty office/post catalog/default preview | Actual embedded UI supplies known offices and actionable native posts; no Agent required |
| OB02 | Deliberate Role launch with independent membership/cwd and directory contact | Actual UI controls issue canonical command, show native receipt and reply |
| OB03 | Selection/draft, reload/poll, narrow viewport, coverage/error disclosures | Exact selection and per-recipient draft continuity; reachable actions and no page overflow |

Public preview snapshots compare exact AgentViews, message windows and Provider starts before/after repeated reads. They cannot prove SQLite/KV/event byte-level nonmutation. Corrupt/legacy/versioned collaboration, bound failed/pending, missing-vs-retired/stale incarnation, and direct KV/event write witnesses belong to native owner tests and must be linked before full Delivery acceptance. Public clients never manufacture those states by editing managed storage. No schema or implementation branch is copied to decide expected outcomes.
