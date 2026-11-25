use super::*;
use crate::cloud::communication::store::transaction;
use crate::cloud::control::workspaces::{expiry_window, page};

const GEN: &str = "generation-one";
const HOST: &str = "worker";
const INSTALLATION: &str = "installation-one";
const FIRST: &str = "lease-one";
const SECOND: &str = "lease-two";

struct Fixture(tempfile::TempDir);

impl Fixture {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        crate::cloud::communication::initialize(root.path(), GEN).unwrap();
        Self(root)
    }

    fn at<T>(&self, now: i64, action: impl Fn(&Connection, i64) -> Result<T>) -> Result<T> {
        transaction(self.0.path(), GEN, true, || true, |db, _| action(db, now))
    }

    fn enroll(&self, now: i64, lease_id: &str, previous_lease_id: Option<&str>, workspaces: Vec<WorkspaceCapability>) -> Result<HostCapabilities> {
        self.enroll_features(now, lease_id, previous_lease_id, workspaces, LeaseFeatures::default())
    }

    fn enroll_features(&self, now: i64, lease_id: &str, previous_lease_id: Option<&str>, workspaces: Vec<WorkspaceCapability>, features: LeaseFeatures) -> Result<HostCapabilities> {
        self.at(now, |db, now| {
            renew(
                db,
                now,
                HOST.into(),
                &RenewControlLease {
                    generation: GEN.into(),
                    installation_id: INSTALLATION.into(),
                    lease_id: lease_id.into(),
                    previous_lease_id: previous_lease_id.map(str::to_owned),
                    workspaces: workspaces.clone(),
                    project_roots: vec!["main".into()],
                },
                &features,
            )
            .map(|(capabilities, _)| capabilities)
        })
    }

    fn submit(&self, now: i64, id: &str, command: ControlCommand) -> Result<ControlReceipt> {
        self.at(now, |db, now| create(db, now, &CreateControlOperation { generation: GEN.into(), request_id: id.into(), host: HOST.into(), installation_id: INSTALLATION.into(), command: command.clone() }))
    }

    fn lookup(&self, now: i64, id: &str) -> ControlReceipt {
        self.at(now, |db, now| get(db, now, GEN, id)).unwrap()
    }

    fn receive(&self, now: i64, lease_id: &str) -> Result<Option<ControlReceipt>> {
        self.at(now, |db, now| poll(db, now, HOST, &PollControlOperation { generation: GEN.into(), installation_id: INSTALLATION.into(), lease_id: lease_id.into(), wait_ms: None }).map(|poll| poll.operation))
    }

    fn report(&self, now: i64, id: &str, lease_id: &str, claim_id: &str, outcome: ControlOutcome) -> Result<ControlReceipt> {
        self.at(now, |db, now| report(db, now, HOST, id, &ReportControlOperation { generation: GEN.into(), installation_id: INSTALLATION.into(), lease_id: lease_id.into(), claim_id: claim_id.into(), outcome: outcome.clone() }))
    }

    fn agent(&self, now: i64, incarnation: &str, retire: bool) -> AgentView {
        self.agent_in(now, incarnation, retire, AgentWorkspace::Project { workspace_id: "WS0000000001".into() })
    }

    fn agent_in(&self, now: i64, incarnation: &str, retire: bool, workspace: AgentWorkspace) -> AgentView {
        self.at(now, |db, now| {
            let input = AgentRegistration { generation: GEN.into(), installation_id: INSTALLATION.into(), host: HOST.into(), incarnation: incarnation.into(), workspace: workspace.clone() };
            if retire { store::retire_agent(db, now, "worker-agent".into(), input) } else { store::register_agent(db, now, "worker-agent".into(), input) }
        })
        .unwrap()
    }
}

#[test]
fn directory_launch_requires_support_and_replays_only_the_original_path() {
    let f = Fixture::new();
    let command = |path: &str, create: bool| {
        let mut command = launch(None, None);
        if let ControlCommand::Launch { working_directory, create_directory, .. } = &mut command {
            *working_directory = Some(path.into());
            *create_directory = create.then_some(true);
        }
        command
    };
    assert_eq!(f.enroll(100, FIRST, None, catalogue()).unwrap().directory_launch, None);
    let legacy = launch(None, None);
    assert!(serde_json::to_value(&legacy).unwrap().get("working_directory").is_none());
    assert_eq!(f.submit(100, "legacy", legacy).unwrap().status, ControlStatus::Queued);
    assert!(matches!(f.submit(100, "new", command("subdir", true)), Err(Error::Capability)));
    let features = LeaseFeatures { directory_launch: true, ..Default::default() };
    assert_eq!(f.enroll_features(101, FIRST, None, catalogue(), features).unwrap().directory_launch, Some(true));
    for path in ["", "../external", "/absolute"] {
        assert!(matches!(f.submit(101, "new", command(path, true)), Err(Error::Invalid(_))));
    }
    let first = f.submit(101, "new", command("subdir", true)).unwrap();
    assert_eq!(f.submit(101, "new", command("subdir", true)).unwrap(), first);
    assert!(matches!(f.submit(101, "new", command("different", true)), Err(Error::Conflict(_))));
    assert!(matches!(f.submit(101, "new", command("subdir", false)), Err(Error::Conflict(_))));
}

fn workspace_features() -> LeaseFeatures {
    LeaseFeatures { workspace_management: true, ..Default::default() }
}

fn add_workspace(parent: &str, directory: &str, name: &str) -> ControlCommand {
    ControlCommand::AddWorkspace { parent: parent.into(), directory: directory.into(), name: name.into(), create_directory: true }
}

#[test]
fn workspace_and_stop_inputs_reject_retired_identifier_formats() {
    let f = Fixture::new();
    let mut invalid = catalogue();
    invalid[0].workspace_id = "ws_g1IMogNN8O8DXrbzbvh4TOT3_Oxzn-06yjXb6OTskPI".into();
    assert!(matches!(f.enroll(100, FIRST, None, invalid.clone()), Err(Error::Invalid(_))));
    f.enroll(100, FIRST, None, catalogue()).unwrap();
    let mut legacy_launch = launch(None, None);
    if let ControlCommand::Launch { workspace_id, .. } = &mut legacy_launch {
        *workspace_id = invalid.pop().unwrap().workspace_id;
    }
    assert!(matches!(f.submit(100, "launch-legacy-id", legacy_launch), Err(Error::Invalid(_))));
    let stop = ControlCommand::Stop { name: "worker-agent".into(), incarnation: "550e8400-e29b-41d4-a716-446655440000".into() };
    assert!(matches!(f.submit(100, "stop-legacy-id", stop), Err(Error::Invalid(_))));
}

#[test]
fn workspace_admission_uses_root_aliases_and_allows_duplicate_display_names() {
    let f = Fixture::new();
    let mut workspaces = catalogue();
    workspaces.push(WorkspaceCapability { workspace_id: "WS0000000002".into(), ..workspaces[0].clone() });
    let command = add_workspace("main", "projects/new", "Example");
    let legacy = f.enroll(100, FIRST, None, workspaces.clone()).unwrap();
    assert_eq!(legacy.workspace_management, None);
    assert!(matches!(f.submit(100, "legacy", command.clone()), Err(Error::Capability)));
    let negotiated = f.enroll_features(101, FIRST, None, workspaces.clone(), workspace_features()).unwrap();
    assert_eq!(negotiated.workspace_management, Some(true));
    assert_eq!(negotiated.project_roots, ["main"]);
    assert!(matches!(f.submit(101, "unknown-parent", add_workspace("second-root", "subdir", "Example")), Err(Error::Capability)));
    assert!(matches!(f.submit(101, "invalid-parent", add_workspace("WS0000000002", "subdir", "Example")), Err(Error::Invalid(_))));
    for path in ["", "../escape", "/absolute", "missing//leaf", "C:\\outside", "project/./leaf"] {
        assert!(matches!(f.submit(101, "invalid-path", add_workspace("main", path, "Example")), Err(Error::Invalid(_))));
    }
    assert!(matches!(f.submit(101, "invalid-name", add_workspace("main", "subdir", "  ")), Err(Error::Invalid(_))));
    let original = f.submit(101, "new", command.clone()).unwrap();
    assert_eq!(f.submit(101, "new", command.clone()).unwrap(), original);
    assert!(matches!(f.submit(101, "new", add_workspace("main", "projects/other", "Example")), Err(Error::Conflict(_))));
    f.enroll(103, FIRST, None, workspaces).unwrap();
    assert_eq!(f.receive(103, FIRST).unwrap(), None);
    assert_eq!(f.lookup(103, "new").outcome, Some(ControlOutcome::Failed { code: "capability_unavailable".into() }));
    assert_eq!(f.at(134, |db, now| capabilities(db, now, GEN.into(), HOST.into())).unwrap().workspace_management, None);
    assert!(matches!(f.receive(134, FIRST), Err(Error::Lease)));
    assert!(matches!(f.submit(134, "offline", command), Err(Error::Offline)));
}

#[test]
fn workspace_pages_keep_same_name_projects_and_expired_catalogues() {
    let f = Fixture::new();
    let source = catalogue()[0].clone();
    let renew_on = |db: &Connection, now, host: &str, lease: &str, workspaces: Vec<WorkspaceCapability>| {
        renew(
            db,
            now,
            host.into(),
            &RenewControlLease { generation: GEN.into(), installation_id: format!("{host}-install"), lease_id: lease.into(), previous_lease_id: None, workspaces, project_roots: vec!["default".into()] },
            &LeaseFeatures::default(),
        )
        .map(|(_, changed)| changed)
    };
    f.at(100, |db, now| renew_on(db, now, "alpha", "a", vec![source.clone()])).map(|changed| assert!(changed)).unwrap();
    f.at(110, |db, now| {
        assert!(!renew_on(db, now, "alpha", "a", vec![source.clone()])?);
        assert!(renew_on(db, now, "beta", "b", vec![WorkspaceCapability { workspace_id: "WS0000000003".into(), ..source.clone() }, source.clone()])?);
        Ok(())
    })
    .unwrap();
    f.at(115, |db, now| {
        let mut renamed = source.clone();
        renamed.name = "Renamed project".into();
        assert!(renew_on(db, now, "alpha", "a", vec![renamed])?);
        Ok(())
    })
    .unwrap();
    f.at(140, |db, now| {
        assert_eq!(expiry_window(db, 139, now)?, (140, true, Some(145)));
        let first = page(db, now, GEN.into(), &WorkspaceQuery { generation: None, host: None, after: None, limit: Some(2) })?;
        assert_eq!(first.items.iter().map(|item| (item.host.as_str(), item.online)).collect::<Vec<_>>(), [("alpha", true), ("beta", false)]);
        let second = page(db, now, GEN.into(), &WorkspaceQuery { generation: None, host: Some("beta".into()), after: first.next, limit: Some(2) })?;
        assert_eq!(second.items.len(), 1);
        assert_eq!(second.items[0].capability.workspace_id, "WS0000000003");
        assert_eq!(second.items[0].capability.name, source.name);
        assert!(!second.items[0].online);
        assert!(second.next.is_none());
        Ok(())
    })
    .unwrap();
    f.at(143, |db, now| {
        assert!(renew_on(db, now, "beta", "replacement", vec![source.clone()])?);
        Ok(())
    })
    .unwrap();
}

#[test]
fn workspace_success_requires_exact_claim_outcome_and_catalogue_publication() {
    let f = Fixture::new();
    let mut workspaces = catalogue();
    f.enroll_features(100, FIRST, None, workspaces.clone(), workspace_features()).unwrap();
    let command = add_workspace("main", "project", "project");
    assert_eq!(f.submit(100, "register", command).unwrap().status, ControlStatus::Queued);
    let claimed = f.receive(100, FIRST).unwrap().unwrap();
    let claim = claimed.claim_id.unwrap();
    let added = ControlOutcome::WorkspaceAdded { workspace_id: "WS0000000004".into() };
    assert!(matches!(f.report(101, "register", FIRST, "wrong-claim", added.clone()), Err(Error::ControlClaim)));
    assert!(matches!(f.report(101, "register", FIRST, &claim, ControlOutcome::WorkspaceAdded { workspace_id: "WS0000000003".into() }), Err(Error::Capability)));
    let agent = f.agent(101, "ExactAgent01", false);
    assert!(matches!(f.report(101, "register", FIRST, &claim, ControlOutcome::Succeeded { agent }), Err(Error::Invalid(_))));
    assert!(matches!(f.report(101, "register", FIRST, &claim, added.clone()), Err(Error::Capability)));
    workspaces.push(WorkspaceCapability { workspace_id: "WS0000000004".into(), name: "project".into(), ..workspaces[0].clone() });
    f.enroll_features(102, SECOND, Some(FIRST), workspaces, workspace_features()).unwrap();
    assert_eq!(f.receive(102, SECOND).unwrap(), None);
    assert_eq!(f.lookup(102, "register").status, ControlStatus::Uncertain);
    assert!(matches!(f.report(102, "register", FIRST, &claim, added.clone()), Err(Error::Lease)));
    let done = f.report(102, "register", SECOND, &claim, added.clone()).unwrap();
    assert_eq!(done.status, ControlStatus::Succeeded);
    assert_eq!(done.outcome, Some(added.clone()));
    assert_eq!(f.report(102, "register", SECOND, &claim, added.clone()).unwrap(), done);
    assert_eq!(f.lookup(132, "register"), done);
    assert!(matches!(f.report(102, "register", SECOND, &claim, ControlOutcome::Failed { code: "other".into() }), Err(Error::ControlOutcome)));
    f.submit(103, "launch", launch(None, None)).unwrap();
    let launch_claim = f.receive(103, SECOND).unwrap().unwrap().claim_id.unwrap();
    assert!(matches!(f.report(103, "launch", SECOND, &launch_claim, added), Err(Error::Invalid(_))));
}

fn catalogue() -> Vec<WorkspaceCapability> {
    vec![WorkspaceCapability {
        workspace_id: "WS0000000001".into(),
        name: "Example".into(),
        default_available: true,
        default_reason: None,
        roles: vec![RoleCapability { name: "coder".into(), available: true, reason: None }, RoleCapability { name: "windows-only".into(), available: false, reason: Some("missing_provider".into()) }],
    }]
}

fn launch(role: Option<&str>, name: Option<&str>) -> ControlCommand {
    ControlCommand::Launch {
        workspace_id: "WS0000000001".into(),
        role: role.map(str::to_owned),
        name: name.map(str::to_owned),
        title: Some("API verification".into()),
        initial_prompt: Some("Check the project snapshot.\nReport findings.".into()),
        working_directory: None,
        create_directory: None,
    }
}

#[test]
fn only_control_lease_enables_typed_operations_and_revalidates_catalogue() {
    let f = Fixture::new();
    let db = f.0.path();
    store::transaction(db, GEN, true, || true, |conn, _| store::renew_host(conn, 10, HOST.into(), HostRegistration { generation: GEN.into(), installation_id: INSTALLATION.into() })).unwrap();
    let no_lease = transaction(db, GEN, false, || true, |conn, _| capabilities(conn, 10, GEN.into(), HOST.into())).unwrap();
    assert!(!no_lease.online);
    assert_eq!(no_lease.observed_at, None);
    assert!(matches!(f.submit(10, "default", launch(None, None)), Err(Error::Offline)));

    let managed = f.enroll(11, FIRST, None, catalogue()).unwrap();
    assert!(managed.online);
    assert_eq!(managed.observed_at, Some(11));
    assert!(matches!(f.submit(11, "..", launch(None, None)), Err(Error::Invalid(_))));
    assert!(matches!(f.submit(11, "unavailable", launch(Some("windows-only"), None)), Err(Error::Capability)));
    assert_eq!(f.submit(11, "default", launch(None, None)).unwrap().status, ControlStatus::Queued);
    assert_eq!(f.submit(11, "explicit", launch(Some("coder"), None)).unwrap().status, ControlStatus::Queued);
    let mut no_roles = catalogue();
    no_roles[0].roles.clear();
    f.enroll(12, FIRST, None, no_roles).unwrap();
    let default = f.receive(13, FIRST).unwrap().unwrap();
    assert_eq!(default.request_id, "default");
    f.report(13, "default", FIRST, &default.claim_id.unwrap(), ControlOutcome::Failed { code: "local_launch_failed".into() }).unwrap();
    assert!(f.receive(14, FIRST).unwrap().is_none());
    assert_eq!(f.lookup(14, "explicit").outcome, Some(ControlOutcome::Failed { code: "capability_unavailable".into() }));
    assert!(matches!(transaction(db, "stale-generation", false, || true, |_, _| Ok(())), Err(Error::Generation)));
    assert!(matches!(transaction(db, GEN, false, || false, |_, _| Ok(())), Err(Error::Credential)));
}

#[test]
fn replay_and_reconcile_a_claim_across_lease_replacement_without_redispatch() {
    let f = Fixture::new();
    f.enroll(100, FIRST, None, catalogue()).unwrap();
    let requested = launch(Some("coder"), Some("worker-agent"));
    f.submit(100, "running", requested.clone()).unwrap();
    f.submit(100, "pending", launch(None, None)).unwrap();
    let acquired = f.receive(101, FIRST).unwrap().unwrap();
    assert_eq!(acquired.status, ControlStatus::Running);
    assert_eq!(f.receive(102, FIRST).unwrap().unwrap(), acquired);
    let claim = acquired.claim_id.unwrap();
    assert!(matches!(f.enroll(103, SECOND, None, catalogue()), Err(Error::Lease)));
    f.enroll(103, SECOND, Some(FIRST), catalogue()).unwrap();
    assert!(matches!(f.receive(104, FIRST), Err(Error::Lease)));
    assert!(matches!(f.enroll(134, FIRST, Some(SECOND), catalogue()), Err(Error::Lease)));
    assert!(f.receive(104, SECOND).unwrap().is_none());
    assert_eq!(f.lookup(104, "pending").outcome, Some(ControlOutcome::Failed { code: "host_lease_lost".into() }));
    let uncertain = f.submit(104, "running", requested).unwrap();
    assert_eq!(uncertain.status, ControlStatus::Uncertain);
    assert_eq!(uncertain.claim_id.as_deref(), Some(claim.as_str()));
    assert!(matches!(f.submit(104, "running", launch(None, None)), Err(Error::Conflict(_))));

    let agent = f.agent(105, "Incarnation1", false);
    let outcome = ControlOutcome::Succeeded { agent };
    assert!(matches!(f.report(106, "running", FIRST, &claim, outcome.clone()), Err(Error::Lease)));
    assert!(matches!(f.report(106, "running", SECOND, "wrong-claim", outcome.clone()), Err(Error::ControlClaim)));
    let result = f.report(106, "running", SECOND, &claim, outcome.clone()).unwrap();
    assert_eq!(f.report(107, "running", SECOND, &claim, outcome).unwrap(), result);
    assert!(matches!(f.report(107, "running", SECOND, &claim, ControlOutcome::Uncertain { code: "unknown".into() }), Err(Error::ControlOutcome)));
    assert_eq!(f.lookup(200, "running"), result);
}

#[test]
fn launch_success_checks_authoritative_exact_membership_even_after_exit() {
    let f = Fixture::new();
    f.enroll(100, FIRST, None, catalogue()).unwrap();
    f.submit(100, "start", launch(None, Some("worker-agent"))).unwrap();
    let claim = f.receive(100, FIRST).unwrap().unwrap().claim_id.unwrap();
    let wrong = f.agent_in(101, "WrongAgent01", false, AgentWorkspace::Project { workspace_id: "WS0000000003".into() });
    let forged = AgentView { workspace: AgentWorkspace::Project { workspace_id: "WS0000000001".into() }, ..wrong.clone() };
    assert!(matches!(f.report(102, "start", FIRST, &claim, ControlOutcome::Succeeded { agent: forged }), Err(Error::AgentWorkspace)));
    assert_eq!(f.lookup(102, "start").status, ControlStatus::Running);
    f.agent(102, "WrongAgent01", true);
    let right = f.agent(103, "RightAgent01", false);
    f.agent(104, "RightAgent01", true);
    let stale_hint = AgentView { workspace: AgentWorkspace::Unexposed, ..right.clone() };
    let outcome = ControlOutcome::Succeeded { agent: stale_hint };
    let receipt = f.report(105, "start", FIRST, &claim, outcome.clone()).unwrap();
    assert_eq!(receipt.outcome, Some(ControlOutcome::Succeeded { agent: right }));
    assert_eq!(f.report(106, "start", FIRST, &claim, outcome).unwrap(), receipt);
}

#[test]
fn exact_stop_cannot_claim_or_retire_a_replacement() {
    let f = Fixture::new();
    f.enroll(100, FIRST, None, catalogue()).unwrap();
    let original = f.agent(101, "Incarnation1", false);
    let stop = |agent: &AgentView| ControlCommand::Stop { name: agent.name.clone(), incarnation: agent.incarnation.clone() };
    f.submit(101, "first-stop", stop(&original)).unwrap();
    f.agent(102, &original.incarnation, true);
    let next = f.agent(102, "Incarnation2", false);
    assert!(f.receive(103, FIRST).unwrap().is_none());
    assert_eq!(f.lookup(103, "first-stop").outcome, Some(ControlOutcome::Failed { code: "stale_agent".into() }));
    assert!(matches!(f.submit(103, "stale-stop", stop(&original)), Err(Error::Agent)));

    f.submit(104, "next-stop", stop(&next)).unwrap();
    let claim = f.receive(104, FIRST).unwrap().unwrap().claim_id.unwrap();
    assert!(matches!(f.report(105, "next-stop", FIRST, &claim, ControlOutcome::Succeeded { agent: original }), Err(Error::Agent)));
    let result = f.report(105, "next-stop", FIRST, &claim, ControlOutcome::Succeeded { agent: next.clone() }).unwrap();
    assert_eq!(result.status, ControlStatus::Succeeded);
    assert_eq!(f.at(106, |conn, _| store::active_agent(conn, &next.name)).unwrap(), None);
    let newer = f.agent(106, "Incarnation3", false);
    assert_eq!(f.submit(107, "next-stop", stop(&next)).unwrap(), result);
    assert_eq!(f.at(107, |conn, _| store::active_agent(conn, &next.name)).unwrap(), Some(newer));
    assert!(matches!(f.receive(130, FIRST), Err(Error::Lease)));
    assert!(matches!(f.enroll(130, FIRST, None, catalogue()), Err(Error::Lease)));
}
