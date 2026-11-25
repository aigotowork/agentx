//! Private, scope-owned enrollment and instance-bound passive diagnostics.
use std::collections::BTreeMap;
use std::fs::{File, OpenOptions};
use std::io::{ErrorKind, Read, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};

#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Config {
    pub namespace_url: String,
    pub token: String,
    pub host: Option<String>,
    pub project_roots: BTreeMap<String, PathBuf>,
}

const FILE: &str = "cloud.toml";
const LIMIT: u64 = 65536;

impl Config {
    #[cfg(target_os = "linux")]
    pub(crate) fn host_name(&self) -> String {
        self.host.clone().unwrap_or_else(super::client::default_host)
    }

    pub(crate) fn load(root: &Path) -> Result<Option<Self>> {
        let mut opts = OpenOptions::new();
        opts.read(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            opts.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
        }
        let file = match opts.open(root.join(FILE)) {
            Ok(file) => file,
            Err(e) if e.kind() == ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e).context("open scope cloud.toml"),
        };
        let metadata = file.metadata()?;
        ensure!(metadata.is_file(), "cloud.toml must be a regular file");
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            ensure!(metadata.uid() == unsafe { libc::geteuid() } && metadata.mode() & 0o077 == 0, "cloud.toml must be private to the daemon account (chmod 600)");
        }
        let mut contents = String::new();
        file.take(LIMIT + 1).read_to_string(&mut contents)?;
        ensure!(contents.len() as u64 <= LIMIT, "cloud.toml exceeds size limit");
        // Parser diagnostics may quote the line containing a bearer.
        let config: Self = toml::from_str(&contents).map_err(|_| anyhow::anyhow!("invalid cloud.toml fields or syntax"))?;
        config.validate()?;
        Ok(Some(config))
    }

    fn validate(&self) -> Result<()> {
        let uri: axum::http::Uri = self.namespace_url.parse().map_err(|_| anyhow::anyhow!("invalid cloud namespace URL"))?;
        let host = uri.host().context("cloud namespace URL requires a host")?;
        let local = host.eq_ignore_ascii_case("localhost") || host.trim_matches(['[', ']']).parse::<std::net::IpAddr>().is_ok_and(|ip| ip.is_loopback());
        ensure!(uri.scheme_str() == Some("https") || local && uri.scheme_str() == Some("http"), "cloud namespace URL requires HTTPS (loopback HTTP permitted)");
        ensure!(uri.query().is_none() && !self.namespace_url.contains('#') && uri.authority().is_some_and(|a| !a.as_str().contains('@')), "cloud URL may not contain credentials, a fragment, or a query");
        let id = uri.path().trim_end_matches('/').strip_prefix("/namespaces/").context("expected a /namespaces/ID URL")?;
        super::validate_id(id).map_err(|_| anyhow::anyhow!("invalid scope ID in cloud URL"))?;
        super::validate_token(&self.token).map_err(|_| anyhow::anyhow!("invalid namespace token format"))?;
        if let Some(host) = &self.host {
            ensure!(host.len() <= 253 && host.as_bytes().first().is_some_and(u8::is_ascii_alphanumeric) && host.bytes().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || b"-_.".contains(&c)), "invalid host name");
        }
        ensure!(!self.project_roots.is_empty(), "cloud.toml needs at least one project root");
        ensure!(self.project_roots.len() <= super::control::types::MAX_WORKSPACES, "cloud.toml exceeds the project root limit");
        for (alias, path) in &self.project_roots {
            super::validate_id(alias).map_err(|_| anyhow::anyhow!("invalid project root alias"))?;
            ensure!(path.is_absolute(), "project root paths must be absolute");
        }
        Ok(())
    }
}

#[derive(clap::Args)]
pub(crate) struct Connect {
    #[arg(long)]
    pub scope: Option<PathBuf>,
    /// Save configuration only; let a service manager start or restart the daemon.
    #[arg(long)]
    pub configure_only: bool,
    /// HTTPS namespace URL, including /namespaces/ID.
    #[arg(long)]
    namespace_url: String,
    /// File containing the namespace token.
    #[arg(long)]
    token_file: PathBuf,
    #[arg(long)]
    host: Option<String>,
    /// Exposed project root alias=path; defaults to default=current directory.
    #[arg(long = "project-root")]
    project_roots: Vec<String>,
}

impl Connect {
    pub(crate) fn save(self, root: &Path, cwd: &Path) -> Result<()> {
        let mut token = String::new();
        File::open(&self.token_file).context("open namespace token file")?.take(15).read_to_string(&mut token)?;
        ensure!(token.len() <= 14, "invalid namespace token length");
        let mut project_roots = BTreeMap::new();
        for item in self.project_roots {
            let (name, path) = item.split_once('=').context("expected --project root alias=path")?;
            ensure!(!path.is_empty(), "missing project root path");
            let path = cwd.join(path).canonicalize().context("resolve project root directory")?;
            ensure!(path.is_dir(), "project root must be a directory");
            ensure!(project_roots.insert(name.to_owned(), path).is_none(), "duplicate project root alias");
        }
        if project_roots.is_empty() {
            project_roots.insert("default".into(), cwd.canonicalize()?);
        }
        let config = Config { namespace_url: self.namespace_url.trim_end_matches('/').into(), token: token.trim_end_matches(['\n', '\r']).into(), host: self.host, project_roots };
        config.validate()?;
        let data = toml::to_string(&config)?;
        ensure!(data.len() as u64 <= LIMIT, "configuration exceeds size limit");
        write_atomic(&root.join(FILE), data.as_bytes())
    }
}

fn write_atomic(dest: &Path, data: &[u8]) -> Result<()> {
    let parent = dest.parent().context("missing parent directory")?;
    let mut tmp = tempfile::NamedTempFile::new_in(parent)?;
    tmp.write_all(data)?;
    tmp.as_file().sync_all()?;
    tmp.persist(dest)?;
    #[cfg(unix)]
    File::open(parent)?.sync_all()?;
    Ok(())
}

#[derive(Serialize, Deserialize)]
struct ConnectionStatus {
    instance: String,
    state: String,
    detail: Option<String>,
}

pub(crate) fn publish_status(root: &Path, instance: &str, state: &str, detail: Option<&str>) -> Result<()> {
    let status = ConnectionStatus { instance: instance.into(), state: state.into(), detail: detail.map(str::to_owned) };
    write_atomic(&root.join(".daemon/cloud-status.json"), &serde_json::to_vec(&status)?)
}

pub(crate) fn describe(root: &Path, instance: Option<&str>) -> Option<String> {
    let config = match Config::load(root) {
        Ok(Some(config)) => config,
        Ok(None) => return None,
        Err(error) => return Some(format!("configuration unavailable ({error})")),
    };
    let status = instance.and_then(|id| {
        let file = File::open(root.join(".daemon/cloud-status.json")).ok()?;
        let status: ConnectionStatus = serde_json::from_reader(file.take(LIMIT)).ok()?;
        (status.instance == id).then_some(status)
    });
    let detail = match status {
        Some(status) => match status.detail {
            Some(reason) => format!("{} ({reason})", status.state),
            None => status.state,
        },
        None => "configured; no current connection report".to_owned(),
    };
    Some(format!("configured endpoint={}\nCloud daemon: {detail}", config.namespace_url))
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn enrollment_secrets_are_private_and_stale_status_is_never_current() -> Result<()> {
        let tmp = tempfile::tempdir()?;
        let root = tmp.path();
        let key = "a".repeat(12);
        let input = root.join("credential");
        std::fs::write(&input, &key)?;
        Connect { configure_only: false, scope: None, namespace_url: "https://example.com/namespaces/team".into(), token_file: input, host: Some("work".into()), project_roots: vec![] }.save(root, root)?;
        let config = Config::load(root)?.unwrap();
        assert_eq!(config.token, key);
        assert_eq!(config.project_roots["default"], root.canonicalize()?);
        assert_eq!(std::fs::metadata(root.join(FILE))?.permissions().mode() & 0o077, 0);
        std::fs::create_dir(root.join(".daemon"))?;
        publish_status(root, "old", "online", None)?;
        assert!(!describe(root, Some("new")).unwrap().contains("online"));
        assert!(describe(root, Some("old")).unwrap().contains("online"));
        std::fs::set_permissions(root.join(FILE), std::fs::Permissions::from_mode(0o644))?;
        assert!(Config::load(root).is_err());
        std::fs::set_permissions(root.join(FILE), std::fs::Permissions::from_mode(0o600))?;
        std::fs::write(root.join(FILE), format!("token = {key}"))?;
        assert!(!describe(root, None).unwrap().contains(&key));
        Ok(())
    }
}
