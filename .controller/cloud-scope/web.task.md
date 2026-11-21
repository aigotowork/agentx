---
schema: task
status: completed
agents: [{"scope":"/home/long/works/longwork/agentx/.agentx","generation":"a6195a5b-1dd4-4a6d-87d4-1f18f1b664fb","participant":{"canonical_name":"voru","incarnation_bits":4745316805035836950}}]
---
Read [the integration contract](AGENTS.md).
Implement and test Management deployment Web client in web/management using generated SDK. Coordinate with SDK owner.

## Web design and verification contract

- `/deployment` is a distinct entry in the existing Management bundle. Use the browser's own origin and accept the admin bearer in a password field; server environment values cannot be browser defaults without exposing the secret. No browser storage, token URL parameters, or local Management requests. Let the SDK enforce HTTPS or loopback HTTP.
- The selected Scope anchors the operator workspace: deployment totals and contextual identity above the Scope list, exact URL/version at the Scope heading, Hosts and Agents as independently paged views. A host opens a server-filtered Agents page. A narrow viewport shows the list or the selected Scope with a back action. Reuse Management tokens, row selection, and dialog focus behavior. No charts or parallel management navigation.
- `deployment/console.tsx` owns the entry and authority lifetime; `workspace.tsx` owns the selection and resource views; `write-dialog.tsx` owns credential input and receipt recovery; `model.ts` binds the public SDK and bounded reads. No direct HTTP routes in product components.
- One explicit create/rotate captures the selected ID, version (rotate), request ID, and credential. No automatic replay; an uncertain result allows only an explicit same-input retry or explicit close with loss of browser recovery. The token remains available to copy before either mutation and after a successful receipt; page exit clears it. Existing native receipts are token-free.
- Browser cases: rejected bearer followed by successful overview/list; deployment identity; paged Scopes, Hosts and Agents with an exact host filter; empty and unavailable views; a dropped post-commit response followed by explicit same-input recovery of the original version; a rotation conflict; mobile list/detail/back; no persisted credentials; insecure origins withhold the admin input. The controller's isolated native acceptance owns real runtime composition.

## Web outcome

[Management entry](../../web/management/AGENTS.md#cloud-deployment-entry) links implementation and run commands. The final standalone Management build passed against the SDK generated from the native Admin projection (including exact `host`, `deployment_id`, and `protocol_version`). [Deployment browser cases](../../web/management/tests/deployment.browser.mjs) passed in Linux Chrome against the [isolated HTTP fixture](../../web/management/tests/deployment.fixture.mjs), including a v5 replay receipt after another operator advanced the live version to v6. The existing [local Management browser suite](../../web/management/tests/browser.mjs) passed on the split entry before subsequent deployment-only refinements. The temporary manual preview was stopped after verification. No native or live service changes were made by this task.

To reproduce, build with `node web/management/build.mjs BUILD_DIRECTORY` (a new absolute path), then run `node web/management/tests/deployment.browser.mjs BUILD_DIRECTORY`. The browser test starts and stops its own fixture. The [Management entry](../../web/management/AGENTS.md#cloud-deployment-entry) documents the separate on-demand manual preview command.

Build coordination finding: the shared SDK build removes `dist/` before recompiling it. A concurrent standalone Vite build briefly saw an absent package entry; the final build and browser verification passed after SDK publication finished. Sequence SDK publication before the Web bundle when building this slice in a shared checkout.
