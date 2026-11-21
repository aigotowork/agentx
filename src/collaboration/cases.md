# Native work delivery cases

All cases are authored, unexecuted by this source owner. The [independent Task](../../.controller/collaboration-workbench/tasks/acceptance.md) owns execution/evidence and the real Provider/browser outcome.

| Boundary / case | Authored source / expected fact |
| --- | --- |
| Failed check → changes → same-result recheck → new result → passing exact acceptance | `tests.rs::failed_check_changes_new_result_pass_and_exact_acceptance`; prior evidence remains and accepted work closes every mutation. |
| Same-result active recheck, open question and new brief | `tests.rs::same_result_recheck_open_question_and_brief_change_remove_eligibility`; each independently removes current acceptance and stale confirmation fails. |
| Exact checker, active request, source match, criterion coverage | `tests.rs::checker_requires_exact_actor_current_request_matching_source_and_full_coverage`; no stale incarnation or incomplete passing report. |
| Role replacement / question amendment | `tests.rs::replacement_preserves_question_author_and_amendment_targets_current_assignment`; historical author remains, current replacement receives answer, old assignment report fails. |
| Immutable artifact identity and traversal | `tests.rs::artifact_paths_and_mutable_git_branches_cannot_escape_reported_identity`; full Git IDs, bounded relative paths, no file reading. |
| Durable deduplication / conflicts | `tests.rs::durable_replay_precedes_revision_and_rejects_changed_intent`; same event/receipt on replay, changed intent rejected. |
| Atomic native message + aggregate failure | `tests.rs::failed_aggregate_write_rolls_back_message_acceptance_receipt_and_revision`; injected aggregate persistence failure rolls back native message event/acceptance and command receipt. |
| Native scope/project/incarnation/generation/capacity | `tests.rs::exact_incarnation_generation_project_and_capacity_are_native_guards`; UI omission cannot bypass guards and capacity cannot leave partial dispatch. |
| Dataset reset | `tests.rs::managed_reset_removes_work_receipts_and_changes_generation_fence`; collaboration rows and receipts are part of managed reset. |
| Native report actor / finite real work conversation | `tests.rs::native_report_resolves_exact_actor_and_thread_read_has_no_cross_work_leak`; missing/wrong actors cannot report, exact actor succeeds, and paged ordinary replies exclude another work. |
| Browser closure and admission | `web/tests.rs`; no actor report/generic request/project selector, raw numeric or duplicate-key tunnel; readonly and input denials retain no-store/CSP and public CORS; foreign origins reach input validation; readonly OPTIONS returns 204 without native work. |
| Explicit caller option | `web/tests.rs::launch_accepts_explicit_name_without_a_browser_actor_claim`; workbench accepts `--iam` as a shared invocation option while browser invocation remains actor-cleared. |
| Shared browser mechanics | Existing `../observability/web/tests.rs` consumes shared numeric admission, integer codec and manifest loader; diagnostic policy/route behavior remains owned there. |

Independent outcome must additionally exercise actual `workbench` and `work report` CLI parsing/identity, real AgentX participants and Provider effects, HTTP interruption/command lookup, page reload/history, actual desktop/narrow interaction, fresh compiled assets and an unrelated-directory standalone binary. Source-level cases are not evidence those outcomes occurred.

The LAN client case also requires command ID creation on an ordinary numeric-LAN HTTP origin where `crypto.randomUUID` is absent. The finite client uses `crypto.getRandomValues` directly with UUIDv4 bits. This substantive issue was identified by the independent owner while consuming source and corrected before execution release; native generation/intent identity semantics remain unchanged.
