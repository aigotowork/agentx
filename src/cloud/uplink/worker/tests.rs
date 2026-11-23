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
    fs::create_dir(&root).unwrap();
    let runtime = tokio::runtime::Builder::new_multi_thread().worker_threads(2).enable_all().build().unwrap();
    let listener = runtime.block_on(tokio::net::TcpListener::bind("127.0.0.1:0")).unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    let admin_token = "a".repeat(32);
    let token = "b".repeat(32);
    let catalog = catalog::Catalog::open(&temp.path().join("deployment"), origin.clone()).unwrap();
    let app = router(Deployment { admin_hash: catalog::hash(&admin_token), catalog: Arc::new(Mutex::new(catalog)), calls: Arc::new(tokio::sync::Semaphore::new(16)) });
    runtime.spawn(async move { axum::serve(listener, app).await.unwrap() });

    let admin = Http::new(&format!("{origin}/api/v1/admin"), &admin_token).unwrap();
    let provisioned: serde_json::Value = admin.request("PUT", "/scopes/team", Some(&json!({ "request_id": "provision", "token": token }))).unwrap();
    let generation = provisioned["generation"].as_str().unwrap();
    let config = Config { url: format!("{origin}/scopes/team"), token, host: Some("work".into()), workspaces: Exports::from([("default".into(), root.clone())]) };
    let native_generation = Uuid::new_v4().to_string();
    let host = lease(host_scope.clone(), &native_generation);
    let stop = AtomicBool::new(false);
    let mut worker = test_worker(&host, &stop, &config);
    let installation = worker.wire.installation.clone();
    let capabilities = || admin.request::<HostCapabilities>("GET", "/scopes/team/hosts/work/capabilities", None).unwrap();
    let post = |id, parent, directory, alias| {
        let body = json!({ "generation": generation, "request_id": id, "host": "work", "installation_id": installation,
            "command": { "kind": "add_workspace", "parent": parent, "directory": directory, "alias": alias, "create_directory": true } });
        admin.request::<ControlReceipt>("POST", "/scopes/team/operations", Some(&body))
    };
    let receipt = |id| admin.request::<ControlReceipt>("GET", &format!("/scopes/team/operations/{id}"), None).unwrap();

    renew_only(&mut worker);
    let initial = capabilities();
    assert!(initial.online);
    assert_eq!(initial.workspace_management, Some(true));
    assert_eq!(initial.workspaces[0].managed_parent, Some(true));
    assert_eq!(post("register", "default", "project", "project").unwrap().status, ControlStatus::Queued);
    thread::scope(|effects| {
        let mut active = None;
        assert_eq!(worker.cycle(effects, &mut active).unwrap(), Duration::ZERO);
        assert!(active.is_none());
        assert!(root.join("project").is_dir());
        assert_eq!(receipt("register").status, ControlStatus::Running);
        assert!(!capabilities().workspaces.iter().any(|item| item.alias == "project"));
        worker.cycle(effects, &mut active).unwrap();
    });
    let added = receipt("register");
    assert_eq!(added.status, ControlStatus::Succeeded);
    assert_eq!(added.outcome, Some(ControlOutcome::WorkspaceAdded { alias: "project".into() }));
    assert_eq!(post("register", "default", "project", "project").unwrap(), added);
    let project = capabilities().workspaces.into_iter().find(|item| item.alias == "project").unwrap();
    assert_ne!(project.managed_parent, Some(true));
    assert_eq!(post("duplicate", "default", "unexpected", "project").unwrap_err().downcast_ref::<HttpError>().unwrap().code, "workspace_alias_taken");
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
    for alias in ["project", "restart-pending"] {
        assert!(advertised.iter().any(|item| item.alias == alias));
    }
    thread::scope(|effects| restarted.cycle(effects, &mut None)).unwrap();
    assert_eq!(receipt("restart-pending").outcome, Some(ControlOutcome::WorkspaceAdded { alias: "restart-pending".into() }));
    assert_eq!(receipt("register"), added);
    assert_eq!(receipt("stale-cloud").outcome, Some(ControlOutcome::Failed { code: "host_lease_lost".into() }));
    for path in ["unexpected", "never-local", "never-cloud"] {
        assert!(!root.join(path).exists());
    }
}
