use std::fs;
use std::sync::{Arc, Mutex};

use serde_json::json;

use crate::cloud::{Deployment, catalog, router};
use crate::daemon::DaemonScope;

use super::*;

fn test_worker<'a>(lease: &'a ScopeLease, stop: &'a AtomicBool, config: &'a Config) -> Worker<'a> {
    let wire = Wire::new(config, config.host_name(), installation_id(lease.scope().root()).unwrap()).unwrap();
    let journal = Journal::open(lease.scope().state()).unwrap();
    Worker { lease, stop, config, wire, journal, status: Status { lease, last: None }, session: None }
}

fn lease(scope: DaemonScope, generation: &str) -> ScopeLease {
    let lease = ScopeLease::acquire(scope).unwrap();
    lease.database_generation().publish(generation.to_owned()).unwrap();
    lease
}

fn renew_only(worker: &mut Worker<'_>) {
    worker.stop.store(true, Ordering::Release);
    thread::scope(|effects| worker.cycle(effects, &mut None)).unwrap();
    worker.stop.store(false, Ordering::Release);
}

#[test]
fn workspace_registration_publishes_before_reporting_and_survives_host_restart() {
    let temp = tempfile::tempdir().unwrap();
    let host_scope = DaemonScope::prepare(temp.path().join("host")).unwrap();
    let root = temp.path().join("export");
    fs::create_dir_all(root.join("auto/.git")).unwrap();
    let runtime = tokio::runtime::Builder::new_multi_thread().worker_threads(2).enable_all().build().unwrap();
    let listener = runtime.block_on(tokio::net::TcpListener::bind("127.0.0.1:0")).unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    let admin_token = "a".repeat(12);
    let token = "b".repeat(12);
    let catalog = catalog::Catalog::open(&temp.path().join("deployment"), origin.clone()).unwrap();
    let app = router(Deployment { admin_hash: catalog::hash(&admin_token), catalog: Arc::new(Mutex::new(catalog)), calls: Arc::new(tokio::sync::Semaphore::new(16)) });
    runtime.spawn(async move { axum::serve(listener, app).await.unwrap() });

    let admin = Http::new(&format!("{origin}/api/v1/admin"), &admin_token).unwrap();
    let provisioned: serde_json::Value = admin.request("PUT", "/namespaces/team", Some(&json!({ "request_id": "provision", "token": token }))).unwrap();
    let generation = provisioned["generation"].as_str().unwrap();
    let config = Config { namespace_url: format!("{origin}/namespaces/team"), token, host: Some("work".into()), project_roots: ProjectRoots::from([("default".into(), root.clone())]) };
    let native_generation = Uuid::new_v4().to_string();
    let host = lease(host_scope.clone(), &native_generation);
    let stop = AtomicBool::new(false);
    let mut worker = test_worker(&host, &stop, &config);
    let installation = worker.wire.installation.clone();
    let capabilities = || admin.request::<HostCapabilities>("GET", "/namespaces/team/hosts/work/capabilities", None).unwrap();
    let post = |id, parent, directory, name| {
        let body = json!({ "generation": generation, "request_id": id, "host": "work", "installation_id": installation,
            "command": { "kind": "add_workspace", "parent": parent, "directory": directory, "name": name, "create_directory": true } });
        admin.request::<ControlReceipt>("POST", "/namespaces/team/operations", Some(&body))
    };
    let receipt = |id| admin.request::<ControlReceipt>("GET", &format!("/namespaces/team/operations/{id}"), None).unwrap();

    renew_only(&mut worker);
    let initial = capabilities();
    assert!(initial.online);
    assert_eq!(initial.workspace_management, Some(true));
    assert_eq!(initial.project_roots, ["default"]);
    assert_eq!(initial.workspaces.len(), 1);
    let auto_id = initial.workspaces[0].workspace_id.clone();
    assert_eq!(initial.workspaces[0].name, "auto");
    assert_eq!(post("overlay", "default", "auto", "project").unwrap().status, ControlStatus::Queued);
    thread::scope(|effects| {
        worker.cycle(effects, &mut None).unwrap();
        worker.cycle(effects, &mut None).unwrap();
    });
    assert_eq!(receipt("overlay").outcome, Some(ControlOutcome::WorkspaceAdded { workspace_id: auto_id.clone() }));
    assert_eq!(capabilities().workspaces.len(), 1);
    assert_eq!(capabilities().workspaces[0].name, "project");
    assert_eq!(post("register", "default", "project", "project").unwrap().status, ControlStatus::Queued);
    thread::scope(|effects| {
        let mut active = None;
        assert_eq!(worker.cycle(effects, &mut active).unwrap(), Duration::ZERO);
        assert!(active.is_none());
        assert!(root.join("project").is_dir());
        assert_eq!(receipt("register").status, ControlStatus::Running);
        assert_eq!(capabilities().workspaces.len(), 1);
        worker.cycle(effects, &mut active).unwrap();
    });
    let added = receipt("register");
    assert_eq!(added.status, ControlStatus::Succeeded);
    let Some(ControlOutcome::WorkspaceAdded { workspace_id: new_id }) = &added.outcome else { panic!("expected project registration: {added:?}") };
    assert_ne!(new_id, &auto_id);
    assert_eq!(post("register", "default", "project", "project").unwrap(), added);
    assert_eq!(capabilities().workspaces.iter().filter(|item| item.name == "project").count(), 2);
    assert!(post("nested", "project", "unexpected", "nested").is_err());

    // A valid cloud claim does not authorize a local action after local lease freshness expires.
    post("stale-local", "default", "never-local", "stale-local").unwrap();
    let session = worker.session.as_mut().unwrap();
    session.renew_at = Instant::now() + RENEW_EVERY;
    session.fresh_until = Instant::now();
    thread::scope(|effects| {
        let mut active = None;
        worker.cycle(effects, &mut active).unwrap();
        worker.cycle(effects, &mut active).unwrap();
    });
    assert_eq!(receipt("stale-local").outcome, Some(ControlOutcome::Failed { code: "lease_not_current".into() }));

    worker.session.as_mut().unwrap().renew_at = Instant::now();
    renew_only(&mut worker);
    post("restart-pending", "default", "restart-pending", "restart-pending").unwrap();
    thread::scope(|effects| worker.cycle(effects, &mut None)).unwrap();
    assert_eq!(receipt("restart-pending").status, ControlStatus::Running);
    post("stale-cloud", "default", "never-cloud", "stale-cloud").unwrap();
    drop(worker);
    drop(host);
    let host = lease(host_scope, &native_generation);
    let mut restarted = test_worker(&host, &stop, &config);
    renew_only(&mut restarted);
    assert_eq!(capabilities().installation_id, installation);
    let advertised = capabilities().workspaces;
    for id in [&auto_id, new_id] {
        assert!(advertised.iter().any(|item| item.workspace_id == *id && item.name == "project"));
    }
    let pending = advertised.iter().find(|item| item.name == "restart-pending").unwrap();
    thread::scope(|effects| restarted.cycle(effects, &mut None)).unwrap();
    assert_eq!(receipt("restart-pending").outcome, Some(ControlOutcome::WorkspaceAdded { workspace_id: pending.workspace_id.clone() }));
    assert_eq!(receipt("register"), added);
    assert_eq!(receipt("stale-cloud").outcome, Some(ControlOutcome::Failed { code: "host_lease_lost".into() }));
    for path in ["unexpected", "never-local", "never-cloud"] {
        assert!(!root.join(path).exists());
    }
}

#[test]
fn launch_success_requires_the_registered_project_and_exact_incarnation() {
    let mut agent = AgentView { name: "worker".into(), host: "alpha".into(), incarnation: "Incarnation1".into(), workspace: AgentWorkspace::Project { workspace_id: "WS0000000001".into() } };
    assert!(matches_launch(&agent, "alpha", "Incarnation1", Some("worker"), "WS0000000001"));
    assert!(!matches_launch(&agent, "alpha", "Incarnation2", None, "WS0000000001"));
    assert!(!matches_launch(&agent, "alpha", "Incarnation1", None, "WS0000000002"));
    agent.workspace = AgentWorkspace::Global;
    assert!(!matches_launch(&agent, "alpha", "Incarnation1", None, "WS0000000001"));
}
