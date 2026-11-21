"""Discriminating cases for DD-11's test/production source classification."""

import importlib.util
from pathlib import Path
import tempfile
import unittest

SPEC = importlib.util.spec_from_file_location("db_guard", Path(__file__).resolve().parent.parent / "scripts/check-db-ownership.py")
GUARD = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(GUARD)


class DbOwnershipGuard(unittest.TestCase):
    def test_production_after_inline_tests_remains_visible(self):
        source = '''#[cfg(test)] mod tests { fn f() { AgentxDb::open(); } }
fn real() { AgentxDb::open_at(path); }
'''
        syntax, _, _ = GUARD.production_source(source)
        self.assertEqual(syntax.count("AgentxDb"), 1)
        self.assertIn("open_at", syntax)
        self.assertEqual(syntax.count("\n"), source.count("\n"))

    def test_raw_strings_comments_and_lifetimes_do_not_change_scope(self):
        source = '''// AgentxDb::open() { }
fn f<'a>(x: &'a str) { let s = r###"AgentxDb::open() }"###; }
#[cfg(all(test, unix))] mod tests { let c = '}'; /* { /* } */ */ }
#[cfg(not(test))] fn real() { AgentxDb::open(); }
'''
        syntax, _, _ = GUARD.production_source(source)
        self.assertEqual(syntax.count("AgentxDb"), 1)
        self.assertIn("fn real", syntax)

    def test_test_only_fields_do_not_hide_production_impl_or_initializer(self):
        source = '''struct Store {
  #[cfg(test)] callback: Option<Box<dyn FnOnce() + Send>>,
  connection: Connection,
}
impl Store {
 fn new() -> Self { Self { #[cfg(test)] callback: None, connection: Connection::open(path) } }
 fn real() { AgentxDb::open(); }
}
'''
        syntax, _, _ = GUARD.production_source(source)
        self.assertNotIn("callback", syntax)
        self.assertIn("Connection::open", syntax)
        self.assertIn("AgentxDb::open", syntax)

    def test_external_test_file_is_excluded_without_excluding_provider_store(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "src/transcript").mkdir(parents=True)
            (root / "src/main.rs").write_text('#[cfg(test)]\n#[path = "cases.rs"]\nmod cases;\nfn f() { AgentxDb::open(); }')
            (root / "src/cases.rs").write_text("fn fixture() { AgentxDb::open_raw(path); }")
            (root / "src/transcript/opencode.rs").write_text("fn search_family_sessions() { Connection::open_with_flags(provider_path, SQLITE_OPEN_READ_ONLY); }")
            report = GUARD.inventory(root)
            self.assertEqual(len(report["violations"]), 1)
            self.assertEqual(report["violations"][0]["file"], "src/main.rs")
            provider = next(row for row in report["findings"] if row["file"].endswith("opencode.rs"))
            self.assertEqual(provider["boundary"], "provider")
            self.assertEqual(report["excluded_external_test_files"], ["src/cases.rs"])

    def test_function_homes_keep_mixed_host_and_daemon_access_distinct(self):
        source = '''use rusqlite::Connection;
fn daemon_projection(db: &AgentxDb) { db.conn().execute("SELECT id FROM events"); }
#[cfg(test)] mod tests { fn fixture() { AgentxDb::open(); } }
fn host() { fn accidental_fallback() { AgentxDb::open(); } }
'''
        syntax, _, _ = GUARD.production_source(source)
        spans = GUARD.function_spans(syntax)
        self.assertEqual(GUARD.function_at(spans, syntax.index("rusqlite")), "<module>")
        self.assertEqual(GUARD.function_at(spans, syntax.index(".conn")), "daemon_projection")
        self.assertEqual(GUARD.function_at(spans, syntax.index("AgentxDb::open")), "host::accidental_fallback")

    def test_only_exact_daemon_application_closure_can_open_managed_storage(self):
        code = '''fn main() {
 let host = AgentxDb::open_at(&database_path);
 daemon::serve(scope, move |generation| {
   let db = AgentxDb::open_at(&database_path);
   SqliteApplication::new(db, generation)
 });
 fn helper() { AgentxDb::open_at(&database_path); }
}'''
        offsets = [match.start() for match in GUARD.re.finditer("AgentxDb::open_at", code)]
        spans = GUARD.function_spans(code)
        outcomes = [GUARD.classify("src/main.rs", GUARD.function_at(spans, offset), "managed_constructor", code, offset)[2] for offset in offsets]
        self.assertEqual(outcomes, [False, True, False])

    def test_borrowed_adapter_never_grants_open_or_nested_host_permission(self):
        self.assertTrue(GUARD.classify("src/agents/service_store.rs", "handle", "connection_access", "", 0)[2])
        self.assertFalse(GUARD.classify("src/agents/service_store.rs", "handle", "managed_constructor", "", 0)[2])
        self.assertFalse(GUARD.classify("src/agents/service_store.rs", "handle::host", "connection_access", "", 0)[2])
        self.assertFalse(GUARD.classify("src/agents/new_store.rs", "handle", "connection_access", "", 0)[2])
        self.assertFalse(GUARD.classify("src/daemon/new.rs", "handle", "sqlite_dependency", "", 0)[2])

    def test_mixed_module_and_provider_permissions_are_kind_and_function_scoped(self):
        self.assertTrue(GUARD.classify("src/agents/identity.rs", "last_stopped", "sql_candidate", "", 0)[2])
        self.assertFalse(GUARD.classify("src/agents/identity.rs", "host::last_stopped", "sql_candidate", "", 0)[2])
        self.assertFalse(GUARD.classify("src/runtime/launch_status.rs", "wait_for_launch", "managed_reference", "", 0)[2])
        self.assertTrue(GUARD.classify("src/runtime/launch_status.rs", "get_batch_failure_details_for_ids", "managed_reference", "", 0)[2])
        self.assertFalse(GUARD.classify("src/runtime/launch_status.rs", "get_batch_failure_details_for_ids", "connection_access", "", 0)[2])
        self.assertFalse(GUARD.classify("src/runtime/launch_status.rs", "host::get_batch_failure_details_for_ids", "managed_reference", "", 0)[2])
        self.assertFalse(GUARD.classify("src/messaging/bundles.rs", "validate_extends_reference", "sql_candidate", "", 0)[2])
        self.assertFalse(GUARD.classify("src/transcript/opencode.rs", "search_family_sessions", "managed_constructor", "", 0)[2])
        self.assertFalse(GUARD.classify("src/transcript/opencode.rs", "search_family_sessions", "managed_reference", "", 0)[2])

    def test_archive_and_provider_constructors_require_explicit_readonly_flags(self):
        for file, function in [("src/service/maintenance_store.rs", "open_archive"),
                               ("src/events/history_store.rs", "capture"), ("src/events/history_store.rs", "query"),
                               ("src/transcript/hermes.rs", "open")]:
            readonly = "Connection::open_with_flags(path, SQLITE_OPEN_READ_ONLY)"
            writable = "Connection::open_with_flags(path, SQLITE_OPEN_READ_ONLY | SQLITE_OPEN_CREATE)"
            self.assertTrue(GUARD.classify(file, function, "sqlite_constructor", readonly, 0)[2])
            self.assertFalse(GUARD.classify(file, function, "sqlite_constructor", writable, 0)[2])
            self.assertFalse(GUARD.classify(file, function, "sqlite_constructor", "Connection::open(path)", 0)[2])

    def test_history_and_observer_homes_do_not_grant_extra_constructor_rights(self):
        readonly = "Connection::open_with_flags(path, SQLITE_OPEN_READ_ONLY)"
        for file in ["src/events/history_store.rs", "src/events/observer_store.rs"]:
            for kind in GUARD.BORROW_KINDS:
                self.assertTrue(GUARD.classify(file, "query", kind, "", 0)[2])
                self.assertFalse(GUARD.classify(file, "query::host", kind, "", 0)[2])
            self.assertFalse(GUARD.classify(file, "query", "managed_constructor", "", 0)[2])
        for file, home in [("src/events/store.rs", "query"), ("src/events/observer_store.rs", "query"),
                           ("src/events/history_store.rs", "other"), ("src/events/history_store.rs", "query::host")]:
            self.assertFalse(GUARD.classify(file, home, "sqlite_constructor", readonly, 0)[2])

    def test_unknown_sql_and_managed_borrows_fail_without_counting_comments(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "src").mkdir()
            (root / "src/main.rs").write_text('''// SELECT id FROM events
fn bad(db: &AgentxDb) { let sql = "SELECT id FROM events"; }
''')
            findings = GUARD.inventory(root)
            self.assertEqual({row["kind"] for row in findings["violations"]}, {"managed_reference", "sql_candidate"})
            self.assertEqual(len(findings["violations"]), 2)
            self.assertTrue(all(row["function"] == "bad" and row["edge"] for row in findings["violations"]))

    def test_named_replication_edge_does_not_grant_sql_connection_or_nested_access(self):
        file = "src/relay/pull.rs"
        self.assertTrue(GUARD.classify(file, "event_cursor", "managed_reference", "", 0)[2])
        for function, kind in [("event_cursor", "connection_access"), ("event_cursor", "sql_candidate"), ("host::event_cursor", "managed_reference"), ("new_cursor", "managed_reference")]:
            self.assertFalse(GUARD.classify(file, function, kind, "", 0)[2])
        self.assertFalse(GUARD.classify("src/relay/mod.rs", "is_worker_heartbeat_fresh", "managed_reference", "", 0)[2])

    def test_pure_filter_sql_is_separate_from_borrowed_authorizer_and_constructors(self):
        result = GUARD.classify("src/events/filters.rs", "build_sql_from_flags", "sql_candidate", "", 0)
        self.assertTrue(result[2])
        self.assertEqual(result[0], "pure_policy")
        self.assertFalse(GUARD.classify("src/events/filters.rs", "build_sql_from_flags", "connection_access", "", 0)[2])
        self.assertTrue(GUARD.classify("src/events/predicate.rs", "<module>", "sqlite_dependency", "", 0)[2])
        self.assertFalse(GUARD.classify("src/events/predicate.rs", "read", "sqlite_constructor", "Connection::open(path)", 0)[2])
        self.assertFalse(GUARD.classify("src/runtime/bootstrap.rs", "get_bootstrap", "managed_reference", "", 0)[2])

    def test_explicit_invocation_actor_does_not_authorize_ambient_or_nested_fallback(self):
        file = "src/integrations/hooks/claude/actor.rs"
        self.assertTrue(GUARD.classify(file, "resolve_actor_from_env", "managed_reference", "", 0)[2])
        self.assertTrue(GUARD.classify(file, "ensure_explicit_matches", "managed_reference", "", 0)[2])
        for home, kind in [("resolve_env_actor", "managed_reference"), ("resolve_actor_from_env", "connection_access"), ("resolve_actor_from_env::fallback", "managed_reference"), ("<module>", "sqlite_dependency")]:
            self.assertFalse(GUARD.classify(file, home, kind, "", 0)[2])

    def test_native_adoption_cannot_authorize_legacy_claim_or_new_sql(self):
        file = "src/runtime/process_tracking.rs"
        self.assertTrue(GUARD.classify(file, "try_adopt_orphans", "connection_access", "", 0)[2])
        self.assertTrue(GUARD.classify(file, "try_owning_instance", "sql_candidate", "", 0)[2])
        for home, kind in [("claim_orphans", "managed_reference"), ("claim_orphans_deferred", "managed_reference"), ("try_adopt_orphans", "sql_candidate"), ("try_owning_instance", "managed_constructor"), ("try_adopt_orphans::host", "connection_access")]:
            self.assertFalse(GUARD.classify(file, home, kind, "", 0)[2])


if __name__ == "__main__":
    unittest.main()
