//! Finite native cases; independent execution owns end-to-end outcome evidence.
use super::*;
use std::fs;

fn args(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| value.to_string()).collect()
}
struct Fixture {
    root: tempfile::TempDir,
    base: HashMap<String, String>,
}
impl Fixture {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let scope = root.path().join("scope");
        fs::create_dir(&scope).unwrap();
        let base = HashMap::from([("HOME".into(), root.path().to_string_lossy().into_owned()), ("AGENTX_DIR".into(), scope.to_string_lossy().into_owned())]);
        Self { root, base }
    }
    fn write(&self, path: &str, value: &str) {
        let path = self.root.path().join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, value).unwrap();
    }
    fn resolve(&self, argv: &[&str]) -> ResolvedEnvironment {
        resolve(&self.base, &args(argv), self.root.path()).unwrap()
    }
}

#[test]
fn ordered_files_and_duplicate_keys_use_later_values_with_real_empties() {
    let fixture = Fixture::new();
    fixture.write("first.env", "VALUE=first\nDUP=first\nDUP=second\nEMPTY=\n");
    fixture.write("second.env", "VALUE=second\n");
    let result = fixture.resolve(&["--env-file", "first.env", "--env-file=second.env", "config", "environment"]);
    assert_eq!(result.values["VALUE"], "second");
    assert_eq!(result.values["DUP"], "second");
    assert_eq!(result.values["EMPTY"], "");
    assert_eq!(result.receipt.variables["VALUE"], VariableProvenance::File { index: 2 });
    assert_eq!(result.receipt.sources.len(), 2);
}

#[test]
fn caller_values_including_empty_win_unless_overridden_and_last_cli_flag_wins() {
    let mut fixture = Fixture::new();
    fixture.write("values.env", "VALUE=file\nEMPTY=file\n");
    fixture.base.extend([("VALUE".into(), "caller".into()), ("EMPTY".into(), "".into()), (OVERRIDE.into(), "true".into())]);
    let result = fixture.resolve(&["--env-file=values.env", "--env-override", "--no-env-override"]);
    assert_eq!(result.values["VALUE"], "caller");
    assert_eq!(result.values["EMPTY"], "");
    assert_eq!(result.receipt.variables["VALUE"], VariableProvenance::Inherited);
    let result = fixture.resolve(&["--env-file=values.env", "--no-env-override", "--env-override"]);
    assert_eq!(result.values["VALUE"], "file");
    assert_eq!(result.values["EMPTY"], "file");
}

#[test]
fn list_selection_replaces_lower_sources_and_explicit_empty_disables_them() {
    let mut fixture = Fixture::new();
    fixture.write("scope/config.toml", "[environment]\nfiles=['missing.env']\n");
    fixture.write("ambient.env", "VALUE=ambient\n");
    fixture.write("cli.env", "VALUE=cli\n");
    fixture.base.insert(FILES.into(), "ambient.env".into());
    assert_eq!(fixture.resolve(&[]).values["VALUE"], "ambient");
    assert_eq!(fixture.resolve(&["--env-file=cli.env"]).values["VALUE"], "cli");
    assert!(!fixture.resolve(&["--env-file="]).values.contains_key("VALUE"));
    fixture.base.insert(FILES.into(), "".into());
    assert!(fixture.resolve(&[]).receipt.sources.is_empty());
}

#[test]
fn toml_override_controls_external_list_before_scope_and_downstream_scope_does_not_recurse() {
    let fixture = Fixture::new();
    fixture.write("scope/config.toml", "[environment]\nfiles=['unused.env']\noverride=true\n");
    fixture.write("other/config.toml", "[environment]\nfiles=['missing.env']\noverride=false\n[launch]\nhints='other'\n");
    let other = fixture.root.path().join("other");
    fixture.write("external.env", &format!("AGENTX_DIR='{}'\nVALUE=external\n", other.display()));
    let result = fixture.resolve(&["--env-file=external.env"]);
    assert_eq!(result.receipt.selector_scope, fixture.root.path().join("scope"));
    assert_eq!(result.receipt.scope, other.canonicalize().unwrap());
    assert!(result.receipt.override_values);
    assert_eq!(result.values["VALUE"], "external");
    assert_eq!(crate::config::AgentxConfig::load_for_scope(&result.receipt.scope, &result.values).hints, "other");
}

#[test]
fn inherited_scope_wins_default_but_external_source_can_bootstrap_an_absent_scope() {
    let mut fixture = Fixture::new();
    fixture.write("external.env", "AGENTX_DIR=created-scope\nVALUE=external\n");
    assert_eq!(fixture.resolve(&["--env-file=external.env"]).receipt.scope, fixture.root.path().join("scope"));
    fixture.base.remove("AGENTX_DIR");
    assert_eq!(fixture.resolve(&["--env-file=external.env"]).receipt.scope, fixture.root.path().join("created-scope"));
    assert!(!fixture.root.path().join("created-scope").exists());
}

#[test]
fn scoped_file_redirect_is_rejected_before_any_scope_creation() {
    let fixture = Fixture::new();
    fixture.write("scope/config.toml", "[environment]\nfiles=['scoped.env']\noverride=true\n");
    fixture.write("scoped.env", "AGENTX_DIR=must-not-exist\nTOKEN=concealed-sentinel\n");
    let error = resolve(&fixture.base, &[], fixture.root.path()).err().unwrap().to_string();
    assert!(error.contains("cannot declare AGENTX_DIR"));
    assert!(!error.contains("concealed-sentinel"));
    assert!(!fixture.root.path().join("must-not-exist").exists());
}

#[test]
fn explicit_scope_is_final_while_project_relative_files_use_local_project() {
    let fixture = Fixture::new();
    fixture.write("project/.env", "AGENTX_DIR=other\nVALUE=project\n");
    let result = fixture.resolve(&["workbench", "--scope", "scope", "--project", "project", "--env-file=.env", "--env-override"]);
    assert_eq!(result.values["VALUE"], "project");
    assert_eq!(result.receipt.project, fixture.root.path().join("project"));
    assert_eq!(result.receipt.scope, fixture.root.path().join("scope"));
    assert!(result.receipt.withheld_keys.contains("AGENTX_DIR"));
}

#[test]
fn launch_dir_and_home_expansion_are_resolved_against_originating_inputs() {
    let fixture = Fixture::new();
    fixture.write("project/.env", "VALUE=project\n");
    fixture.write("shared.env", "SHARED=home\n");
    let result = fixture.resolve(&["launch", "--tool", "codex", "--dir=project", "--env-file", "~/shared.env", "--env-file=.env"]);
    assert_eq!(result.values["VALUE"], "project");
    assert_eq!(result.values["SHARED"], "home");
}

#[test]
fn remote_directory_and_native_scope_arguments_do_not_select_local_sources() {
    let fixture = Fixture::new();
    fixture.write("local.env", "VALUE=local\n");
    let result = fixture.resolve(&["launch", "--tool", "codex", "--device", "remote", "--dir", "/remote-only-project", "--scope", "/native-only-scope", "--env-file=local.env"]);
    assert_eq!(result.values["VALUE"], "local");
    assert_eq!(result.receipt.project, fixture.root.path());
    assert_eq!(result.receipt.selector_scope, fixture.root.path().join("scope"));
}

#[test]
fn selectors_inside_dotenv_are_withheld_and_cannot_select_another_source() {
    let fixture = Fixture::new();
    fixture.write("one.env", "AGENTX_ENV_FILES=missing.env\nAGENTX_ENV_OVERRIDE=true\nAGENTX_ENV_RETAIN_KEYS=invalid\nVALUE=one\n");
    let result = fixture.resolve(&["--env-file=one.env"]);
    assert_eq!(result.values["VALUE"], "one");
    assert!(!result.values.contains_key(FILES));
    assert!(!result.values.contains_key(OVERRIDE));
    assert_eq!(result.receipt.withheld_keys.len(), 3);
}

#[test]
fn errors_are_secret_free_and_directory_sources_fail() {
    let fixture = Fixture::new();
    fixture.write("invalid.env", "TOKEN='concealed-sentinel\n");
    for file in ["missing.env", "invalid.env", "scope"] {
        let error = resolve(&fixture.base, &args(&["--env-file", file]), fixture.root.path()).err().unwrap().to_string();
        assert!(error.contains(file));
        assert!(!error.contains("concealed-sentinel"));
        assert!(!fixture.root.path().join("scope/agentx.db").exists());
    }
}

#[test]
fn dotenv_quotes_comments_and_command_text_remain_data() {
    let fixture = Fixture::new();
    fixture.write("values.env", "export FIRST='literal $HOME $(touch never-created)'\nSECOND=plain # comment\nMULTI=\"line1\\nline2\"\n");
    let result = fixture.resolve(&["--env-file=values.env"]);
    assert_eq!(result.values["FIRST"], "literal $HOME $(touch never-created)");
    assert_eq!(result.values["SECOND"], "plain");
    assert_eq!(result.values["MULTI"], "line1\nline2");
    assert!(!fixture.root.path().join("never-created").exists());
}

#[test]
fn envrc_is_not_dotenv_and_invalid_boolean_does_not_echo_input() {
    let mut fixture = Fixture::new();
    fixture.write(".envrc", "export VALUE=one\n");
    let error = resolve(&fixture.base, &args(&["--env-file=.envrc"]), fixture.root.path()).err().unwrap().to_string();
    assert!(error.contains("authorized direnv"));
    fixture.base.insert(OVERRIDE.into(), "concealed-sentinel".into());
    let error = resolve(&fixture.base, &[], fixture.root.path()).err().unwrap().to_string();
    assert!(error.contains("true or false"));
    assert!(!error.contains("concealed-sentinel"));
}

#[test]
fn diagnostic_receipts_expose_provenance_and_never_values() {
    let fixture = Fixture::new();
    fixture.write("values.env", "PRIVATE_TOKEN=concealed-sentinel\nEMPTY=\n");
    let result = fixture.resolve(&["--env-file=values.env"]);
    let json = serde_json::to_string(&result.receipt).unwrap();
    assert!(json.contains("PRIVATE_TOKEN"));
    assert!(json.contains("loaded"));
    assert!(!json.contains("concealed-sentinel"));
    assert_eq!(result.receipt.variables["EMPTY"], VariableProvenance::File { index: 1 });
}

#[test]
fn independent_invocation_resolution_and_shell_restoration_keep_distinct_values() {
    let first = Fixture::new();
    let second = Fixture::new();
    first.write("values.env", "VALUE=first\nEMPTY=\n");
    second.write("values.env", "VALUE=second\n");
    let one = first.resolve(&["--env-file=values.env"]);
    let two = second.resolve(&["--env-file=values.env"]);
    let mut shell = HashMap::from([("VALUE".into(), "login".into()), ("SHELL_ONLY".into(), "kept".into())]);
    restore_originating_values(&mut shell, &one.values, &one.originating_keys);
    assert_eq!(shell["VALUE"], "first");
    assert_eq!(shell["EMPTY"], "");
    assert_eq!(shell["SHELL_ONLY"], "kept");
    restore_originating_values(&mut shell, &two.values, &two.originating_keys);
    assert_eq!(shell["VALUE"], "second");
    assert_eq!(one.values["VALUE"], "first");
    assert!(!first.base.contains_key("VALUE"));
}

#[test]
fn parser_preserves_opaque_values_separators_and_existing_global_flags() {
    let input = args(&["--env-file=a", "--iam", "caller", "codex", "--agentx-prompt", "--env-file", "--no-env-override", "--", "--env-file", "native"]);
    let (cleaned, selection) = extract_options(&input).unwrap();
    assert_eq!(selection.files, Some(vec!["a".into()]));
    assert_eq!(selection.override_values, Some(false));
    assert_eq!(cleaned, args(&["--iam", "caller", "codex", "--agentx-prompt", "--env-file", "--", "--env-file", "native"]));
    assert!(extract_options(&args(&["--env-file=a", "--env-file="])).is_err());
    let opaque = args(&["work", "command", "--command-id", "--env-file", "-i", "--env-override"]);
    assert_eq!(extract_options(&opaque).unwrap(), (opaque, CliSelection::default()));
}

#[test]
fn metadata_and_internal_entries_bypass_file_resolution() {
    for input in [vec!["--help-all"], vec!["events", "sub", "--help-all"], vec!["work", "report", "--help-all"]] {
        assert!(is_lightweight(&args(&input)), "{input:?}");
    }
    assert!(!is_lightweight(&args(&["events", "--sql", "--help-all"])));
    for input in [vec!["--version"], vec!["help", "codex"], vec!["config", "--help"], vec!["daemon", "serve", "--scope", "scope"], vec!["pty", "codex"], vec!["relay-worker"], vec!["launch", "--tool", "codex", "--stdio", "--", "--version"]] {
        assert!(is_lightweight(&args(&input)), "{input:?}");
    }
    assert!(!is_lightweight(&args(&["launch", "--tool", "codex", "--agentx-prompt", "--help"])));
    assert!(!is_lightweight(&args(&["config", "environment"])));
}

#[test]
fn retained_key_metadata_survives_a_new_origin_without_reloading_files() {
    let mut fixture = Fixture::new();
    fixture.base.insert(RETAIN.into(), "[\"VALUE\",\"EMPTY\"]".into());
    fixture.base.insert("VALUE".into(), "inherited-file".into());
    fixture.base.insert("EMPTY".into(), "".into());
    let result = fixture.resolve(&[]);
    assert_eq!(result.originating_keys, BTreeSet::from(["VALUE".into(), "EMPTY".into()]));
    assert!(result.receipt.sources.is_empty());
    let mut shell = HashMap::new();
    restore_originating_values(&mut shell, &result.values, &result.originating_keys);
    assert_eq!(shell["VALUE"], "inherited-file");
    assert_eq!(shell["EMPTY"], "");
}
