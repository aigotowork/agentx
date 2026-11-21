#!/usr/bin/env python3
"""Inventory production SQLite boundaries; source evidence is not process proof.

See .docs/architecture/acceptance.md DD-11. This conservative Rust source scan
excludes cfg(test) items and their external modules, preserves line numbers, and
reports query/connection candidates for independent inspection. It does not
resolve aliases, macros or arbitrary cfg expressions like a Rust compiler.
"""

import argparse
import json
from pathlib import Path
import re


# Source-linked permissions: .docs/model/storage-boundary.md. An explicit
# adapter home grants borrowed work only; no suffix or client directory rule.
DAEMON_ADAPTERS = {
    "src/management/activity.rs": "Management OfficeActivity -> scoped addressed event and human mailbox observations",
    "src/inspection/store.rs": "SqliteApplication -> Inspection handle -> generation-scoped ledger, deterministic jobs and atomic episode continuation",
    "src/management/human.rs": "Management HumanMailbox/HandleHumanMessages -> scoped mailbox projections and transactional explicit handling",
    "src/management/relations.rs": "Management LaunchRelations -> indexed exact creation projections",
    "src/agents/launch_relations.rs": "Launch/Lifecycle/AttachmentStore -> immutable creation facts and exact resume aliases",
    "src/management/store.rs": "SqliteApplication -> Management handle -> exact projections and durable operation receipts",
    "src/service/application.rs": "SqliteApplication -> domain handle",
    "src/collaboration/store.rs": "SqliteApplication -> collaboration::service::handle re-export -> store",
    "src/service/listing_store.rs": "queries_store -> listing load",
    "src/service/queries_store.rs": "Application -> Queries handle",
    "src/service/maintenance_store.rs": "Application -> Maintenance handle",
    "src/events/store.rs": "Application -> Events handle",
    "src/events/listening_store.rs": "Events handle -> Begin/Pulse/EndListen",
    "src/events/history_store.rs": "Events Query -> scoped retained archive/history window",
    "src/events/observer_store.rs": "Events Begin/Check/EndObserver; db postcommit -> exact observer wake",
    "src/agents/service_store.rs": "Application -> Participants handle",
    "src/agents/attachments_store.rs": "Participants -> AttachmentStore",
    "src/agents/invocation_binding.rs": "Participants -> explicit invocation binding",
    "src/agents/lifecycle_store.rs": "Participants -> Lifecycle handle",
    "src/agents/lifecycle_resume_store.rs": "Lifecycle -> Resume selection/reservation",
    "src/agents/launch_store.rs": "Participants -> Launch handle",
    "src/agents/context_store.rs": "Participants Lifecycle Context -> frozen instructions and initial-context receipts",
    "src/agents/hook_store.rs": "Participants -> HookSession/StopSession",
    "src/agents/model_request_store.rs": "Participants -> fenced ObserveModelRequest",
    "src/agents/cleanup_store.rs": "native adapters -> SystemCleanup",
    "src/agents/hermes_store.rs": "Participants -> Hermes handle",
    "src/agents/copilot_store.rs": "Participants -> Copilot handle; Hosts -> validate_endpoint",
    "src/agents/claude_store.rs": "Participants -> Claude handle",
    "src/agents/claude_store/identity.rs": "Claude handle -> exact root/child resolution",
    "src/agents/claude_store/output.rs": "Claude handle -> frozen output/confirmation",
    "src/agents/claude_store/stop.rs": "Claude handle -> exact stop/activation",
    "src/messaging/service_store.rs": "Application -> Messages handle",
    "src/messaging/bundle_store.rs": "Messages -> Bundle handle",
    "src/messaging/inbox_store.rs": "Messages -> PendingInbox",
    "src/messaging/delivery_store.rs": "Messages -> Delivery handle",
    "src/runtime/hosts_store.rs": "Application -> Hosts handle",
    "src/runtime/process_host_store.rs": "Hosts -> ProcessHost handle",
    "src/relay/service_store.rs": "Application -> Relay handle",
    "src/relay/control_store.rs": "Relay handle -> control acceptance",
    "src/transcript/store.rs": "Queries -> TranscriptSource/Inventory/Correlations",
}
BORROW_KINDS = {"managed_reference", "connection_access", "sqlite_dependency", "sql_candidate"}
# Exact top-level lexical homes. Nested named host functions do not inherit a
# caller's rule. Imports are dependency evidence, not constructor permission.
HELPERS = {
    "src/inspection/recovery.rs": ("Inspection store -> exact current-instance episode and stored status eligibility", {"eligible"}),
    "src/agents/workspaces.rs": ("Launch reserve/endpoint operations and invocation binding -> fenced workspace endpoint policy", {
        "endpoint_binding", "retired", "require_write_scope", "endpoint_inspect", "endpoint_lookup", "bind_endpoint", "is_bound_endpoint", "protect_replacement", "migrate_endpoint", "endpoint_reset"}),
    "src/agents/binding.rs": ("invocation_binding/lifecycle/launch/Hermes -> explicit binding", {
        "explicit", "try_persist_terminal_launch_context", "store_launch_context", "migrate_placeholder_notify", "delete_true_placeholder_instance", "migrate_placeholder_runtime_state", "delete_true_placeholder_if_migrated", "retire_true_placeholder_after_canonical_bind", "retire_switched_identity", "recreate_instance_from_placeholder", "bind_session_with_context", "bind_session_to_process_inner", "bind_session_for_tool_with_context", "bind_session_for_tool_inner", "recover_process_binding_for_instance", "recover_process_binding_inner", "initialize_instance_with_context", "initialize_instance_inner", "log_created_and_auto_subscribe", "create_orphaned_pty_identity_with_context", "create_orphaned_pty_identity_inner", "auto_subscribe_defaults"}),
    "src/agents/identity.rs": ("native participant/message/read handlers -> identity/history", {
        "resolve_identity", "resolve_identity_with_expectation", "resolve_from_name", "last_stopped", "describe_missing_agent", "known_agent_names"}),
    "src/agents/names.rs": ("native participant/lifecycle/launch/Hermes -> name allocation", {"allocate_unreserved_name", "collect_taken_names"}),
    "src/agents/lifecycle.rs": ("native stores -> stored-only lifecycle/status", {
        "try_set_status", "set_status", "get_instance_status_stored", "try_finalize_launch_failure_detail_stored", "get_launch_blocker_detail_stored"}),
    "src/messaging/submission.rs": ("Messages handle -> routing/intent/persistence", {
        "deliverable_instances", "resolve_recipients", "try_resolve_reply_to_local", "try_get_thread_from_event", "get_intent_from_event", "resolve_delivery", "persist_message"}),
    "src/runtime/process_tracking.rs": ("Queries/Lifecycle -> supplied orphan facts", {
        "owning_instance", "adopt_orphan", "attach_runtime_state", "recover_single_orphan_to_db"}),
    "src/runtime/launch_status.rs": ("Hosts ObserveLaunch -> stored-only diagnosis", {
        "observe_launch", "diagnostic_records", "get_ready_for_batch", "get_blocked_for_batch", "launch_failed_events_for_batch", "stopped_detail_for_instance", "get_batch_instance_names", "get_launch_status", "get_launch_batch"}),
    "src/integrations/hooks/claude/native.rs": ("Hosts -> Claude mod finite handler/health", {
        "handle_host", "report_health", "state", "record_report", "load", "finish", "wake_due", "validated_owner"}),
    "src/runtime/notify/wake.rs": ("native Relay/domain postcommit -> endpoint snapshots", {
        "snapshot_all_wake_ports", "snapshot_wake_ports", "lookup_ports", "wake_launch_waiters"}),
    "src/relay/pull.rs": ("Relay Receive -> authenticated import", {
        "handle_control_message_with", "handle_state_message_with", "handle_device_gone_with", "import_remote_events", "insert_remote_event", "finish_imported_events"}),
    "src/relay/backfill.rs": ("Relay Backfill -> import/correlation/gaps", {
        "tick", "already_imported", "devices_with_gaps", "save_gaps_checked", "load_gaps", "save_gaps", "handle_backfill_request", "handle_backfill_response"}),
    "src/relay/push.rs": ("Relay Export/control -> state projection", {"build_state", "build_push_payload", "sealed_payload_budget"}),
    "src/relay/control.rs": ("Relay native import/backfill -> exact response", {"read_remote_capabilities", "take_rpc_result_from"}),
    "src/relay/mod.rs": ("Relay native handler -> scoped durable state", {
        "device_short_id_for_db", "remember_device_short_id", "safe_kv_get", "safe_kv_set", "worker_heartbeat_age", "clear_relay_device_state", "clear_runtime_relay_kv", "set_relay_status", "is_relay_handled_by_daemon"}),
}
# Additional inspected mixed-helper edges grant only their observed access
# kinds. They never inherit a module's wider connection/SQL permission.
EXACT_PERMISSIONS = {
    ("src/runtime/process_tracking.rs", "try_owning_instance"): (
        {"sqlite_dependency", "sql_candidate"}, "daemon_borrow", "Queries/adoption -> required borrowed process/session owner read"),
    ("src/runtime/process_tracking.rs", "try_adopt_orphans"): (
        {"managed_reference", "connection_access"}, "daemon_borrow", "Lifecycle ClaimOrphans/Start -> supplied-fact atomic adoption"),
    ("src/runtime/process_tracking.rs", "try_claim_orphans_deferred"): (
        {"managed_reference"}, "daemon_borrow", "Lifecycle Start -> fallible explicit-scope deferred adoption"),
    ("src/integrations/hooks/claude/actor.rs", "<module>"): (
        {"managed_reference"}, "daemon_borrow", "Lifecycle Start -> explicit-invocation actor dependency"),
    ("src/integrations/hooks/claude/actor.rs", "resolve_actor_from_env"): (
        {"managed_reference"}, "daemon_borrow", "Lifecycle start_effects -> originating invocation capability resolution"),
    ("src/integrations/hooks/claude/actor.rs", "ensure_explicit_matches"): (
        {"managed_reference"}, "daemon_borrow", "Lifecycle start_effects -> exact actor/name consistency"),
    ("src/agents/identity.rs", "instance_not_found_error_for"): (
        {"managed_reference"}, "daemon_borrow", "Participants resolve_actor -> resolve_from_name -> missing identity detail"),
    ("src/agents/names.rs", "describe_generated_names"): (
        {"managed_reference"}, "daemon_borrow", "Hosts BootstrapSnapshot -> stored name-shape projection"),
    ("src/agents/names.rs", "reserve_generated_name_with_timeout"): (
        {"managed_reference"}, "daemon_borrow", "Lifecycle reserve_fork -> allocation with originating timeout"),
    ("src/agents/lifecycle.rs", "is_in_wake_grace_shared"): (
        {"managed_reference"}, "daemon_borrow", "SystemCleanup Listing -> shared wake policy"),
    ("src/agents/lifecycle.rs", "wake_grace"): (
        {"managed_reference"}, "daemon_borrow", "SystemCleanup -> shared stored wake-grace state"),
    ("src/agents/lifecycle.rs", "instance_status"): (
        {"managed_reference"}, "daemon_borrow", "Native get_instance_status_stored -> stored status projection"),
    ("src/agents/lifecycle.rs", "try_finalize_launch_failure"): (
        {"managed_reference"}, "daemon_borrow", "Stored launch finalizer -> exact required row/status transaction"),
    ("src/messaging/submission.rs", "resolve_reply_to_local"): (
        {"managed_reference"}, "daemon_borrow", "Messages Submit -> persist_message -> local reply correlation"),
    ("src/runtime/notify/wake.rs", "wake"): (
        {"managed_reference"}, "daemon_borrow", "db post_commit -> named domain endpoint wake"),
    ("src/events/filters.rs", "build_sql_from_flags"): (
        {"sql_candidate"}, "pure_policy", "Explicit event filters -> constrained expression string"),
    ("src/events/predicate.rs", "<module>"): (
        {"sqlite_dependency"}, "daemon_borrow", "Events/Participants -> scoped authorizer read; no constructor"),
    ("src/events/predicate.rs", "read"): (
        {"sqlite_dependency"}, "daemon_borrow", "Native projection -> borrowed authorizer; exact Reset drop"),
    ("src/events/predicate.rs", "read::drop"): (
        {"sqlite_dependency"}, "daemon_borrow", "Authorizer Reset -> clear borrowed connection authorizer"),
    ("src/runtime/bootstrap.rs", "<module>"): (
        {"managed_reference"}, "daemon_borrow", "Hosts BootstrapSnapshot -> borrowed projection dependency"),
    ("src/runtime/bootstrap.rs", "snapshot"): (
        {"managed_reference"}, "daemon_borrow", "Hosts BootstrapSnapshot -> stored participant/name/roster"),
    ("src/runtime/bootstrap.rs", "get_active_instances"): (
        {"managed_reference"}, "daemon_borrow", "BootstrapSnapshot -> active roster projection"),
    ("src/runtime/launch_status.rs", "get_failed_for_batch"): (
        {"managed_reference"}, "daemon_borrow", "Hosts ObserveLaunch -> stored-only batch failure diagnosis"),
    ("src/runtime/launch_status.rs", "emit_row_finalized_event"): (
        {"managed_reference"}, "daemon_borrow", "ObserveLaunch stored diagnosis -> required finalized event"),
    ("src/runtime/launch_status.rs", "get_batch_failure_details_for_ids"): (
        {"managed_reference"}, "daemon_borrow", "ObserveLaunch timeout -> stored batch failure/blocker projection"),
    ("src/agents/names.rs", "allocate_subagent_instance_in_directory"): (
        {"managed_reference", "connection_access", "sqlite_dependency", "sql_candidate"}, "daemon_borrow", "Claude identity -> child allocator with explicit cwd"),
    ("src/relay/backfill.rs", "clear_gaps"): (
        {"managed_reference"}, "daemon_borrow", "Receive reset/import or Backfill tick -> clear gaps"),
    ("src/relay/backfill.rs", "record_gap"): (
        {"managed_reference"}, "daemon_borrow", "Receive import_remote_events -> detected skipped range"),
    ("src/relay/backfill.rs", "local_reset_ts"): (
        {"managed_reference"}, "daemon_borrow", "Backfill apply_answer -> local reset cutoff"),
    ("src/relay/backfill.rs", "apply_answer"): (
        {"managed_reference"}, "daemon_borrow", "Backfill tick -> exact stored RPC answer import/dedup"),
    ("src/relay/control.rs", "peer_accepts_action"): (
        {"managed_reference"}, "daemon_borrow", "Backfill tick -> stored peer capability read"),
    ("src/relay/pull.rs", "state_ts_watermark"): (
        {"managed_reference"}, "daemon_borrow", "Receive handle_state_message_with -> stored state watermark"),
    ("src/relay/pull.rs", "record_state_ts_watermark"): (
        {"managed_reference"}, "daemon_borrow", "Receive handle_state_message_with -> accepted watermark"),
    ("src/relay/pull.rs", "cached_reset_ts"): (
        {"managed_reference"}, "daemon_borrow", "Receive handle_state_message_with -> reset cutoff"),
    ("src/relay/pull.rs", "event_cursor"): (
        {"managed_reference"}, "daemon_borrow", "Receive import/ends_below_cursor -> durable event cursor"),
    ("src/relay/pull.rs", "ends_below_cursor"): (
        {"managed_reference"}, "daemon_borrow", "Receive handle_state_message_with -> duplicate boundary"),
    ("src/relay/pull.rs", "resolve_short_id"): (
        {"managed_reference"}, "daemon_borrow", "Receive authenticated departure -> stored short ID"),
    ("src/relay/pull.rs", "emit_device_event"): (
        {"managed_reference"}, "daemon_borrow", "Receive join/reconnect/departure -> native device event"),
    ("src/relay/push.rs", "with_detail_previews"): (
        {"managed_reference"}, "daemon_borrow", "Export/control stored build_state -> bounded detail previews"),
}
PROVIDER_FUNCTIONS = {
    "src/transcript/hermes.rs": {"open", "content_column", "parse_hermes_sqlite_with_usage", "search_hermes_sessions"},
    "src/transcript/opencode.rs": {"family_text_rows", "search_family_sessions", "table_exists", "load_v1_messages", "load_v2_messages", "parse_opencode_sqlite_with_usage"},
    "src/runtime/resume_client.rs": {"lookup_opencode_family_session"},
}
ARCHIVE_READERS = {("src/service/maintenance_store.rs", "open_archive"),
                   ("src/events/history_store.rs", "capture"), ("src/events/history_store.rs", "query")}
PATTERNS = {
    "managed_constructor": r"\bAgentxDb\s*::\s*open(?:_at|_raw)?\b",
    "managed_reference": r"\bAgentxDb\b(?!\s*::\s*open)",
    "sqlite_constructor": r"\bConnection\s*::\s*open(?:_with_flags|_in_memory)?\b",
    "connection_access": r"\.\s*conn\s*\(",
    "sqlite_dependency": r"\brusqlite\b",
}
SQL = re.compile(r"\b(?:SELECT\s|INSERT\s+(?:OR\s+\w+\s+)?INTO\s|UPDATE\s+\w+\s+SET\s|DELETE\s+FROM\s|PRAGMA\s)")


def blank(text):
    return "".join("\n" if char == "\n" else " " for char in text)


def mask_literals(source, keep_strings=False):
    """Keep Rust syntax positions, masking comments/strings/chars, not lifetimes."""
    chars = list(source)
    pos = 0
    while pos < len(source):
        end = pos
        string = False
        if source.startswith("//", pos):
            end = source.find("\n", pos)
            end = len(source) if end < 0 else end
        elif source.startswith("/*", pos):
            end, depth = pos + 2, 1
            while end < len(source) and depth:
                if source.startswith("/*", end):
                    depth += 1
                    end += 2
                elif source.startswith("*/", end):
                    depth -= 1
                    end += 2
                else:
                    end += 1
        else:
            raw = re.match(r'(?:br|cr|r)(#{0,255})"', source[pos:]) if source[pos] in "rbc" else None
            if raw and (pos == 0 or not (source[pos - 1].isalnum() or source[pos - 1] == "_")):
                string = True
                closing = '"' + raw[1]
                close = source.find(closing, pos + len(raw[0]))
                end = len(source) if close < 0 else close + len(closing)
            elif source[pos] == '"':
                string = True
                end = pos + 1
                while end < len(source):
                    if source[end] == "\\":
                        end += 2
                    elif source[end] == '"':
                        end += 1
                        break
                    else:
                        end += 1
            elif source[pos] == "'":
                char = re.match(r"'(?:\\(?:u\{[0-9a-fA-F_]+\}|x[0-9a-fA-F]{2}|.)|[^'\\\n])'", source[pos:])
                if char:
                    end = pos + len(char[0])
        if end > pos:
            if not (string and keep_strings):
                chars[pos:end] = blank(source[pos:end])
            pos = end
        else:
            pos += 1
    return "".join(chars)


def item_end(code, start):
    field = re.match(r"\s*(?:pub(?:\([^)]*\))?\s+)?\w+\s*:", code[start:])
    if field:
        depth = 0
        for pos in range(start + field.end(), len(code)):
            char = code[pos]
            if char == "," and depth == 0:
                return pos + 1
            if char in "<([{" and not (char == "<" and pos and code[pos - 1] == "-"):
                depth += 1
            elif char in ">)]}" and not (char == ">" and pos and code[pos - 1] == "-"):
                depth -= 1
        raise ValueError("cfg(test) field has no terminating comma")
    marker = re.search(r"[;{]", code[start:])
    if not marker:
        raise ValueError("cfg(test) item has no body or semicolon")
    pos = start + marker.start()
    if code[pos] == ";":
        return pos + 1
    depth = 1
    end = pos + 1
    while end < len(code) and depth:
        depth += (code[end] == "{") - (code[end] == "}")
        end += 1
    if depth:
        raise ValueError("unbalanced cfg(test) item")
    return end


def production_source(source):
    """Return syntax/text without test items and test-only external mod bindings."""
    code = mask_literals(source)
    syntax, text = list(code), list(mask_literals(source, keep_strings=True))
    external = []
    attribute = re.compile(r"#\s*\[\s*cfg\s*\(([^\]]+)\)\s*\]")
    excluded_end = 0
    for match in attribute.finditer(code):
        condition = match[1]
        if match.start() < excluded_end or not re.search(r"\btest\b", condition) or re.search(r"\bnot\s*\([^)]*\btest\b", condition):
            continue
        end = item_end(code, match.end())
        declaration = re.search(r"\bmod\s+(\w+)\s*;", code[match.end():end])
        if declaration:
            path = re.search(r'#\s*\[\s*path\s*=\s*"([^"]+)"\s*\]', source[match.end():end])
            external.append((declaration[1], path[1] if path else None))
        syntax[match.start():end] = blank(code[match.start():end])
        text[match.start():end] = blank(source[match.start():end])
        excluded_end = end
    return "".join(syntax), "".join(text), external


def function_spans(code):
    """Lexical function homes for auditable, function-scoped policy decisions.

    This deliberately does not infer call reachability. The architecture's
    storage-boundary model owns daemon caller closure; unknown homes stay
    visible as module scope instead of receiving a directory-wide exception.
    """
    return [(match.start(), item_end(code, match.start()), match[1])
            for match in re.finditer(r"\bfn\s+(\w+)\s*(?:<|\()", code)]


def function_at(spans, offset):
    homes = [name for start, end, name in spans if start <= offset < end]
    return "::".join(homes) if homes else "<module>"


def call_expression(code, offset):
    opening = code.find("(", offset)
    if opening < 0:
        return ""
    depth = 1
    end = opening + 1
    while end < len(code) and depth:
        depth += (code[end] == "(") - (code[end] == ")")
        end += 1
    return code[offset:end]


def daemon_initializer(code, offset, home):
    if home != "main":
        return False
    pattern = r"daemon\s*::\s*serve\s*\(\s*scope\s*,\s*move\s*\|\s*generation\s*\|\s*\{"
    for match in re.finditer(pattern, code):
        end = item_end(code, match.end() - 1)
        if match.end() <= offset < end:
            closure = code[match.end():end]
            return bool(re.search(r"SqliteApplication\s*::\s*new\s*\(\s*db\s*,\s*generation", closure)
                        and re.fullmatch(r"AgentxDb\s*::\s*open_at\s*\(\s*&\s*database_path\s*\)", call_expression(code, offset)))
    return False


def classify(file, home, kind, code, offset):
    """Return explicit kind permission and inspected consumer edge, not reachability proof."""
    if file.startswith("src/db/"):
        return "persistence", "Persistence constructor/migration implementation", True
    if kind == "managed_constructor":
        if file == "src/main.rs" and daemon_initializer(code, offset, home):
            return "daemon_initializer", "main -> daemon::serve -> SqliteApplication", True
        return "unclassified", "No production managed opener is authorized here", False
    if kind == "sqlite_constructor":
        expression = call_expression(code, offset)
        readonly = ("open_with_flags" in expression and "SQLITE_OPEN_READ_ONLY" in expression
                    and not re.search(r"SQLITE_OPEN_(?:READ_WRITE|CREATE)\b", expression))
        if (file, home) in ARCHIVE_READERS and readonly:
            return "archive_read", "Native scoped archive reader -> read-only archive", True
        if home in PROVIDER_FUNCTIONS.get(file, set()) and readonly:
            return "provider", "Explicit external Provider read-only adapter", True
        return "unclassified", "Raw SQLite constructors need an exact read-only archive/Provider site", False
    if kind in {"sqlite_dependency", "sql_candidate"}:
        if home in PROVIDER_FUNCTIONS.get(file, set()) or (home == "<module>" and kind == "sqlite_dependency" and file in PROVIDER_FUNCTIONS):
            return "provider", "External Provider adapter; managed storage is excluded", True
    exact = EXACT_PERMISSIONS.get((file, home))
    if exact and kind in exact[0]:
        return exact[1], exact[2], True
    if file in DAEMON_ADAPTERS and kind in BORROW_KINDS and "::" not in home:
        return "daemon_borrow", DAEMON_ADAPTERS[file], True
    if file in HELPERS and kind in BORROW_KINDS:
        edge, functions = HELPERS[file]
        if home in functions or (home == "<module>" and kind in {"managed_reference", "sqlite_dependency"}):
            return "daemon_borrow", edge, True
    return "unclassified", "No exact file/function/kind disposition; inspect actual caller", False


def inventory(root):
    sources = {}
    test_files = set()
    for path in sorted((root / "src").rglob("*.rs")):
        source = path.read_text()
        code, text, external = production_source(source)
        sources[path] = (source, code, text)
        base = path.parent if path.stem in {"main", "lib", "mod"} else path.with_suffix("")
        for name, explicit in external:
            candidates = [path.parent / explicit] if explicit else [base / (name + ".rs"), base / name / "mod.rs"]
            for candidate in candidates:
                test_files.add(candidate.resolve())
                nested = candidate.parent if candidate.stem == "mod" else candidate.with_suffix("")
                if nested.is_dir():
                    test_files.update(child.resolve() for child in nested.rglob("*.rs"))
    findings = []
    for path, (source, code, text) in sources.items():
        if path.resolve() in test_files:
            continue
        relative = path.relative_to(root).as_posix()
        functions = function_spans(code)
        for kind, pattern in PATTERNS.items():
            for match in re.finditer(pattern, code):
                line = source.count("\n", 0, match.start()) + 1
                home = function_at(functions, match.start())
                boundary, edge, allowed = classify(relative, home, kind, code, match.start())
                findings.append({"file": relative, "function": home, "line": line, "kind": kind, "boundary": boundary, "edge": edge, "allowed": allowed, "source": source.splitlines()[line - 1].strip()})
        for match in SQL.finditer(text):
            line = source.count("\n", 0, match.start()) + 1
            home = function_at(functions, match.start())
            boundary, edge, allowed = classify(relative, home, "sql_candidate", code, match.start())
            findings.append({"file": relative, "function": home, "line": line, "kind": "sql_candidate", "boundary": boundary, "edge": edge, "allowed": allowed})
    violations = [row for row in findings if not row["allowed"]]
    return {"root": str(root), "policy_source": ".docs/model/storage-boundary.md", "source_files": len(sources), "excluded_external_test_files": sorted(str(path.relative_to(root)) for path in test_files if path.is_file()), "findings": findings, "violations": violations, "limits": "Conservative source policy with exact lexical homes/kinds and source-reviewed consumer edges; no alias/macro/arbitrary-cfg or executable call-graph resolution, transaction/process proof or legacy binary fencing. Unclassified SQL candidates fail for disposition; pure policy strings may require a narrow explicit rule."}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parent.parent)
    parser.add_argument("--inventory", action="store_true", help="Report current migration surface without enforcing cutover")
    parser.add_argument("--output", type=Path, help="Write complete JSON evidence")
    args = parser.parse_args()
    report = inventory(args.root.resolve())
    if args.output:
        args.output.write_text(json.dumps(report, indent=2) + "\n")
    summary = {key: value for key, value in report.items() if key not in {"findings", "violations"}}
    summary.update(findings=len(report["findings"]), violations=len(report["violations"]), affected_files=sorted({row["file"] for row in report["violations"]}))
    print(json.dumps(summary, indent=2))
    return int(bool(report["violations"]) and not args.inventory)


if __name__ == "__main__":
    raise SystemExit(main())
