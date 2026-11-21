#!/usr/bin/env python3
"""Generate the current production database inventory without opening databases.

Run from any directory: python3 .docs/model/generate-db-access.py
The integration-owned scanner supplies test exclusion and SQLite candidates.
This model-owned projection adds references, process/domain ownership and the
known mixed Provider/AgentX boundaries; it does not enforce final cutover.
"""

from collections import Counter, defaultdict
import hashlib
import json
from pathlib import Path
import re
import types


ROOT = Path(__file__).resolve().parents[2]
SCANNER = "scripts/check-db-ownership.py"
REFERENCES = {
    "db_adapter_reference": r"\bAgentxDb\b",
    "database_record_or_module_reference": r"\b(?:crate::db|db::(?:InstanceRow|AgentxDb|DEV_ROOT_KV_KEY))\b",
}
OPENERS = {"managed_constructor", "sqlite_constructor", "python_sqlite_constructor"}


def digest(data):
    return hashlib.sha256(data).hexdigest()


def owner(path):
    if path.startswith("src/agents/"):
        return "Participant identity / consistency"
    if path.startswith("src/messaging/"):
        return "Messaging / consistency"
    if path.startswith("src/db/"):
        return "Persistence / consistency"
    if path.startswith(("src/integrations/", "src/runtime/")):
        return "Provider / runtime hosts"
    if path.startswith("src/relay/"):
        return "Relay / product clients"
    if path.startswith("src/tui/"):
        return "TUI / product clients"
    if path.startswith("src/events/"):
        return "Events / product clients"
    if path.startswith("src/transcript/"):
        return "External transcript adapter / product clients"
    if path == "src/main.rs":
        return "Executable composition / product clients"
    return "CLI / product clients"


def boundary(path, function):
    if path in {"src/transcript/opencode.rs", "src/transcript/hermes.rs", "src/integrations/plugins/hermes/__init__.py"}:
        return "provider"
    if path == "src/cli/commands/resume.rs" and function == "lookup_opencode_family_session":
        return "provider"
    return "managed"


def collect(scanner, root):
    scan = scanner.inventory(root)
    findings = defaultdict(list)
    for finding in scan["findings"]:
        findings[finding["file"]].append(finding)
    excluded = set(scan["excluded_external_test_files"])
    files = []
    for path in sorted((root / "src").rglob("*.rs")):
        relative = path.relative_to(root).as_posix()
        if relative in excluded:
            continue
        source = path.read_text()
        code, _, _ = scanner.production_source(source)
        for kind, pattern in REFERENCES.items():
            for match in re.finditer(pattern, code):
                findings[relative].append({"line": source.count("\n", 0, match.start()) + 1, "kind": kind})
        if not findings[relative]:
            continue
        functions = [(code.count("\n", 0, match.start()) + 1, match[1]) for match in re.finditer(r"\bfn\s+(\w+)\s*(?:<|\()", code)]
        lines = source.splitlines()
        sites = []
        for row in sorted(findings[relative], key=lambda item: (item["line"], item["kind"])):
            function = next((name for line, name in reversed(functions) if line <= row["line"]), None)
            sites.append({"line": row["line"], "kind": row["kind"], "source": lines[row["line"] - 1].strip(), "nearest_function": function, "boundary": boundary(relative, function)})
        files.append(file_record(relative, source, sites))
    # The embedded Hermes adapter reads the Provider's state.db, never AgentX.
    for path in sorted((root / "src/integrations/plugins").rglob("*.py")):
        source = path.read_text()
        relative = path.relative_to(root).as_posix()
        sites = []
        for number, line in enumerate(source.splitlines(), 1):
            if re.search(r"\bsqlite3\.connect\s*\(", line):
                sites.append({"line": number, "kind": "python_sqlite_constructor", "source": line.strip(), "boundary": boundary(relative, None)})
            elif scanner.SQL.search(line) or "import sqlite3" in line or re.search(r"\bdb\.execute\s*\(", line):
                sites.append({"line": number, "kind": "python_sqlite_reference", "source": line.strip(), "boundary": boundary(relative, None)})
        if sites:
            files.append(file_record(relative, source, sites))
    return scan, sorted(files, key=lambda item: item["path"])


def file_record(path, source, sites):
    boundaries = {site["boundary"] for site in sites}
    return {"path": path, "source_sha256": digest(source.encode()), "owner": owner(path), "boundary": next(iter(boundaries)) if len(boundaries) == 1 else "mixed", "opener_count": sum(site["kind"] in OPENERS for site in sites), "sites": sites}


def markdown(report):
    summary = report["summary"]
    lines = ["---", "schema: document", "status: active", "owner: architecture", "last_updated: 2026-11-17", "description: Current production database dependencies and owners.", "links: [./README.md, ./db-access.json, ./generate-db-access.py, ../architecture/acceptance.md]", "---", "", "# Production database access inventory", "", f"The current working tree contains {report['rust_source_files']} Rust source files; {len(report['excluded_external_test_files'])} external test files and inline test items are excluded. The inventory covers {summary['affected_files']} affected Rust/resource files and {summary['sites']} syntactic candidates/references. It records {summary['managed_opener_sites']} managed constructor sites and {summary['provider_opener_sites']} Provider constructor sites. A constructor function reference is a site, not necessarily an executed open; in-memory constructors in the persistence adapter are included.", "", "[JSON evidence](db-access.json) retains every candidate, source line, source hash, domain owner and per-site database boundary, plus the exact scanner bytes/hash. [Generator](generate-db-access.py) reuses the integration-owned scanner; run `python3 .docs/model/generate-db-access.py` to regenerate against the current working tree. No database, live data scope or Provider is opened. Multiple kinds at one line are separate dependency evidence, not separate operations.", "", "## Boundary decisions", "", "- Provider session lookups read external OpenCode/Kilo storage through the owning read-only adapter; managed AgentX storage remains daemon-owned.", "- Managed AgentX KV reads remain daemon-owned. Read-only access does not make a managed database a Provider exception.", "- `transcript/{hermes,opencode}.rs` and the embedded Hermes Python `state.db` reads are external Provider adapters. Preserve their explicit read-only contracts.", "- AgentX archives are managed product data: archive connections belong to daemon maintenance/read handlers, even when opened read-only. Clients cannot select an arbitrary database path.", "", "## Capability ownership", "", "| Owning capability | Affected files | Constructor sites | Candidates/references |", "| --- | ---: | ---: | ---: |"]
    for name in sorted({item["owner"] for item in report["files"]}):
        owned = [item for item in report["files"] if item["owner"] == name]
        lines.append(f"| {name} | {len(owned)} | {sum(item['opener_count'] for item in owned)} | {sum(len(item['sites']) for item in owned)} |")
    lines += ["", "## Complete affected-file map", "", "The owner is the source responsibility, not proof of current daemon composition. Paths link to current source; line numbers and content hashes in JSON refer to the recorded source snapshot. All candidate-only files remain in this map so migration cannot focus exclusively on explicit constructors.", "", "| Source | Boundary | Owner | Constructor source lines | Sites |", "| --- | --- | --- | --- | ---: |"]
    for item in report["files"]:
        openers = ", ".join(f"{site['line']} ({site['boundary']})" for site in item["sites"] if site["kind"] in OPENERS) or "—"
        lines.append(f"| [{item['path']}](../../{item['path']}) | {item['boundary']} | {item['owner']} | {openers} | {len(item['sites'])} |")
    lines += ["", "## Final cutover and evidence limits", "", "This is the complete syntactic inventory under the declared scanner coverage, with the mixed-store decisions above. It is not compiler-resolved SQL lineage or a final exclusivity pass. Alias constructors, macro expansions and arbitrary cfg expressions remain scanner limitations; nearest-function labels are lexical hints. Candidate SQL may include comments/strings or read-only text. Raw SQLite access inside a daemon-only domain adapter still requires ownership review; a file-name allowlist alone cannot prove process ownership.", "", "Final source-cutover and fixed-binary process/FD/legacy evidence belong to [independent acceptance](../architecture/acceptance.md) and the [Delivery](../../.controller/domain-daemon/delivery.md). Existing clients and hosts are not declared migrated by this source inventory. Old binaries that ignore the singleton lock require an explicit supported cutover; a new lock does not exclude them.", ""]
    return "\n".join(lines)


def main():
    scanner_bytes = (ROOT / SCANNER).read_bytes()
    scanner = types.ModuleType("db_ownership_scanner")
    exec(compile(scanner_bytes, SCANNER, "exec"), scanner.__dict__)
    scan, files = collect(scanner, ROOT)
    opener_counts = Counter(site["boundary"] for item in files for site in item["sites"] if site["kind"] in OPENERS)
    report = {"scope": "Current production Rust source and embedded Python SQLite adapters; test-only Rust items/modules excluded by retained scanner.", "scanner": SCANNER, "scanner_sha256": digest(scanner_bytes), "scanner_source": scanner_bytes.decode(), "generator_sha256": digest(Path(__file__).read_bytes()), "rust_source_files": scan["source_files"], "excluded_external_test_files": scan["excluded_external_test_files"], "files": files, "limitations": [scan["limits"], "References are candidates, not executed opens; function labels are lexical hints.", "Source links are current; line numbers/hashes refer to the recorded source snapshot.", "This source inventory does not establish final cutover, process FD ownership or legacy process exclusion."], "summary": {"affected_files": len(files), "boundary_files": dict(Counter(item["boundary"] for item in files)), "managed_opener_sites": opener_counts["managed"], "provider_opener_sites": opener_counts["provider"], "sites": sum(len(item["sites"]) for item in files)}}
    (ROOT / ".docs/model/db-access.json").write_text(json.dumps(report, indent=2) + "\n")
    (ROOT / ".docs/model/db-access.md").write_text(markdown(report))
    print(json.dumps({"scanner_sha256": report["scanner_sha256"], **report["summary"]}, indent=2))


if __name__ == "__main__":
    main()
