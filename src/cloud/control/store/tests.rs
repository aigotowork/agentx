use super::*;
use crate::cloud::communication::types::AgentRegistration;

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
        self.enroll_features(now, lease_id, previous_lease_id, workspaces, false)
    }

    fn enroll_features(&self, now: i64, lease_id: &str, previous_lease_id: Option<&str>, workspaces: Vec<WorkspaceCapability>, directory_launch: bool) -> Result<HostCapabilities> {
        self.at(now, |db, now| {
            renew(
                db,
                now,
                HOST.into(),
                &RenewControlLease { generation: GEN.into(), installation_id: INSTALLATION.into(), lease_id: lease_id.into(), previous_lease_id: previous_lease_id.map(str::to_owned), workspaces: workspaces.clone() },
                directory_launch,
            )
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
        self.at(now, |db, now| {
            let input = AgentRegistration { generation: GEN.into(), installation_id: INSTALLATION.into(), host: HOST.into(), incarnation: incarnation.into() };
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
    assert_eq!(f.enroll_features(101, FIRST, None, catalogue(), true).unwrap().directory_launch, Some(true));
    for path in ["", "../external", "/absolute"] {
        assert!(matches!(f.submit(101, "new", command(path, true)), Err(Error::Invalid(_))));
    }
    let first = f.submit(101, "new", command("subdir", true)).unwrap();
    assert_eq!(f.submit(101, "new", command("subdir", true)).unwrap(), first);
    assert!(matches!(f.submit(101, "new", command("different", true)), Err(Error::Conflict(_))));
    assert!(matches!(f.submit(101, "new", command("subdir", false)), Err(Error::Conflict(_))));
}

#[test]
fn directory_capability_upgrade_preserves_legacy_leases_and_receipts() {
    let f = Fixture::new();
    f.enroll(100, FIRST, None, catalogue()).unwrap();
    let command = launch(None, None);
    let queued = f.submit(100, "before-upgrade", command.clone()).unwrap();
    store::transaction(
        f.0.path(),
        GEN,
        true,
        || true,
        |db, _| {
            db.execute_batch("ALTER TABLE control_leases DROP COLUMN directory_launch; PRAGMA user_version=1;")?;
            Ok(())
        },
    )
    .unwrap();

    let legacy = transaction(f.0.path(), GEN, false, || true, |db, _| capabilities(db, 101, GEN.into(), HOST.into())).unwrap();
    assert!(legacy.online);
    assert_eq!(legacy.directory_launch, None);
    assert_eq!(f.submit(101, "before-upgrade", command).unwrap(), queued);
}

fn catalogue() -> Vec<WorkspaceCapability> {
    vec![WorkspaceCapability {
        alias: "main".into(),
        default_available: true,
        default_reason: None,
        roles: vec![RoleCapability { name: "coder".into(), available: true, reason: None }, RoleCapability { name: "windows-only".into(), available: false, reason: Some("missing_provider".into()) }],
    }]
}

fn launch(role: Option<&str>, name: Option<&str>) -> ControlCommand {
    ControlCommand::Launch {
        workspace: "main".into(),
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
    let legacy = transaction(db, GEN, false, || true, |conn, _| capabilities(conn, 10, GEN.into(), HOST.into())).unwrap();
    assert!(!legacy.online);
    assert_eq!(legacy.observed_at, None);
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

    let agent = f.agent(105, "incarnation-1", false);
    let outcome = ControlOutcome::Succeeded { agent };
    assert!(matches!(f.report(106, "running", FIRST, &claim, outcome.clone()), Err(Error::Lease)));
    assert!(matches!(f.report(106, "running", SECOND, "wrong-claim", outcome.clone()), Err(Error::ControlClaim)));
    let result = f.report(106, "running", SECOND, &claim, outcome.clone()).unwrap();
    assert_eq!(f.report(107, "running", SECOND, &claim, outcome).unwrap(), result);
    assert!(matches!(f.report(107, "running", SECOND, &claim, ControlOutcome::Uncertain { code: "unknown".into() }), Err(Error::ControlOutcome)));
    assert_eq!(f.lookup(200, "running"), result);
}

#[test]
fn exact_stop_cannot_claim_or_retire_a_replacement() {
    let f = Fixture::new();
    f.enroll(100, FIRST, None, catalogue()).unwrap();
    let original = f.agent(101, "incarnation-1", false);
    let stop = |agent: &AgentView| ControlCommand::Stop { name: agent.name.clone(), incarnation: agent.incarnation.clone() };
    f.submit(101, "first-stop", stop(&original)).unwrap();
    f.agent(102, &original.incarnation, true);
    let next = f.agent(102, "incarnation-2", false);
    assert!(f.receive(103, FIRST).unwrap().is_none());
    assert_eq!(f.lookup(103, "first-stop").outcome, Some(ControlOutcome::Failed { code: "stale_agent".into() }));
    assert!(matches!(f.submit(103, "stale-stop", stop(&original)), Err(Error::Agent)));

    f.submit(104, "next-stop", stop(&next)).unwrap();
    let claim = f.receive(104, FIRST).unwrap().unwrap().claim_id.unwrap();
    assert!(matches!(f.report(105, "next-stop", FIRST, &claim, ControlOutcome::Succeeded { agent: original }), Err(Error::Agent)));
    let result = f.report(105, "next-stop", FIRST, &claim, ControlOutcome::Succeeded { agent: next.clone() }).unwrap();
    assert_eq!(result.status, ControlStatus::Succeeded);
    assert_eq!(f.at(106, |conn, _| store::active_agent(conn, &next.name)).unwrap(), None);
    let newer = f.agent(106, "incarnation-3", false);
    assert_eq!(f.submit(107, "next-stop", stop(&next)).unwrap(), result);
    assert_eq!(f.at(107, |conn, _| store::active_agent(conn, &next.name)).unwrap(), Some(newer));
    assert!(matches!(f.receive(130, FIRST), Err(Error::Lease)));
    assert!(matches!(f.enroll(130, FIRST, None, catalogue()), Err(Error::Lease)));
}
