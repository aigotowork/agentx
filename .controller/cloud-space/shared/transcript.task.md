---
schema: task
status: active
agents: [{"scope":"/home/long/works/longwork/agentx/.agentx","generation":"a6195a5b-1dd4-4a6d-87d4-1f18f1b664fb","participant":{"canonical_name":"zehulo","incarnation_bits":4745317343516636543}}]
---

Implement exact cloud transcript, role/title/cwd and actual status from the managed Codex host, end to end through native contract, SDK inputs and shared UI. After launch navigate to exact transcript. Keep data bounded, incremental, authenticated, coalesce updates, no repeating full transcript, no raw secrets/reasoning. Prefer host publishing sanitized bounded projection upon changes via existing uplink. Own transcript/metadata native and web files, coordinate shared workbench.ts with parent (edit small hunks only). Parent owns SDK regeneration and refresh loop. Preserve dirty changes. Verify targeted checks; no deploy. Record wire changes and evidence here.

Design: The owning Codex supervisor extracts visible user text, public commentary and final answers, plus constant tool activity labels from its exact thread. A bounded batch worker sends only new redacted records plus changed or renewing facts over the existing supervisor-authenticated IPC and scope bearer. The scope store owns exact admission, idempotency, retention and ordered pagination. Admin and scope reads use the same generation-fenced contract. Space projects these facts into its existing roster and supplies the same Conversation reader with separate older and live cursors; the parent owns change observation. Legacy roster consumers opt out of facts by default. Historical input is a finite recovery scan, with no background re-read of the full Provider transcript.
