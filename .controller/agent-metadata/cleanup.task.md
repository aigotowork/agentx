---
schema: task
status: completed
agent: maho
description: Simplify Agent metadata across native contracts and all consumers
tasks: [./identity.task.md, ./launch.task.md, ./consumers.task.md, ./verification.task.md]
---

# Outcome

User requests complete removal of AgentX single-group metadata, its name prefixes, dedicated addressing/filtering, configuration, instructions, tests and documentation. Do not retain aliases or deprecated support. Future labels are documentation-only and are not designed or implemented in this work.

Native participant names and exact incarnation references remain the addressing model. Workspace and adopted Role retain their meanings. Remove the obsolete property from current schema and projections; fresh storage omits it. Existing database upgrades must remove the obsolete column without destroying rows or changing identities. Existing live services/databases and globally installed executables remain untouched; verify with temporary scopes and a separate candidate. The separately authorized launch-selection Task owns adoption of the exact verified candidate to project `.bin/agentx`, without restarting services.

## Ownership and method

This is a removal across independent large native and consumer surfaces. Three bounded implementation owners work in disjoint file sets; maho owns repository documentation, external fixtures and final integration. Native declarations own contracts. Use targeted source reads because the codebase-memory daemon failed to accept a client; no graph completeness is assumed.

Baseline was clean before this work. The earlier discussion note is replaced by the future-direction document. Complete scoped source changes before one coordinated build/test pass. Retain meaningful addressing and upgrade tests, replacing obsolete feature tests with applicable native cases. Report exact verification and limits here.

## Retained historical material

The superseded duplicate build source under `.artifacts/space-uiux-rollout-remo/candidate-r1/source` is removed from the maintained tree; current native/module sources are authoritative. A reversible local copy exists at `/tmp/agentx-metadata-cleanup-backup/source` for this editing session. Superseded static build directories beside that source copy are also removed, with the same local backup. Existing executable artifacts and running services are untouched.

Historical textual captures are normalized to the current metadata projection, so they are no longer byte-identical original protocol captures and must not be used as proof of current support. Original executable hashes still identify those historical binaries. New verification belongs to [verification](verification.task.md). The database access inventory is regenerated from current source rather than retaining embedded obsolete implementations from a pre-daemon revision.

## Integration release

All native source owners released their writes (suki 39259, mike 39362, bali 39508). maho released the complete source to lore for coordinated formatting, all-target compilation, internal/native tests, affected shell/stdio tests, one `.bin/agentx-metadata` package and isolated CLI/HTTP/generated-hook/browser acceptance. That initial candidate also served the separately authorized [launch-selection Task](../cli-selection.task.md); its rejected picker behavior required the corrected, separately bound artifact described below. tara owns project-path adoption of the accepted exact bytes, without a service restart.

Final source integration also updates transcript exact-name/history selection, onidle's native list template, external real-tool fixture JSON fields and generated observability bindings. Superseded embedded-source inventory captures now route to the regenerated [current inventory](../../.docs/model/db-access.md), rather than repeating removed implementations. The earlier diagnostic duplicate source under `.artifacts/humazo-deadlock/{mod,host}.rs` is removed with a reversible session copy in `/tmp/agentx-metadata-cleanup-backup/humazo-deadlock/`. Maintained sources remain authoritative.

## Residual source audit

The final tracked-text audit scanned 6,984 maintained source/document/fixture files, excluding runtime state, installed shared guidance, binary/dependency output and Git history. Removed API/environment/addressing identifiers and the former duplicate-name fields have zero remaining matches. The broader semantic scan retains only unrelated DOM/XML and Serde syntax, GitHub release identifiers, uv cache markers, PTY trace markers, cryptographic authentication and visual badge terminology. The bounded audit and dispositions are retained in `/tmp/agentx-metadata-removal-audit.json`; `git diff --check` passed after the integration/doc cleanup. Runtime acceptance remains owned by [verification](verification.md).

## Compatibility disposition

Removing serialized native fields changes the native protocol to epoch 3 while retaining scope, peer, daemon-instance and generation fences. Existing running services are not restarted by this Task. Configuration observability retains typed bodies in same-process memory only; the journal stores configuration references/stage metadata, without configuration field IDs or bodies. No durable old-body reader/import contract requires an additional compatibility path. The released source therefore contains no deprecated aliases or replacement metadata mechanism.

## Native verification disposition

All-target compilation passed after the bounded missing-import/warning repair. The first complete internal suite ran 3,316 cases: 3,276 passed, 21 failed, 19 ignored. Every failed guarantee subsequently passed its owner-repaired targeted retry; [the verification receipt](verification.md) retains exact first failures, repairs and executed scope. No second full internal suite was run. v27 preservation/rollback, mailbox restoration, historical projection and exact-identity guarantees have passing evidence.

All repair batches are released: bali 40307 preserves schema/history/message assertions while updating fixtures; suki 40467 preserves absent versus explicit-empty instruction sources and exact daemon fences while correcting attach/help/config fixtures; mike 40549 isolates observation worker fixtures from autonomous Inspector requests, corrects the MQTT username byte count and establishes the shutdown test's daemon dependency before its timed section. suki 40666 aligns context-plugin expectations and corrects the terminal fixture after proving that the unchanged cursor behavior leaves six spaces. lore owns the single affected-case retry and package lane. These attributed fixture defects do not authorize changing concurrent context/selector semantics or weakening worker, queue, mailbox, credential or exact-identity guarantees.

The first serial retry passed 20 of 21 cases, including historical projection. A11 then exposed a real older-schema migration defect: the missing-column repair chose version 23 for an actual version-20 database and skipped additive versions 21–23, leaving durable mailbox tables absent. bali corrected the repair start so it cannot exceed the recorded version. lore's affected retry passed all 30 selected cases, including A11, current-stamp repair and v27 preservation/rollback. All initial internal failures now have passing repaired-case evidence; no second full internal suite was run.

External bundled scripts passed 4/4. The first Codex stdio pass returned 12 passes and 10 failures: its fake hook inventory predated the two model-request hooks, and its passive-preview assertion reused a scope already opened by the shared fixture while also injecting a bound parent identity. maho released a test-only correction: advertise all eight hooks and run the unbound human preview against a fresh copy of the exact CLI-created profile configuration, retaining no-database/no-Provider/no-preflight and unchanged-config assertions. All ten failed cases passed their targeted retry, giving 22/22 combined stdio passes; previously passed external cases were retained.

## Candidate and follow-up boundary

The initial package completed in 893.85 seconds: [candidate](../../.bin/agentx-metadata), version `0.7.31`, 32,721,368 bytes, SHA256 `f3237d940e692408f37b848fb9b16792ea4ad02d901802ef4420dcdc6d6082a3`. Its evidence root retains `candidate-artifact.json` and `candidate-source-inputs-r2.json`; all 1,239 input hashes were rechecked with no drift. That immutable artifact passed CLI and native management/browser acceptance but was rejected for selector adoption; it was never adopted.

lore released source and Cargo ownership after the initial artifact freeze. tara implemented the separately user-authorized attach/kill/fork/resume selector extension under its [own Task](../cli-selection.task.md), with separate post-change checks/artifact. That later source is not part of the initial candidate or its acceptance claim. Both artifacts and their frozen acceptance inputs remain preserved.

Initial launch-selector PTY acceptance exposed a real stdout-isolation defect: `Terminal::clear()` queries cursor position through Crossterm, which sends DSR to stdout even though the picker backend writes stderr. The runner-only PTY response fix could not repair that behavior. Native source/API diagnosis identified the clear operation rather than the fullscreen constructor as the cause. tara corrected the alternate-screen clear directly through stderr, retaining Ratatui's fullscreen constructor/autoresize and every original output/cancel/restoration assertion.

The accepted corrected [candidate](../../.bin/agentx-selection) is version `0.7.31`, 32,747,704 bytes, SHA256 `b45bafee8e28e00ccb8529ba70fa892c528a0fb9b81477ea1e7bf795d8905fbc`. Package-r2 completed in 956.33 seconds with no drift across 1,241 recorded inputs. Its manifest SHA256 is `40dc581638191e44945f8397b0cac7e699706586727ae5234fadb82920fdf09d`. tara's selected native cases passed 20/20 and exact-release lifecycle-target PTY passed 9/9. lore independently verified original unadapted launch PTY 10/10, CLI shape 11 checks, native management M01–M11 plus transcript and embedded-browser B01–B03 on those exact corrected bytes; both fixture manifests report unchanged candidate bytes and zero cleanup errors. [Final independent result](verification.md#corrected-fixed-artifact-and-final-independent-result) owns receipts, input/package binding and evidence limits.

## Accepted outcome and adoption

The cleanup is complete for the accepted source/artifact scope. tara installed the exact accepted bytes to project [`.bin/agentx`](../../.bin/agentx), version `0.7.31`, with the same SHA256 `b45bafee8e28e00ccb8529ba70fa892c528a0fb9b81477ea1e7bf795d8905fbc`. The rollback copy is `.bin/agentx.before-selection-20261116-200321`, SHA256 `9cdc66606ef4e2880868adcfc5e81638a3ab0ca54ef010e6f7ee804e5c8b2cf0`. `/tmp/agentx-target-selection-rc_xota2/adoption.json` records exact install/version/help checks and no service restart; maho independently matched installed and rollback file hashes to that receipt. Current running daemons, PTY hosts, Relay processes and globally installed executables were not replaced or restarted.

Native upgrade/preservation, affected workflows/stdio, compiled Web consumers, CLI, generated-hook HTTP and finite browser/terminal acceptance are verified within the documented Linux local scope. Paid real Providers, macOS/Windows and a full external Relay roundtrip remain unverified. All 19 ignored full-suite cases remain ignored; this record does not imply they ran or that a second full suite was executed.

The source/native lane has transferred to the separately authorized labels Delivery after the corrected artifact/input freeze. Subsequent classification design or implementation does not change this cleanup's fixed-artifact claim. This Task contains no labels feature implementation; follow the separately owned [classification document](../../.docs/agent-classification.md) for that later work.

## Coordination feedback

The older maho-owned idle reaction `sub-b7b7d0ab` broadcast a phase notice to unrelated and closed teams (event 41148). That notice grants no assignment or verification transfer. lore remains the sole verifier/package owner; source repairs and adoption keep the explicit ownership above. The one-shot reaction is consumed and no longer appears in active subscriptions. Use directed release messages to avoid restarting historical work or duplicating checks.


## Home-scope recovery feedback (2026-11-17)

A user-reported failure of installed `agentx 0.7.31` (cohort `native-build-v1:95d9a9e7-6de0-40f2-95e3-6f9a29f827c7`) on `/home/long/.agentx` exposed a stamped-v26 database missing the earlier v21 shared-runtime, v22 message-acceptance and v23 collaboration tables. All additive instance columns were present, so `repair_migrate_from` selected v26; `project_instance_columns` then failed with `no such table: participant_sessions`. The daemon initialization error projection omitted this underlying SQLite cause, leaving only `In-place schema upgrade failed; existing database preserved` and the client's readiness deadline. The origin of the inconsistent stamp is unproven.

Recovered the selected home scope after a SQLite online backup and successful disposable-copy validation: applied only the idempotent native `shared_runtime::SCHEMA`, `message_acceptance::SCHEMA`, `collaboration::SCHEMA` and `launch_relations::SCHEMA`, without changing the version stamp, then let the exact installed binary perform its normal v26-to-v27 migration. Startup/owner locks excluded concurrent daemon acquisition during the actual additive repair. Candidate integrity check passed; 49 instance rows, 222890 events and the database generation were retained across candidate upgrade. Actual additive repair likewise retained version, row counts, event maximum and generation. Home daemon restart succeeded; `agentx l` from the home directory with explicit home scope returned its successful launch preview without starting a Provider. Other live scopes were untouched.

Machine-local backup, exact additive SQL and before/after receipt: `/home/long/.local/state/agentx/recovery/20261117-043759/` (`before-repair.db`, `repair.sql`, `receipt.json`). Source follow-up remains open: validate required additive tables when choosing migration repair, and retain a useful safe underlying initialization cause in startup diagnostics. This operational recovery does not claim those source defects are fixed or authorize changing the active labels/source-build lane.


## Authorized root-cause correction (2026-11-17)

The user explicitly requested fixing the cause after home-scope recovery. Work inline: bounded persistence/startup correction, no new team or release workflow. Source ownership notice sent to nezi; no Cargo process was active at lane inspection.

Causal model: schema version is a claim about committed structure, not proof. Compatibility checks omit v21-v23 table groups, repair selection considers instance columns but not additive tables, and missing newer tables at a current stamp can take the destructive archive route. `init_db` also accepts existing versioned older schemas and can stamp current without applying their column migrations; this is a supported mechanism for inconsistent stamps, not proof of the exact historical producer of the user's v26 file. Startup converts `anyhow::Error` with `to_string()`, discarding its SQLite cause.

Selected correction: one versioned additive-table inventory shared by compatibility detection and earliest-repair selection; choose the earliest missing column/table/projection prerequisite, replay existing canonical migrations transactionally, and preserve existing databases on migration failure. Restrict bootstrap to fresh/unversioned initialization or an already-current no-op so it cannot promote/downgrade a versioned schema. Preserve the full error chain at the daemon's local bootstrap adapter. Do not introduce a new migration framework or alter live databases/binaries.

Cases: old v26 and current-stamped databases missing v21-v23 tables must repair without inode/generation/event/instance loss; each additive table must trigger repair; missing tables plus later missing instance column must select the earlier repair; dependent-index failure after additive table repair must roll back table creation, columns and version without archive; newer schemas remain untouched; direct bootstrap must reject versioned promotion/demotion. Existing migration/projection and initialization concurrency cases cover adjacent guarantees.


Historical producer mechanism is now identified in committed source `9420211` (schema v26): `repair_migrate_from(20)` returns 23 when `launch_profile` is absent, skipping migrations 21, 22 and 23, then running 24–26 and committing a v26 stamp. A bounded SQLite reproduction using the actual v20 fixture and canonical v24–v26 SQL yields exactly the six missing intermediate tables found in the home backup; receipt: `/home/long/.local/state/agentx/recovery/20261117-043759/historical-cause.json`. The earlier `version.min(migration - 1)` fix prevented recurrence on true old-version upgrades but did not recognize existing incorrectly stamped databases. This is stronger causal evidence than the bootstrap hypothesis; no retained execution log identifies the exact historical invocation that wrote this user's file. Bootstrap stamping protection is defense against a separately confirmed unsafe entry path.


Implementation refinements: after replay, verify the required additive tables inside the same transaction before committing the stamp. SQLite silently treats an existing same-name view as satisfying `CREATE TABLE IF NOT EXISTS`; the collision regression therefore verifies failed postconditions preserve the database. The first focused native compile found one new test API mistake (corrected) and two existing labels compile failures: `LabelsSupport` needed `&client` in `runtime/launcher.rs`, and the query-budget assertion moved a SQLite error in a pattern guard. Applied those minimal compile corrections, preserving the original query-budget assertion, and notified nezi. No active competing Cargo process was observed.


First native database execution: 151 tests, 147 passed, 4 failed. Passing coverage included home-shaped recovery, all ten individually missing additive tables, projection rollback after additive repair, bootstrap version guards, existing projection preservation and labels migrations. The table/view postcondition regression ran against the earlier compiled source and failed as expected before the newly added postcondition was compiled. The column-guard regression exposed diagnostic/legacy routing precedence: check unsupported base-column absence before additive table absence, retaining archive only where no known migration introduces that base column. Two v16/v17 fixtures omitted the pre-existing `origin_device_id` field needed by the v28 workspace index; restored that field in the synthetic fixtures without weakening preset/context/cursor preservation assertions. Rerun is restricted to the affected schema test family and directly affected labels migration checks.


Final root-cause correction status: source complete and verified within Linux local scope. `just test --bin agentx db::tests:: -- --test-threads=1` passed 34/34 (includes one `tui::db` substring match). The exact resulting harness `target/debug/deps/agentx-aa1d376cc9afeff5` then passed `db::labels::tests::` 2/2 and `labels_selection_before_page_missing_negative_cursor_and_budget` 1/1. The first 151-case database run remains historical; its four failed guarantees all passed the corrected focused run. No second broad database/full suite was run. Scoped rustfmt and `git diff --check` passed. Existing labels unused-import/progress-handler-result warnings remain outside this correction.

Updated `src/db/mod.rs`, startup error conversion in `src/main.rs`, and `src/db/AGENTS.md`; the two minimal labels compilation repairs are identified above. No release packaging, installed/default binary replacement, home-scope re-migration or live daemon restart occurred during this root-cause source correction. The preceding home recovery remains operational; this source fix must be included in the next accepted package to reach other scopes through their installed executable.
