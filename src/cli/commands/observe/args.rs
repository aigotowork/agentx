//! Family grammar and explicit local operand decoding; no daemon calls.

use std::fs::OpenOptions;
use std::io::Read;
use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand};
use serde::{Serialize, de::DeserializeOwned};

use crate::observability::configuration::{CompareConfig, ConfigFieldId, ConfigFieldSelection, ConfigSnapshotRef, ContextFieldId, ParticipantFieldId, validate_selection};
use crate::observability::context::ScopeRef;
use crate::observability::events::{MAX_PAGE_RECORDS, ObservationWindow};
use crate::observability::runtime::{ClientInvocationId, RequestObservationRef};

pub(super) const MAX_OPERAND_BYTES: usize = 32 * 1024;

#[derive(Debug, Parser)]
#[command(name = "agentx observe", about = "Read existing AgentX diagnostic evidence", after_help = "No subcommand means inspect. Files contain exact native JSON refs/windows (at most 32 KiB).\nUsage and recovery: .docs/observability/cli.md")]
pub(super) struct ObserveArgs {
    /// Existing data scope; default: originating AGENTX_DIR / home resolver
    #[arg(long, global = true, value_name = "PATH")]
    pub scope: Option<PathBuf>,
    /// Emit one JSON value; events --follow emits newline-delimited values
    #[arg(long, global = true)]
    pub json: bool,
    #[command(subcommand)]
    pub command: Option<ObserveCommand>,
}

#[derive(Debug, Subcommand)]
pub(super) enum ObserveCommand {
    /// Read the existing daemon's runtime/process snapshot
    Inspect,
    /// List retained hook traces, or join one trace with daemon request timing
    Trace {
        /// Invocation UUID from the trace list or a diagnostic log
        id: Option<uuid::Uuid>,
        /// Read the log without contacting any daemon
        #[arg(long)]
        local: bool,
        /// Maximum retained entries in each source, 1..32
        #[arg(long, default_value = "16", value_parser = clap::value_parser!(u16).range(1..=32))]
        limit: u16,
    },
    /// Read one exact native RequestObservationRef from a local JSON file
    Request {
        #[arg(long)]
        ref_file: PathBuf,
    },
    /// Read one exact retained ConfigSnapshotRef from a local JSON file
    Config {
        #[arg(long)]
        ref_file: PathBuf,
    },
    /// Explicitly reevaluate this invocation's scoped settings locally
    ConfigCurrent,
    /// Compare two retained configuration refs; never reread current settings
    Compare {
        #[arg(long)]
        left_file: PathBuf,
        #[arg(long)]
        right_file: PathBuf,
        /// Native setting name, context.FIELD or participant.FIELD; repeatable
        #[arg(long)]
        field: Vec<String>,
    },
    /// Read a bounded native event window; follow stops on gaps/era changes
    Events {
        #[arg(long)]
        window_file: Option<PathBuf>,
        /// Records per page, 1..256
        #[arg(long, default_value_t = 128)]
        limit: u16,
        #[arg(long)]
        follow: bool,
    },
}

#[derive(Clone, Debug)]
pub(super) enum PreparedCommand {
    Inspect,
    Trace { id: Option<ClientInvocationId>, local: bool, limit: u16 },
    Request(RequestObservationRef),
    Config(ConfigSnapshotRef),
    ConfigCurrent,
    Compare(CompareConfig),
    Events { window: Option<ObservationWindow>, limit: u16, follow: bool },
}

impl ObserveArgs {
    pub fn prepare(&self) -> Result<PreparedCommand, String> {
        if self.scope.as_ref().is_some_and(|path| path.as_os_str().is_empty()) {
            return Err("--scope requires a nonempty existing directory path".into());
        }
        match &self.command {
            None | Some(ObserveCommand::Inspect) => Ok(PreparedCommand::Inspect),
            Some(ObserveCommand::Trace { id, local, limit }) => {
                let id = id.map(ClientInvocationId);
                if id.is_some_and(|id| !id.validate()) {
                    return Err("trace ID must be a version 4 UUID".into());
                }
                Ok(PreparedCommand::Trace { id, local: *local, limit: *limit })
            }
            Some(ObserveCommand::Request { ref_file }) => {
                let reference: RequestObservationRef = read_operand(ref_file)?;
                if reference.id.0 == 0 {
                    return Err("request reference id must be positive".into());
                }
                Ok(PreparedCommand::Request(reference))
            }
            Some(ObserveCommand::Config { ref_file }) => Ok(PreparedCommand::Config(config_ref(ref_file)?)),
            Some(ObserveCommand::ConfigCurrent) => Ok(PreparedCommand::ConfigCurrent),
            Some(ObserveCommand::Compare { left_file, right_file, field }) => {
                let fields = if field.is_empty() { ConfigFieldSelection::All } else { ConfigFieldSelection::Only(field.iter().map(|name| field_selector(name)).collect::<Result<Vec<_>, _>>()?) };
                validate_selection(&fields).map_err(|_| "invalid or duplicate --field selection".to_owned())?;
                Ok(PreparedCommand::Compare(CompareConfig { left: config_ref(left_file)?, right: config_ref(right_file)?, fields }))
            }
            Some(ObserveCommand::Events { window_file, limit, follow }) => {
                if *limit == 0 || *limit > MAX_PAGE_RECORDS {
                    return Err(format!("--limit must be 1..={MAX_PAGE_RECORDS}"));
                }
                let window = window_file.as_ref().map(|path| read_operand::<ObservationWindow>(path)).transpose()?;
                if let Some(window) = &window {
                    // Era and numeric-window validation stays with the journal,
                    // which deliberately checks changed eras before frontiers.
                    window.filter.validate().map_err(|_| "invalid native event window filter".to_owned())?;
                }
                Ok(PreparedCommand::Events { window, limit: *limit, follow: *follow })
            }
        }
    }
}

impl PreparedCommand {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Inspect => "inspect",
            Self::Trace { .. } => "trace",
            Self::Request(_) => "request",
            Self::Config(_) => "config",
            Self::ConfigCurrent => "config-current",
            Self::Compare(_) => "compare",
            Self::Events { .. } => "events",
        }
    }

    /// Scope is chosen once, independently of operand refs. A ref never selects
    /// another scope or authorizes a cross-scope query implicitly.
    pub fn validate_scope(&self, scope: &ScopeRef) -> Result<(), String> {
        let matches = match self {
            Self::Request(reference) => &reference.daemon.scope == scope,
            Self::Config(reference) => &reference.epoch.daemon.scope == scope,
            Self::Compare(input) => &input.left.epoch.daemon.scope == scope && &input.right.epoch.daemon.scope == scope,
            Self::Events { window: Some(window), .. } => {
                &window.after.stream.epoch.daemon.scope == scope
                    && window.filter.configuration.as_ref().is_none_or(|reference| &reference.epoch.daemon.scope == scope)
                    && window.filter.domain_event.as_ref().is_none_or(|reference| &reference.scope == scope)
            }
            _ => true,
        };
        if matches { Ok(()) } else { Err("native operand belongs to a different scope; select its exact scope explicitly".into()) }
    }
}

fn config_ref(path: &Path) -> Result<ConfigSnapshotRef, String> {
    let reference: ConfigSnapshotRef = read_operand(path)?;
    if reference.sequence == 0 {
        return Err("configuration reference sequence must be positive".into());
    }
    Ok(reference)
}

/// The native setting catalog and selection validator remain authoritative.
/// Prefixes name the other two closed native field enums for CLI callers.
pub(super) fn field_selector(name: &str) -> Result<ConfigFieldId, String> {
    use ContextFieldId as C;
    use ParticipantFieldId as P;
    let value = match name {
        "context.cwd" => ConfigFieldId::Context(C::Cwd),
        "context.requested_scope" => ConfigFieldId::Context(C::RequestedScope),
        "context.canonical_scope" => ConfigFieldId::Context(C::CanonicalScope),
        "context.stdin_is_tty" => ConfigFieldId::Context(C::StdinIsTty),
        "context.stdout_is_tty" => ConfigFieldId::Context(C::StdoutIsTty),
        "context.explicit_name_selection" => ConfigFieldId::Context(C::ExplicitNameSelection),
        "context.provider_home" => ConfigFieldId::Context(C::ProviderHome),
        "context.tool" => ConfigFieldId::Context(C::Tool),
        "context.launch_regime" => ConfigFieldId::Context(C::LaunchRegime),
        "context.launch_backend" => ConfigFieldId::Context(C::LaunchBackend),
        "context.effective_working_directory" => ConfigFieldId::Context(C::EffectiveWorkingDirectory),
        "context.effective_dev_root" => ConfigFieldId::Context(C::EffectiveDevRoot),
        "participant.timeout" => ConfigFieldId::Participant(P::Timeout),
        "participant.subagent_timeout" => ConfigFieldId::Participant(P::SubagentTimeout),
        "participant.hints" => ConfigFieldId::Participant(P::Hints),
        _ => ConfigFieldId::setting(name).map_err(|_| "unknown --field; use a native setting, context.FIELD or participant.FIELD".to_owned())?,
    };
    Ok(value)
}

pub(super) fn read_operand<T: DeserializeOwned + Serialize>(path: &Path) -> Result<T, String> {
    // Reject devices/directories/FIFOs before opening an explicit local input.
    let metadata = std::fs::metadata(path).map_err(|_| format!("cannot read native operand file {}", path.display()))?;
    if !metadata.is_file() {
        return Err(format!("native operand must be a regular file: {}", path.display()));
    }
    if metadata.len() > MAX_OPERAND_BYTES as u64 {
        return Err("native operand exceeds 32 KiB".into());
    }
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        // A file replaced with a FIFO between metadata/open must not wait for
        // an input peer. Opened non-regular objects are rejected below.
        options.custom_flags(nix::libc::O_NONBLOCK);
    }
    let file = options.open(path).map_err(|_| format!("cannot open native operand file {}", path.display()))?;
    if !file.metadata().map_err(|_| "cannot inspect native operand file".to_owned())?.is_file() {
        return Err("native operand must be a regular file".into());
    }
    let mut bytes = Vec::new();
    file.take(MAX_OPERAND_BYTES as u64 + 1).read_to_end(&mut bytes).map_err(|_| "cannot read native operand file".to_owned())?;
    if bytes.len() > MAX_OPERAND_BYTES {
        return Err("native operand exceeds 32 KiB".into());
    }
    decode_operand(&bytes)
}

pub(super) fn decode_operand<T: DeserializeOwned + Serialize>(bytes: &[u8]) -> Result<T, String> {
    if bytes.len() > MAX_OPERAND_BYTES {
        return Err("native operand exceeds 32 KiB".into());
    }
    let text = std::str::from_utf8(bytes).map_err(|_| "native operand must be UTF-8 JSON".to_owned())?;
    // Deserialize the original bytes first, retaining serde's duplicate-field
    // and native reference validation, instead of normalizing through Value.
    let native: T = serde_json::from_str(text).map_err(|error| format!("invalid native JSON/reference at line {}, column {}", error.line(), error.column()))?;
    let supplied: serde_json::Value = serde_json::from_str(text).map_err(|_| "invalid native JSON".to_owned())?;
    let canonical = serde_json::to_value(&native).map_err(|_| "native operand cannot be represented".to_owned())?;
    if !known_keys(&supplied, &canonical) {
        return Err("unknown field in native JSON operand".into());
    }
    Ok(native)
}

/// Detect ignored unknown struct keys from the native serialization shape;
/// omitted optional fields stay valid and no public schema is duplicated.
fn known_keys(supplied: &serde_json::Value, canonical: &serde_json::Value) -> bool {
    match (supplied, canonical) {
        (serde_json::Value::Object(left), serde_json::Value::Object(right)) => left.iter().all(|(key, value)| right.get(key).is_some_and(|known| known_keys(value, known))),
        (serde_json::Value::Array(left), serde_json::Value::Array(right)) => left.len() == right.len() && left.iter().zip(right).all(|(value, known)| known_keys(value, known)),
        _ => true,
    }
}
