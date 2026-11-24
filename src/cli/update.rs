//! Daily, non-blocking checks for the latest published stable release.

use crate::paths::{FLAGS_DIR, agentx_path, atomic_write};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, SystemTime};

const CHECK_INTERVAL: Duration = Duration::from_secs(86400);
const RETRY_INTERVAL: Duration = Duration::from_secs(900);

pub(crate) fn flag_path() -> PathBuf {
    agentx_path(&[FLAGS_DIR, "update_check"])
}

fn parse_version(v: &str) -> Option<(u32, u32, u32)> {
    let mut parts = v.trim().strip_prefix('v').unwrap_or(v.trim()).split('.');
    let version = (parts.next()?.parse().ok()?, parts.next()?.parse().ok()?, parts.next()?.parse().ok()?);
    if parts.next().is_some() { None } else { Some(version) }
}

fn is_fresh(path: &Path, interval: Duration) -> bool {
    path.metadata().and_then(|m| m.modified()).is_ok_and(|mtime| SystemTime::now().duration_since(mtime).unwrap_or(Duration::ZERO) < interval)
}

// Separate attempt timestamp preserves known notices on network failures,
// without spawning a checker on every subsequent CLI command.
fn claim_check(flag: &Path) -> bool {
    let attempt = flag.with_extension("attempt");
    if is_fresh(&attempt, RETRY_INTERVAL) {
        return false;
    }
    if let Some(parent) = attempt.parent()
        && fs::create_dir_all(parent).is_err()
    {
        return false;
    }
    if attempt.exists() && fs::remove_file(&attempt).is_err() {
        return false;
    }
    fs::OpenOptions::new().write(true).create_new(true).open(attempt).is_ok()
}

fn spawn_background_check(flag: &Path, current: &str) {
    if !claim_check(flag) {
        return;
    }
    let Ok(exe) = std::env::current_exe() else {
        return;
    };
    let mut cmd = Command::new(exe);
    crate::sys::process::detach_session(&mut cmd);
    let _ = cmd.arg("__update-check").arg(flag).arg(current).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).spawn();
}

pub(crate) fn run_background_check(flag: &Path, current: &str) {
    if let Some(latest) = fetch_latest_version() {
        write_update_cache(flag, current, &latest);
    }
}

fn write_update_cache(flag: &Path, current: &str, latest: &str) {
    let (Some(current), Some(remote)) = (parse_version(current), parse_version(latest)) else {
        return;
    };
    atomic_write(flag, if remote > current { latest } else { "" });
}

// Installer assets belong to published releases, not arbitrary Git tags.
fn fetch_latest_version() -> Option<String> {
    let latest = fetch_release_redirect().or_else(fetch_release_api);
    #[cfg(windows)]
    {
        latest.or_else(fetch_release_powershell)
    }
    #[cfg(not(windows))]
    latest
}

fn fetch_release_api() -> Option<String> {
    let mut cmd = Command::new("curl");
    hide_console(&mut cmd);
    let output = cmd.args(["-fsSL", "--max-time", "8", "-H", "Accept: application/vnd.github+json", "https://api.github.com/repos/aigotowork/agentx/releases/latest"]).stdin(Stdio::null()).output().ok()?;
    if !output.status.success() {
        return None;
    }
    published_version(&output.stdout)
}

// GitHub's latest-release redirect shares the installer's published-release
// semantics and remains usable when the REST API rate limit is exhausted.
fn fetch_release_redirect() -> Option<String> {
    let mut cmd = Command::new("curl");
    hide_console(&mut cmd);
    let output = cmd.args(["-fsSIL", "--max-time", "8", "-o", if cfg!(windows) { "NUL" } else { "/dev/null" }, "-w", "%{url_effective}", "https://github.com/aigotowork/agentx/releases/latest"]).stdin(Stdio::null()).output().ok()?;
    if !output.status.success() {
        return None;
    }
    version_from_release_url(String::from_utf8_lossy(&output.stdout).trim())
}

fn version_from_release_url(url: &str) -> Option<String> {
    let version = url.strip_prefix("https://github.com/aigotowork/agentx/releases/tag/v")?;
    parse_version(version)?;
    Some(version.into())
}

#[cfg(windows)]
fn fetch_release_powershell() -> Option<String> {
    let mut cmd = Command::new(windows_installer_program());
    hide_console(&mut cmd);
    let output = cmd.args(["-NoProfile", "-Command", "$ErrorActionPreference='Stop'; [Net.ServicePointManager]::SecurityProtocol = [Net.ServicePointManager]::SecurityProtocol -bor [Net.SecurityProtocolType]::Tls12; $r=Invoke-RestMethod -TimeoutSec 8 'https://api.github.com/repos/aigotowork/agentx/releases/latest'; $r | Select-Object tag_name,draft,prerelease | ConvertTo-Json -Compress"])
        .env_remove("PSModulePath").stdin(Stdio::null()).output().ok()?;
    if !output.status.success() {
        return None;
    }
    published_version(&output.stdout)
}

fn published_version(body: &[u8]) -> Option<String> {
    let release: serde_json::Value = serde_json::from_slice(body).ok()?;
    if release.get("draft").and_then(|v| v.as_bool()) == Some(true) || release.get("prerelease").and_then(|v| v.as_bool()) == Some(true) {
        return None;
    }
    let version = release.get("tag_name")?.as_str()?.strip_prefix('v')?.to_string();
    parse_version(&version)?;
    Some(version)
}

fn hide_console(cmd: &mut Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
    }
    #[cfg(not(windows))]
    let _ = cmd;
}

#[derive(Clone, Debug)]
pub struct UpdateInfo {
    pub current: String,
    pub latest: String,
    pub available: bool,
}

pub fn fetch_update_info() -> anyhow::Result<UpdateInfo> {
    let current = env!("CARGO_PKG_VERSION").to_string();
    let latest = fetch_latest_version().ok_or_else(|| anyhow::anyhow!("Could not fetch the latest published release from GitHub"))?;
    let current_version = parse_version(&current).ok_or_else(|| anyhow::anyhow!("Invalid current version: {current}"))?;
    let latest_version = parse_version(&latest).ok_or_else(|| anyhow::anyhow!("Invalid release version: {latest}"))?;
    Ok(UpdateInfo { current, latest, available: latest_version > current_version })
}

/// GitHub updates may replace standalone executables only. Legacy installation
/// metadata is read locally so we never overwrite files owned by another tool.
pub(crate) fn ensure_standalone_install(exe: &Path) -> anyhow::Result<()> {
    ensure_standalone_install_with_uv_dirs(exe, &uv_tool_dirs())
}

fn ensure_standalone_install_with_uv_dirs(exe: &Path, uv_dirs: &[PathBuf]) -> anyhow::Result<()> {
    let resolved = fs::canonicalize(exe)?;
    let prefix = resolved.parent().and_then(Path::parent);
    let managed = resolved.components().any(|part| part.as_os_str() == "Cellar") || prefix.is_some_and(|path| path.join("pyvenv.cfg").is_file()) || uv_owns_exe(&resolved, uv_dirs) || pip_owns_exe(&resolved);
    anyhow::ensure!(!managed, "This AgentX executable is managed by another installer: {}. Install a standalone copy from https://github.com/aigotowork/agentx/releases/latest and switch your PATH to it.", resolved.display());
    Ok(())
}

fn uv_tool_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(dir) = std::env::var_os("UV_TOOL_DIR").filter(|v| !v.is_empty()) {
        dirs.push(dir.into());
    }
    if let Some(appdata) = std::env::var_os("APPDATA") {
        dirs.push(PathBuf::from(&appdata).join("uv/tools"));
        dirs.push(PathBuf::from(appdata).join("uv/data/tools"));
    }
    if let Some(data) = std::env::var_os("XDG_DATA_HOME") {
        dirs.push(PathBuf::from(data).join("uv/tools"));
    }
    if let Some(home) = dirs::home_dir() {
        dirs.push(home.join(".local/share/uv/tools"));
    }
    dirs
}

fn same_path(a: &Path, b: &Path) -> bool {
    matches!((fs::canonicalize(a), fs::canonicalize(b)), (Ok(a), Ok(b)) if a == b)
}

// uv may copy the executable to a separate bin directory. Its receipt claims
// the destination even when the bytes subsequently changed.
fn uv_owns_exe(exe: &Path, dirs: &[PathBuf]) -> bool {
    dirs.iter().any(|dir| {
        let Ok(receipt) = fs::read_to_string(dir.join("agentx/uv-receipt.toml")) else {
            return false;
        };
        let Ok(receipt) = toml::from_str::<toml::Value>(&receipt) else {
            return false;
        };
        receipt
            .get("tool")
            .and_then(|v| v.get("entrypoints"))
            .and_then(|v| v.as_array())
            .is_some_and(|entries| entries.iter().any(|entry| entry.get("from").and_then(|v| v.as_str()) == Some("agentx") && entry.get("install-path").and_then(|v| v.as_str()).is_some_and(|installed| same_path(exe, Path::new(installed)))))
    })
}

fn record_path(line: &str) -> Option<String> {
    if !line.starts_with('"') {
        return Some(line.split(',').next()?.to_string());
    }
    let mut chars = line[1..].chars().peekable();
    let mut path = String::new();
    while let Some(c) = chars.next() {
        if c == '"' {
            if chars.peek() == Some(&'"') {
                chars.next();
                path.push('"');
            } else {
                return (chars.next() == Some(',')).then_some(path);
            }
        } else {
            path.push(c);
        }
    }
    None
}

fn site_owns_exe(site: &Path, exe: &Path) -> bool {
    let Ok(entries) = fs::read_dir(site) else {
        return false;
    };
    entries.flatten().any(|entry| {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        name.starts_with("agentx-") && name.ends_with(".dist-info") && fs::read_to_string(entry.path().join("RECORD")).is_ok_and(|record| record.lines().filter_map(record_path).any(|p| same_path(&site.join(p), exe)))
    })
}

fn pip_owns_exe(exe: &Path) -> bool {
    let Some(scripts) = exe.parent() else { return false };
    let (Some(prefix), Some(name)) = (scripts.parent(), scripts.file_name().and_then(|n| n.to_str())) else { return false };
    if name != "bin" && !name.eq_ignore_ascii_case("scripts") {
        return false;
    }
    let mut sites = vec![prefix.join("Lib/site-packages"), prefix.join("site-packages")];
    for lib in [prefix.join("lib"), prefix.join("lib64")] {
        if let Ok(entries) = fs::read_dir(lib) {
            for python in entries.flatten() {
                sites.push(python.path().join("site-packages"));
                sites.push(python.path().join("dist-packages"));
            }
        }
    }
    sites.into_iter().any(|site| site_owns_exe(&site, exe))
}

pub(crate) fn windows_installer_program() -> String {
    for name in ["pwsh.exe", "powershell.exe"] {
        if let Some(program) = crate::runtime::terminal::which_bin(name) {
            let mut cmd = Command::new(&program);
            hide_console(&mut cmd);
            if cmd.args(["-NoProfile", "-Command", "exit 0"]).env_remove("PSModulePath").stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).status().is_ok_and(|s| s.success()) {
                return program;
            }
        }
    }
    "powershell.exe".into()
}

pub fn get_update_info() -> Option<(String, &'static str)> {
    if std::env::var_os("AGENTX_UPDATE_VERIFY").is_some() {
        return None;
    }
    let flag = flag_path();
    let current = env!("CARGO_PKG_VERSION");
    if !is_fresh(&flag, CHECK_INTERVAL) {
        spawn_background_check(&flag, current);
    }
    let latest = fs::read_to_string(&flag).ok()?.trim().to_string();
    if latest.is_empty() {
        return None;
    }
    if parse_version(current)? >= parse_version(&latest)? {
        atomic_write(&flag, "");
        return None;
    }
    Some((latest, "agentx update"))
}

pub fn get_update_notice() -> Option<String> {
    let (latest, _) = get_update_info()?;
    Some(format!("→ agentx v{latest} available — run `agentx update`"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stable_published_releases_only() {
        assert_eq!(published_version(br#"{"tag_name":"v0.7.29","draft":false,"prerelease":false}"#), Some("0.7.29".into()));
        for body in [br#"{"tag_name":"v0.8.0-rc1"}"#.as_slice(), br#"{"tag_name":"v0.8.0","prerelease":true}"#, br#"{"tag_name":"v0.8.0","draft":true}"#, br#"{"tag_name":"invalid"}"#, b"invalid JSON"] {
            assert_eq!(published_version(body), None);
        }
        assert_eq!(parse_version("1.2.3.4"), None);
        assert_eq!(version_from_release_url("https://github.com/aigotowork/agentx/releases/tag/v0.7.29"), Some("0.7.29".into()));
        assert_eq!(version_from_release_url("https://github.com/aigotowork/agentx/releases/tag/v0.8.0-rc1"), None);
    }

    #[test]
    fn cache_preserves_notice_on_invalid_fetch_and_clears_on_valid_upgrade() {
        let tmp = tempfile::tempdir().unwrap();
        let flag = tmp.path().join("flags/update_check");
        write_update_cache(&flag, "0.7.28", "0.7.29");
        let modified = flag.metadata().unwrap().modified().unwrap();
        for latest in ["", "invalid", "0.8.0-rc1"] {
            write_update_cache(&flag, "0.7.28", latest);
            assert_eq!(fs::read_to_string(&flag).unwrap(), "0.7.29");
            assert_eq!(flag.metadata().unwrap().modified().unwrap(), modified);
        }
        assert!(claim_check(&flag));
        assert!(!claim_check(&flag));
        assert_eq!(fs::read_to_string(&flag).unwrap(), "0.7.29");
        write_update_cache(&flag, "0.7.29", "0.7.29");
        assert_eq!(fs::read_to_string(&flag).unwrap(), "");
    }

    fn pip_fixture(prefix: &Path, scripts: &str, site: &str, record: &str) -> PathBuf {
        let exe = prefix.join(scripts).join(if scripts == "bin" { "agentx" } else { "agentx.exe" });
        let metadata = prefix.join(site).join("agentx-0.7.29.dist-info");
        fs::create_dir_all(exe.parent().unwrap()).unwrap();
        fs::create_dir_all(&metadata).unwrap();
        fs::write(&exe, b"binary").unwrap();
        fs::write(metadata.join("RECORD"), record).unwrap();
        exe
    }

    #[test]
    fn standalone_updates_refuse_legacy_owned_files() {
        let tmp = tempfile::tempdir().unwrap();
        let ordinary = tmp.path().join("standalone/agentx");
        let brew = tmp.path().join("Cellar/agentx/0.7.29/bin/agentx");
        let venv = tmp.path().join("custom venv/Scripts/agentx.exe");
        let uv = tmp.path().join("custom bin/agentx.exe");
        let pip = pip_fixture(&tmp.path().join("Python313"), "Scripts", "site-packages", "../Scripts/agentx.exe,sha256=test,6\n");
        for file in [&ordinary, &brew, &venv, &uv] {
            fs::create_dir_all(file.parent().unwrap()).unwrap();
            fs::write(file, b"existing binary").unwrap();
        }
        fs::write(tmp.path().join("custom venv/pyvenv.cfg"), "home = unknown").unwrap();
        let tools = tmp.path().join("custom tools");
        fs::create_dir_all(tools.join("agentx")).unwrap();
        fs::write(tools.join("agentx/uv-receipt.toml"), format!("[tool]\nentrypoints = [{{ from = 'agentx', install-path = '{}' }}]\n", uv.display())).unwrap();
        let uv_dirs = [tools];
        for file in [&brew, &venv, &uv, &pip] {
            let before = fs::read(file).unwrap();
            let error = ensure_standalone_install_with_uv_dirs(file, &uv_dirs).unwrap_err();
            assert!(error.to_string().contains("github.com/aigotowork/agentx/releases/latest"), "{error:#}");
            assert_eq!(fs::read(file).unwrap(), before);
        }
        ensure_standalone_install_with_uv_dirs(&ordinary, &uv_dirs).unwrap();
        assert_eq!(record_path("\"../bin/with,comma\",hash,6"), Some("../bin/with,comma".into()));
    }
}
