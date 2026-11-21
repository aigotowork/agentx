//! Read-only Info projections. Exact local subjects arrive from the native Agent snapshot.
use crate::{
    agents::{
        launch_title,
        workspaces::{self, LaunchCollaboration},
    },
    db::InstanceRow,
    management::contract::LaunchRelations,
    runtime::launch_profile::LaunchProvenance,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct Reference {
    pub scope: PathBuf,
    pub generation: String,
    pub target: Target,
    /// In-process snapshot adapter metadata; never sent over native IPC.
    #[serde(skip)]
    pub legacy: Option<Legacy>,
}

#[derive(Clone, Debug)]
pub(crate) enum Legacy {
    Exact,
    StopSnapshot { stopped_at: f64 },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) enum Target {
    Incarnation { name: String, bits: u64 },
    Stopped { name: String, event: i64, bits: Option<u64> },
}

impl Target {
    pub(crate) fn bits(&self) -> Option<u64> {
        match self {
            Self::Incarnation { bits, .. } => Some(*bits),
            Self::Stopped { bits, .. } => *bits,
        }
    }

    pub(crate) fn name(&self) -> &str {
        match self {
            Self::Incarnation { name, .. } | Self::Stopped { name, .. } => name,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) enum Section {
    Facts,
    Bootstrap,
    Instructions,
    ProfileArguments,
    Relations { after: Option<String>, limit: u32 },
    Bindings,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) enum Projection {
    Facts(Box<Facts>),
    Text(Option<String>),
    ProfileArguments(Option<LaunchProvenance>),
    Relations { total_children: u64, page: LaunchRelations },
    Bindings(Vec<crate::management::contract::FileBindingView>),
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub(crate) struct Facts {
    pub title: Option<String>,
    pub tool: Option<String>,
    pub created_at: Option<f64>,
    pub status: Option<String>,
    pub role: Option<Role>,
    pub profile: Option<Profile>,
    pub directory: Option<String>,
    pub workspace: Option<String>,
    pub terminal_preset: Option<String>,
    pub headless: Option<bool>,
    pub pid: Option<u32>,
    pub session_id: Option<String>,
    pub transcript_path: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct Role {
    pub name: String,
    pub description: Option<String>,
    pub namespace: String,
    pub path: PathBuf,
    pub profile: String,
    pub default: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct Profile {
    pub name: String,
    pub description: Option<String>,
    pub tool: String,
    pub selected_by: String,
}

/// Facts retained by native launch storage, independent of the read transport.
pub(crate) enum Saved {
    Current(Box<InstanceRow>),
    Stopped(Value),
}

pub(crate) fn snapshot_creation(snapshot: &Value) -> Option<f64> {
    let value = snapshot.get("created_at")?;
    value.as_f64().or_else(|| value.as_str()?.parse().ok()).filter(|time| f64::is_finite(*time))
}

fn nonempty(value: &str) -> Option<String> {
    (!value.is_empty()).then(|| value.to_owned())
}

impl Saved {
    fn text(&self, key: &str) -> Option<&str> {
        let Self::Stopped(snapshot) = self else { return None };
        snapshot.get(key).and_then(Value::as_str).filter(|value| !value.is_empty())
    }

    pub(crate) fn directory(&self) -> Option<&str> {
        match self {
            Self::Current(row) => Some(row.directory.as_str()).filter(|path| !path.is_empty()),
            Self::Stopped(_) => self.text("directory"),
        }
    }

    pub(crate) fn collaboration(&self) -> Option<LaunchCollaboration> {
        let value;
        let checked = match self {
            Self::Current(row) => return workspaces::collaboration(row),
            Self::Stopped(snapshot) => snapshot.get("launch_context")?,
        };
        let checked = if let Some(raw) = checked.as_str() {
            value = serde_json::from_str(raw).ok()?;
            &value
        } else {
            checked
        };
        serde_json::from_value(checked.get("collaboration")?.clone()).ok()
    }

    pub(crate) fn profile(&self) -> Option<LaunchProvenance> {
        let raw = match self {
            Self::Current(row) => row.launch_profile.as_deref()?,
            Self::Stopped(_) => self.text("launch_profile")?,
        };
        serde_json::from_str(raw).ok()
    }

    pub(crate) fn facts(&self, current_status: Option<String>) -> Facts {
        let mut facts = match self {
            Self::Current(row) => Facts {
                title: launch_title::from_context(row.launch_context.as_deref()),
                tool: nonempty(&row.tool),
                status: current_status,
                created_at: Some(row.created_at),
                terminal_preset: row.terminal_preset_effective.clone().or_else(|| row.terminal_preset_requested.clone()),
                headless: Some(row.background != 0),
                pid: row.pid.and_then(|pid| u32::try_from(pid).ok()).filter(|pid| *pid > 0),
                session_id: row.session_id.clone(),
                transcript_path: nonempty(&row.transcript_path),
                ..Facts::default()
            },
            Self::Stopped(snapshot) => Facts {
                title: launch_title::from_snapshot(snapshot),
                tool: self.text("tool").map(str::to_owned),
                status: Some("stopped".into()),
                created_at: snapshot_creation(snapshot),
                terminal_preset: self.text("terminal_preset_effective").or_else(|| self.text("terminal_preset_requested")).map(str::to_owned),
                headless: snapshot.get("background").and_then(|value| value.as_bool().or_else(|| value.as_i64().map(|number| number != 0))),
                session_id: self.text("session_id").map(str::to_owned),
                transcript_path: self.text("transcript_path").map(str::to_owned),
                ..Facts::default()
            },
        };
        facts.directory = self.directory().map(str::to_owned);
        if let Some(adopted) = self.collaboration() {
            facts.workspace = Some(adopted.workspace.unwrap_or_else(|| "global".into()));
            facts.role = adopted.role.map(|role| Role { name: role.name, description: role.description, namespace: role.namespace, path: role.path, profile: role.profile, default: role.default });
        }
        facts.profile = self.profile().map(|profile| Profile { name: profile.name, description: profile.definition.description, tool: profile.definition.tool, selected_by: profile.selected_by });
        facts
    }
}
