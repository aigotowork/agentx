//! Exact Linux supervisor facts, shared by authenticated IPC and control work.
//! A durable marker is evidence of an attempted launch, not of readiness.
use std::{
    fs::{self, File},
    io,
    os::fd::{AsRawFd, FromRawFd, OwnedFd},
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
#[cfg(test)]
use uuid::Uuid;

use super::{Actor, AgentView, private_directory, read_json_file};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(in crate::cloud) struct OwnedProcess {
    pub agent: AgentView,
    pub pid: u32,
    pub process_identity: String,
    pub session_id: String,
    local_generation: String,
    cloud_generation: String,
    installation_id: String,
}

impl OwnedProcess {
    pub(in crate::cloud) fn matches_binding(&self, local_generation: &str, cloud_generation: &str, installation_id: &str) -> bool {
        self.local_generation == local_generation && self.cloud_generation == cloud_generation && self.installation_id == installation_id
    }
}

#[derive(Clone, Deserialize, Serialize)]
pub(super) struct Record {
    pub actor: Actor,
    pub local_generation: String,
    pub pid: u32,
    pub process_identity: String,
    pub session: Option<String>,
    pub ready: bool,
}

fn directory(root: &Path) -> PathBuf {
    root.join("cloud-client").join("supervisors")
}

fn path(root: &Path, incarnation: &str) -> Result<PathBuf> {
    ensure!(crate::cloud::identifiers::valid_incarnation(incarnation), "invalid local cloud Agent incarnation");
    Ok(directory(root).join(format!("{incarnation}.json")))
}

impl Record {
    pub(super) fn start(actor: Actor, local_generation: String) -> Result<Self> {
        let pid = std::process::id();
        let process_identity = crate::sys::process::identity(pid).context("unable to resolve cloud supervisor process identity")?;
        Ok(Self { actor, local_generation, pid, process_identity, session: None, ready: false })
    }

    pub(super) fn persist(&self, root: &Path) -> Result<()> {
        let directory = directory(root);
        private_directory(&directory)?;
        let mut pending = tempfile::NamedTempFile::new_in(&directory)?;
        serde_json::to_writer(&mut pending, self)?;
        pending.as_file().sync_all()?;
        pending.persist(path(root, &self.actor.agent.incarnation)?)?;
        File::open(directory)?.sync_all()?;
        Ok(())
    }

    pub(super) fn live(&self) -> bool {
        crate::sys::process::identity(self.pid).as_deref() == Some(self.process_identity.as_str())
            && fs::read_to_string(format!("/proc/{}/stat", self.pid)).ok().and_then(|stat| stat.rsplit_once(") ")?.1.bytes().next()).is_some_and(|state| !matches!(state, b'Z' | b'X' | b'x'))
    }

    pub(super) fn owns(&self, pid: u32, identity: &str) -> bool {
        self.pid == pid && self.process_identity == identity && self.live()
    }
}

pub(super) fn record(root: &Path, incarnation: &str) -> Result<Option<Record>> {
    match read_json_file::<Record>(&path(root, incarnation)?) {
        Ok(record) => {
            ensure!(record.actor.agent.incarnation == incarnation, "exact supervisor record does not match its Agent incarnation");
            Ok(Some(record))
        }
        Err(error) if error.downcast_ref::<io::Error>().is_some_and(|e| e.kind() == io::ErrorKind::NotFound) => Ok(None),
        Err(error) => Err(error),
    }
}

pub(super) fn records(root: &Path) -> Result<Vec<Record>> {
    let entries = match fs::read_dir(directory(root)) {
        Ok(entries) => entries,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e.into()),
    };
    let mut records = Vec::new();
    for entry in entries {
        let entry = entry?;
        let name = entry.file_name();
        if let Some(incarnation) = name.to_str().and_then(|name| name.strip_suffix(".json"))
            && crate::cloud::identifiers::valid_incarnation(incarnation)
            && let Some(record) = record(root, incarnation)?
            && record.live()
        {
            records.push(record);
        }
    }
    Ok(records)
}

pub(in crate::cloud) fn owned_process(root: &Path, incarnation: &str) -> Result<Option<OwnedProcess>> {
    Ok(record(root, incarnation)?.filter(|r| r.ready && r.live()).and_then(|r| {
        r.session.map(|session_id| OwnedProcess {
            agent: r.actor.agent,
            pid: r.pid,
            process_identity: r.process_identity,
            session_id,
            local_generation: r.local_generation,
            cloud_generation: r.actor.generation,
            installation_id: r.actor.installation_id,
        })
    }))
}

fn open_pidfd(pid: u32) -> Result<Option<OwnedFd>> {
    // SAFETY: Linux pidfd_open returns a freshly owned descriptor or -1.
    let raw = unsafe { libc::syscall(libc::SYS_pidfd_open, pid as libc::pid_t, 0u32) };
    if raw < 0 {
        let error = io::Error::last_os_error();
        return if error.raw_os_error() == Some(libc::ESRCH) { Ok(None) } else { Err(error.into()) };
    }
    // SAFETY: descriptor came from a successful pidfd_open and has one owner.
    Ok(Some(unsafe { OwnedFd::from_raw_fd(raw as _) }))
}

pub(in crate::cloud) fn exited_process(root: &Path, agent: &AgentView, expected: (&str, &str, &str)) -> Result<bool> {
    let Some(record) = record(root, &agent.incarnation)? else { return Ok(false) };
    if !record.ready || &record.actor.agent != agent || (record.local_generation.as_str(), record.actor.generation.as_str(), record.actor.installation_id.as_str()) != expected {
        return Ok(false);
    }
    let Some(fd) = open_pidfd(record.pid)? else { return Ok(true) };
    if let Some(current) = crate::sys::process::identity(record.pid)
        && record.process_identity != current
    {
        // The PID exists but belongs to another boot or process incarnation.
        return Ok(true);
    }
    let mut descriptor = libc::pollfd { fd: fd.as_raw_fd(), events: libc::POLLIN, revents: 0 };
    // SAFETY: one valid owned descriptor; this is a nonblocking observation.
    let result = unsafe { libc::poll(&mut descriptor, 1, 0) };
    if result < 0 {
        return Err(io::Error::last_os_error().into());
    }
    Ok(result > 0 && descriptor.revents & libc::POLLIN != 0)
}

pub(in crate::cloud) fn stop_exact(root: &Path, agent: &AgentView) -> Result<bool> {
    let Some(record) = record(root, &agent.incarnation)? else { return Ok(false) };
    ensure!(&record.actor.agent == agent, "stop target differs from exact cloud supervisor record");
    if !record.live() {
        return Ok(false);
    }
    let Some(fd) = open_pidfd(record.pid)? else { return Ok(false) };
    if !record.live() {
        return Ok(false);
    }
    // SAFETY: only the pinned process receives TERM, even if the PID is reused.
    if unsafe { libc::syscall(libc::SYS_pidfd_send_signal, fd.as_raw_fd(), libc::SIGTERM, std::ptr::null::<libc::siginfo_t>(), 0u32) } < 0 {
        let error = io::Error::last_os_error();
        return if error.raw_os_error() == Some(libc::ESRCH) { Ok(false) } else { Err(error.into()) };
    }
    let deadline = Instant::now() + Duration::from_secs(25);
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        ensure!(!remaining.is_zero(), "exact cloud supervisor received TERM but exit is unconfirmed");
        let mut descriptor = libc::pollfd { fd: fd.as_raw_fd(), events: libc::POLLIN, revents: 0 };
        // SAFETY: poll reads this valid one-element descriptor array.
        let result = unsafe { libc::poll(&mut descriptor, 1, remaining.as_millis().max(1) as i32) };
        if result > 0 {
            ensure!(descriptor.revents & libc::POLLIN != 0, "exact cloud supervisor exit observation failed");
            return Ok(true);
        }
        if result < 0 && io::Error::last_os_error().kind() != io::ErrorKind::Interrupted {
            return Err(io::Error::last_os_error().into());
        }
    }
}

/// Take a parent chain from the same live peer incarnation. Each step checks
/// that the observed parent predates the child; a recycled PID cannot become
/// an ancestor. Failure to inspect a vanished peer closes actor admission.
pub(super) fn lineage(pid: u32, identity: &str) -> Result<Vec<(u32, String)>> {
    use crate::sys::process::identity as process_identity;
    let mut result = Vec::new();
    let mut next = pid;
    let mut expected = identity.to_owned();
    for _ in 0..128 {
        let stat = fs::read_to_string(format!("/proc/{next}/stat")).context("read authenticated process lineage")?;
        let mut fields = stat.rsplit_once(") ").context("invalid authenticated process stat")?.1.split_whitespace();
        let parent: u32 = fields.nth(1).context("missing authenticated process parent")?.parse()?;
        let ticks: u64 = fields.nth(17).context("missing authenticated process incarnation")?.parse()?;
        ensure!(expected.rsplit(':').next().and_then(|value| value.parse().ok()) == Some(ticks), "authenticated process incarnation changed");
        result.push((next, expected));
        if parent == 0 {
            return Ok(result);
        }
        let parent_identity = process_identity(parent).context("authenticated process parent disappeared")?;
        let parent_ticks = parent_identity.rsplit(':').next().and_then(|value| value.parse::<u64>().ok()).context("invalid process parent incarnation")?;
        ensure!(parent_ticks <= ticks, "authenticated process parent changed");
        next = parent;
        expected = parent_identity;
    }
    anyhow::bail!("authenticated process lineage exceeds bound")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stop_targets_only_exact_live_incarnation_and_preserves_exit_evidence() {
        struct Child(std::process::Child);
        impl Drop for Child {
            fn drop(&mut self) {
                let _ = self.0.kill();
                let _ = self.0.wait();
            }
        }
        let mut child = Child(std::process::Command::new("/bin/sleep").arg("30").spawn().unwrap());
        let root = tempfile::tempdir().unwrap();
        let agent = AgentView { name: "worker".into(), host: "alpha".into(), incarnation: crate::cloud::identifiers::random_id(), workspace: super::super::AgentWorkspace::Unexposed };
        let actor = Actor { generation: Uuid::new_v4().to_string(), installation_id: Uuid::new_v4().to_string(), agent: agent.clone() };
        let mut record = Record::start(actor, Uuid::new_v4().to_string()).unwrap();
        record.pid = child.0.id();
        record.process_identity = crate::sys::process::identity(record.pid).unwrap();
        record.persist(root.path()).unwrap();
        assert!(owned_process(root.path(), &agent.incarnation).unwrap().is_none());

        record.ready = true;
        record.session = Some("thread".into());
        record.persist(root.path()).unwrap();
        let stale = AgentView { incarnation: crate::cloud::identifiers::random_id(), ..agent.clone() };
        let expected = (&*record.local_generation, &*record.actor.generation, &*record.actor.installation_id);
        assert!(!stop_exact(root.path(), &stale).unwrap());
        assert!(!exited_process(root.path(), &agent, expected).unwrap());
        assert!(child.0.try_wait().unwrap().is_none());

        assert!(stop_exact(root.path(), &agent).unwrap());
        assert!(exited_process(root.path(), &agent, expected).unwrap());
        assert!(owned_process(root.path(), &agent.incarnation).unwrap().is_none());
        assert!(!exited_process(root.path(), &agent, ("stale-generation", expected.1, expected.2)).unwrap());
    }
}
